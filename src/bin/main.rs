use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use blaze_service::prelude::*;
use blaze_service::server::schema::{UserData, UserStats};
use blaze_service::server::service::{
    get_all_free_users, get_all_pro_users, get_all_starter_users, get_unverified_users,
    is_user_exists, is_user_verified, save_user, verify_user,
};
use blaze_service::{error, info, warn};
use std::sync::OnceLock;

static SERVER_START_TIME: OnceLock<chrono::DateTime<chrono::Local>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting Blaze Service...");

    dotenv::dotenv().ok();

    let port = std::env::var("PORT").expect("PORT must be set 😠");
    // Create necessary directories
    create_dirs().await?;

    // Create the router
    let app = create_router().await;

    let addr = format!("0.0.0.0:{}", port);
    start_cleanup_task().await;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let server_time = chrono::Local::now();

    // Initialize server start time
    SERVER_START_TIME.get_or_init(|| server_time);

    info!("Server listening on {}", addr);
    info!("Server started at {}", server_time.to_rfc3339().yellow());
    axum::serve(listener, app).await?;
    Ok(())
}

async fn create_router() -> Router {
    Router::new()
        .route("/v1/blz/health", get(health_check))
        .route("/v1/blz/auth/register", post(auth_register))
        .route("/v1/blz/auth/verify-email", post(auth_verify_email))
        .route("/v1/blz/auth/verify-code", post(auth_verify_code))
        .route("/billing/plans", get(billing_plans))
        .route("/v1/blz/users/stats", get(get_user_stats))
    // .route("/billing/checkout", post(billing_checkout))
    // .route("/billing/webhook", post(stripe_webhook))
    // .route("/account/status", get(account_status))
}

// Start background cleanup task
pub async fn start_cleanup_task() {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            match cleanup_expired_otps().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Cleaned up {} expired OTP(s)", count);
                    }
                }
                Err(e) => error!("OTP cleanup failed: {}", e),
            }
        }
    });
}

async fn health_check() -> impl IntoResponse {
    let uptime_hours = if let Some(start_time) = SERVER_START_TIME.get() {
        let now = chrono::Local::now();
        let duration = now.signed_duration_since(*start_time);
        duration.num_milliseconds() as f64 / 3600000.0 // Convert milliseconds to hours
    } else {
        0.0
    };

    let response = serde_json::json!({
        "status": "healthy",
        "uptime_hours": format!("{:.2}", uptime_hours)
    });

    info!("Health check: Uptime: {:.2} hours", uptime_hours);

    (StatusCode::OK, Json(response))
}

/// This endpoint handles user registration and saves the user data.
async fn auth_register(Json(payload): Json<UserRegisterRequest>) -> impl IntoResponse {
    info!("User registration attempt for email: {}", payload.email);
    if is_empty_field(&payload.username) || is_empty_field(&payload.email) {
        warn!("Registration failed: Empty username or email");
        return (
            StatusCode::BAD_REQUEST,
            Json(UserRegisterResponse {
                email: "".to_string(),
                is_created: false,
            }),
        );
    }

    match is_user_exists(&payload.email).await {
        Ok(exists) => {
            if exists {
                warn!("User already exists with email: {}", payload.email);
                return (
                    StatusCode::CONFLICT,
                    Json(UserRegisterResponse {
                        email: "".to_string(),
                        is_created: false,
                    }),
                );
            }
        }
        Err(e) => {
            error!(
                "Some error occurred while checking user existence for email: {}, Error: {:?}",
                payload.email, e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UserRegisterResponse {
                    email: "".to_string(),
                    is_created: false,
                }),
            );
        }
    }

    match save_user(&payload).await {
        Ok(response) => {
            info!(
                "User registered successfully with email: {}",
                response.email
            );
            (StatusCode::CREATED, Json(response))
        }
        Err(e) => {
            error!(
                "User registration failed for email: {}, Error: {:?}",
                payload.email, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UserRegisterResponse {
                    email: "".to_string(),
                    is_created: false,
                }),
            )
        }
    }
}

/// This endpoint handles email verification requests which sends a verification code to the user's email.
async fn auth_verify_email(Json(payload): Json<VerifyEmailRequest>) -> impl IntoResponse {
    info!("Verify email attempt for email: {}", payload.email);

    if is_empty_field(&payload.email) {
        warn!("Email verification failed: Empty email");
        return (
            StatusCode::BAD_REQUEST,
            Json(VerifyEmailResponse {
                is_code_sent: false,
            }),
        );
    }

    // Check user exists
    match is_user_exists(&payload.email).await {
        Ok(exists) => {
            if !exists {
                warn!(
                    "Email verification failed: User not found for email: {}",
                    payload.email
                );
                return (
                    StatusCode::NOT_FOUND,
                    Json(VerifyEmailResponse {
                        is_code_sent: false,
                    }),
                );
            }
        }
        Err(e) => {
            error!(
                "Some error occurred while checking user existence for email: {}, Error: {:?}",
                payload.email, e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyEmailResponse {
                    is_code_sent: false,
                }),
            );
        }
    }

    // Check if already verified
    match is_user_verified(&payload.email).await {
        Ok(is_verified) => {
            if is_verified {
                info!("User already verified for email: {}", payload.email);
                return (
                    StatusCode::CONFLICT,
                    Json(VerifyEmailResponse {
                        is_code_sent: false,
                    }),
                );
            }
        }
        Err(e) => {
            error!(
                "Some error occurred while checking user verification for email: {}, Error: {:?}",
                payload.email, e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyEmailResponse {
                    is_code_sent: false,
                }),
            );
        }
    }

    match verify_user(&payload).await {
        Ok(response) => {
            info!("Verification code sent to email: {}", payload.email);
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!(
                "Email verification failed for email: {}, Error: {:?}",
                payload.email, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyEmailResponse {
                    is_code_sent: false,
                }),
            )
        }
    }
}

// TODO: Explicitly handle cases like user not found, OTP expired, invalid OTP, etc, right now its either 200 or 500.
/// This endpoint handles verification code submission for email verification.
async fn auth_verify_code(Json(payload): Json<VerifyOtpRequest>) -> impl IntoResponse {
    info!("OTP verification attempt for email: {}", payload.email);
    if is_empty_field(&payload.email) || is_empty_field(&payload.otp) {
        warn!("OTP verification failed: Empty email or OTP");
        return (
            StatusCode::BAD_REQUEST,
            Json(VerifyOtpResponse {
                is_verified: false,
                message: "Email or OTP cannot be empty".to_string(),
                api_key: None,
            }),
        );
    }
    match verify_otp_service(&payload).await {
        Ok(response) => {
            info!("OTP verified for email: {}", payload.email);
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!(
                "OTP verification failed for email: {}, Error: {:?}",
                payload.email, e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyOtpResponse {
                    is_verified: false,
                    message: "Something went wrong, Error: ".to_string() + &e.to_string(),
                    api_key: None,
                }),
            )
        }
    }
}

async fn billing_plans() -> impl IntoResponse {
    let plans = vec![Plans::free_plan(), Plans::starter_plan(), Plans::pro_plan()];
    (StatusCode::OK, Json(plans))
}

async fn get_user_stats() -> impl IntoResponse {
    let unverified_user = get_unverified_users().await.unwrap_or_else(|e| {
        error!("Failed to fetch unverified users: {:?}", e);
        Vec::new()
    });
    let free_users = get_all_free_users().await.unwrap_or_else(|e| {
        error!("Failed to fetch free users: {:?}", e);
        Vec::new()
    });
    let starter_users = get_all_starter_users().await.unwrap_or_else(|e| {
        error!("Failed to fetch starter users: {:?}", e);
        Vec::new()
    });
    let pro_users = get_all_pro_users().await.unwrap_or_else(|e| {
        error!("Failed to fetch pro users: {:?}", e);
        Vec::new()
    });

    let userdata = UserData {
        unverified_users: unverified_user.into_iter().map(UserStats::from).collect(),
        free_users: free_users.into_iter().map(UserStats::from).collect(),
        stater_users: starter_users.into_iter().map(UserStats::from).collect(),
        pro_users: pro_users.into_iter().map(UserStats::from).collect(),
    };
    (StatusCode::OK, Json(userdata))
}

fn is_empty_field(field: &str) -> bool {
    if field.trim().is_empty() { true } else { false }
}
