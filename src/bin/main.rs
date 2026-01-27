use anyhow::Result;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use blaze_service::info;
use blaze_service::prelude::UserLoginRequest;

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting Blaze Service...");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // Create necessary directories
    tokio::fs::create_dir_all("data/users").await?;
    tokio::fs::create_dir_all("data/billing").await?;

    let server_time = chrono::Local::now();
    // Sanitize server_time for directory name
    let _time_string = server_time.to_rfc3339();

    tokio::fs::create_dir_all(format!("logs/{}", server_time.format("%Y-%m-%d"))).await?;

    let addr = format!("0.0.0.0:{}", port);
    let app = create_router().await;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    info!("Server listening on {}", addr);
    info!("Server started at {}", server_time.to_rfc3339().yellow());

    Ok(())
}

async fn create_router() -> Router {
    Router::new().route("/auth/login", post(auth_login))
    //         .route("/auth/verify", post(auth_verify))
    //         .route("/billing/plans", get(billing_plans))
    //         .route("/billing/checkout", post(billing_checkout))
    //         .route("/billing/webhook", post(stripe_webhook))
    //         .route("/account/status", get(account_status))
    //
}

async fn auth_login(Json(_payload): Json<UserLoginRequest>) -> impl IntoResponse {}
