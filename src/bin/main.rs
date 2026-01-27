use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use blaze_service::prelude::*;
use blaze_service::server::service::{is_user_exists, save_user, verify_user};
use blaze_service::{error, info};
use std::sync::OnceLock;

static SERVER_START_TIME: OnceLock<chrono::DateTime<chrono::Local>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting Blaze Service...");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

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
    //         .route("/billing/plans", get(billing_plans))
    //         .route("/billing/checkout", post(billing_checkout))
    //         .route("/billing/webhook", post(stripe_webhook))
    //         .route("/account/status", get(account_status))
    //
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

    (StatusCode::OK, Json(response))
}

/// This endpoint handles user registration and saves the user data.
async fn auth_register(Json(payload): Json<UserRegisterRequest>) -> impl IntoResponse {
    if is_empty_field(&payload.username) || is_empty_field(&payload.email) {
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
                return (
                    StatusCode::CONFLICT,
                    Json(UserRegisterResponse {
                        email: "".to_string(),
                        is_created: false,
                    }),
                );
            }
        }
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UserRegisterResponse {
                    email: "".to_string(),
                    is_created: false,
                }),
            );
        }
    }

    match save_user(payload).await {
        Ok(response) => (StatusCode::CREATED, Json(response)),
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(UserRegisterResponse {
                email: "".to_string(),
                is_created: false,
            }),
        ),
    }
}

/// This endpoint handles email verification requests which sends a verification code to the user's email.
async fn auth_verify_email(Json(payload): Json<VerifyEmailRequest>) -> impl IntoResponse {
    if is_empty_field(&payload.email) {
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
                return (
                    StatusCode::NOT_FOUND,
                    Json(VerifyEmailResponse {
                        is_code_sent: false,
                    }),
                );
            }
        }
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(VerifyEmailResponse {
                    is_code_sent: false,
                }),
            );
        }
    }

    match verify_user(payload).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyEmailResponse {
                is_code_sent: false,
            }),
        ),
    }
}

// TODO: Explicitly handle cases like user not found, OTP expired, invalid OTP, etc, right now its either 200 or 500.
/// This endpoint handles verification code submission for email verification.
async fn auth_verify_code(Json(payload): Json<VerifyOtpRequest>) -> impl IntoResponse {
    if is_empty_field(&payload.email) || is_empty_field(&payload.otp) {
        return (
            StatusCode::BAD_REQUEST,
            Json(VerifyOtpResponse {
                is_verified: false,
                message: "Email or OTP cannot be empty".to_string(),
            }),
        );
    }
    match verify_otp_service(payload).await {
        Ok(response) => (StatusCode::OK, Json(response)),
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyOtpResponse {
                is_verified: false,
                message: "Internal server error".to_string(),
            }),
        ),
    }
}

fn is_empty_field(field: &str) -> bool {
    if field.trim().is_empty() { true } else { false }
}
