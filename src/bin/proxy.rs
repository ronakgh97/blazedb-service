use anyhow::Result;
use axum::routing::get;
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
};
use blaze_service::server::crypto::{extract_email_from_api_key, hash_api_key};
use blaze_service::server::ports::calculate_container_port;
use blaze_service::server::schema::User;
use blaze_service::server::service::get_data_path;
use blaze_service::server::storage::DataStore;
use blaze_service::{error, info};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    // LRU Cache: api_key_hash -> User (auto-eviction when full)
    user_cache: Arc<RwLock<LruCache<String, CachedUser>>>,
    user_store: DataStore<String, User>, // In-memory user store (loaded from disk)
    client: reqwest::Client,
    start_time: Instant,
}

#[derive(Clone, Debug)]
struct CachedUser {
    email: String,
    username: String,
    instance_id: String,
    // TODO: Quota and rate limit enforcement remaining
    #[allow(unused)]
    is_verified: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting Blaze Proxy Server...");

    dotenv::dotenv().ok();

    let user_store = DataStore::<String, User>::new(get_data_path().join("users.json"))?;

    // LRU Cache with automatic eviction + background reload strategy
    // - Max 1024 entries (oldest evicted when full)
    // - Background task reloads user_store every 60s
    // - Cache invalidation happens naturally on next access after reload
    let state = AppState {
        user_store,
        user_cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(1024).unwrap()))),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
        start_time: Instant::now(),
    };

    update_cache_task(state.clone()).await;

    let app = create_router(state);

    dotenv::dotenv().ok();

    let port = std::env::var("PROXY_PORT").unwrap_or("8000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let server_time = chrono::Utc::now();

    info!("Proxy server listening on {}", addr);
    info!("Server started at {}", server_time.to_rfc3339());
    info!("Ready to accept connections");

    axum::serve(listener, app).await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/blazedb/{*path}", any(proxy_handler))
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let uptime_secs = state.start_time.elapsed().as_secs();
    let uptime_hrs = uptime_secs as f64 / 3600.0;

    Json(serde_json::json!({
        "status": "ok",
        "service": "blaze-proxy",
        "uptime_hrs": format!("{:.2}", uptime_hrs),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn proxy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Result<Response, ProxyError> {
    let path = uri.path();

    // Block restricted endpoints
    if path.contains("/v1/blazedb/embed") || path.contains("/v1/blazedb/query") {
        error!("Blocked request to restricted endpoint: {}", path);
        return Err(ProxyError::BlockedEndpoint);
    }

    // Extract instance_id from URL
    let instance_id = path
        .trim_end_matches('/')
        .split('/')
        .last()
        .ok_or(ProxyError::InvalidPath)?
        .to_string();

    info!(
        "{} {} (Instance ID: {})",
        method.as_str(),
        path,
        &instance_id.chars().take(8).collect::<String>()
    );

    // Extract API key
    let api_key = extract_api_key(&headers)?;

    // Extract email from API key
    let email = extract_email_from_api_key(&api_key).ok_or(ProxyError::InvalidApiKey)?;

    info!(" ↳ User email: {}", email);

    // Verify API key and get user data (with cache)
    let api_key_hash = hash_api_key(&api_key).await;
    let user = verify_api_key(&state, &api_key_hash, &email).await?;

    info!(" ↳ User: {} ({})", user.username, user.email);

    // Verify instance_id matches user's instance_id
    if user.instance_id != instance_id {
        error!(
            "  ✗ Instance ID mismatch! User: {}, Requested: {}",
            user.instance_id, instance_id
        );
        return Err(ProxyError::Forbidden);
    }

    // Strip instance_id from path and build target URL
    // Example: /v1/blazedb/query/a1a70763... → /v1/blazedb/query
    let stripped_path = path.rsplitn(2, '/').nth(1).unwrap_or("/v1/blazedb");

    // Build target URL based on environment
    // INSIDE DOCKER: Use container DNS name (e.g., http://blazedb-a1a70763:8080) [prod]
    // OUTSIDE DOCKER: Use localhost with port mapping (e.g., http://localhost:PORT) [dev]
    let container_url = if std::env::var("PROXY_MODE").unwrap_or_default() == "external" {
        format!(
            "http://localhost:{}{}",
            calculate_container_port(&instance_id),
            stripped_path
        )
    } else {
        // Running INSIDE Docker - use internal DNS
        format!("http://blazedb-{}:8080{}", instance_id, stripped_path)
    };

    info!(" ↳ Forwarding to: {}", container_url);

    // Forward request
    let response = forward_request(&state.client, &container_url, method, headers, body).await?;

    info!("  ✓ Response: {}", response.status());

    Ok(response)
}

#[inline]
async fn forward_request(
    client: &reqwest::Client,
    target_url: &str,
    method: Method,
    mut headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ProxyError> {
    headers.remove("Authorization");
    headers.remove("authorization");

    let mut req_builder = match method {
        Method::GET => client.get(target_url),
        Method::POST => client.post(target_url),
        Method::PUT => client.put(target_url),
        Method::DELETE => client.delete(target_url),
        _ => return Err(ProxyError::UnsupportedMethod),
    };

    // Add remaining headers (Content-Type, Accept, etc.)
    req_builder = req_builder.headers(headers);

    if !body.is_empty() {
        req_builder = req_builder.body(body);
    }

    // Send request
    let response = req_builder.send().await.map_err(|e| {
        error!("  ✗ Failed to connect to BlazeDB: {}", e);
        ProxyError::InstanceUnavailable
    })?;

    // Convert reqwest::Response to axum::Response
    let status = response.status();
    let mut builder = Response::builder().status(status);

    // Copy response headers
    for (key, value) in response.headers().iter() {
        builder = builder.header(key, value);
    }

    // Get response body
    let body_bytes = response
        .bytes()
        .await
        .map_err(|_| ProxyError::InstanceError)?;

    builder
        .body(Body::from(body_bytes))
        .map_err(|_| ProxyError::InternalError)
}

fn extract_api_key(headers: &HeaderMap) -> Result<String, ProxyError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or(ProxyError::MissingApiKey)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ProxyError::InvalidApiKey)?;

    let api_key = if auth_str.starts_with("Bearer ") {
        auth_str
            .split_whitespace()
            .nth(1)
            .ok_or(ProxyError::InvalidApiKey)?
    } else {
        auth_str
    };

    if !api_key.starts_with("blz_") {
        return Err(ProxyError::InvalidApiKey);
    }

    Ok(api_key.to_string())
}

async fn verify_api_key(
    state: &AppState,
    api_key_hash: &str,
    email: &String,
) -> Result<CachedUser, ProxyError> {
    // Check LRU cache first
    {
        let mut cache = state.user_cache.write().await;
        if let Some(cached) = cache.get(api_key_hash) {
            info!("  ↳ Cache hit!");
            return Ok(cached.clone());
        }
    }

    // Cache miss - load from disk or memory and verify
    let cached_user = load_and_verify(&state.user_store, api_key_hash, email).await?;

    // Update LRU cache (auto-evicts oldest entry if full)
    {
        let mut cache = state.user_cache.write().await;
        cache.put(api_key_hash.to_string(), cached_user.clone());
    }

    Ok(cached_user)
}

// Load and verify user from DataStore (thread-safe with RwLock)
async fn load_and_verify(
    user_store: &DataStore<String, User>,
    api_key_hash: &str,
    email: &String,
) -> Result<CachedUser, ProxyError> {
    let user = user_store
        .get(email)
        .map_err(|_| ProxyError::DatastoreNotFound)?
        .ok_or(ProxyError::InvalidApiKey)?;

    // Verify API key hash matches
    let key_valid = user
        .api_key
        .iter()
        .any(|k| !k.is_revoked && k.api_key_hash == api_key_hash);

    if !key_valid {
        return Err(ProxyError::InvalidApiKey);
    }

    Ok(CachedUser {
        email: user.email.clone(),
        username: user.username.clone(),
        instance_id: user.instance_id.clone(),
        is_verified: user.is_verified,
    })
}

/// Background task to reload user store from disk periodically
/// This ensures cache stays fresh without clearing it (LRU will naturally evict stale entries)
async fn update_cache_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            // Reload user store from disk (cache will naturally refresh on next access)
            if let Err(e) = state.user_store.reload() {
                error!("Failed to reload user store: {}", e);
            }
        }
    });
}

#[derive(Debug)]
enum ProxyError {
    MissingApiKey,
    InvalidApiKey,
    InvalidPath,
    Forbidden,
    BlockedEndpoint,
    DatastoreNotFound,
    #[allow(unused)]
    DatastoreError,
    InstanceUnavailable,
    InstanceError,
    UnsupportedMethod,
    InternalError,
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ProxyError::MissingApiKey => (
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header with API key",
            ),
            ProxyError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            ProxyError::BlockedEndpoint => (
                StatusCode::UNAUTHORIZED,
                "This endpoint is not available",
            ),
            ProxyError::InvalidPath => (
                StatusCode::BAD_REQUEST,
                "Invalid request path - missing instance_id",
            ),
            ProxyError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Instance ID does not match your API key",
            ),
            ProxyError::DatastoreNotFound => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "User datastore not found",
            ),
            ProxyError::DatastoreError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read user datastore",
            ),
            ProxyError::InstanceUnavailable => {
                (StatusCode::BAD_GATEWAY, "BlazeDB instance is unavailable")
            }
            ProxyError::InstanceError => (
                StatusCode::BAD_GATEWAY,
                "Error communicating with BlazeDB instance",
            ),
            ProxyError::UnsupportedMethod => {
                (StatusCode::METHOD_NOT_ALLOWED, "HTTP method not supported")
            }
            ProxyError::InternalError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal proxy error")
            }
        };

        (
            status,
            Json(serde_json::json!({
                "error": message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
            .into_response()
    }
}
