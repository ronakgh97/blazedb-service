use anyhow::Result;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use blaze_service::prelude::UserLoginRequest;

#[tokio::main]
async fn main() -> Result<()> {
    let port = 3000;
    let addr = format!("0.0.0:{}", port);

    let app = create_router().await;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

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
