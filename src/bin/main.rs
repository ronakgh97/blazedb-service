use anyhow::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use blaze_service::info;
use blaze_service::prelude::{
    UserRegisterRequest, UserRegisterResponse, VerifyEmailRequest, create_dirs,
};
use blaze_service::server::service::{is_user_exists, save_user};

#[tokio::main]
async fn main() -> Result<()> {
    info!("Starting Blaze Service...");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    // Create necessary directories
    create_dirs().await?;

    let addr = format!("0.0.0.0:{}", port);
    let app = create_router().await;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let server_time = chrono::Local::now();
    axum::serve(listener, app).await?;
    info!("Server listening on {}", addr);
    info!("Server started at {}", server_time.to_rfc3339().yellow());

    Ok(())
}

async fn create_router() -> Router {
    Router::new()
        .route("/v1/blz/auth/register", post(auth_register))
        .route("v1/blz/auth/verify", post(auth_verify))
    //         .route("/billing/plans", get(billing_plans))
    //         .route("/billing/checkout", post(billing_checkout))
    //         .route("/billing/webhook", post(stripe_webhook))
    //         .route("/account/status", get(account_status))
    //
}

async fn auth_register(Json(payload): Json<UserRegisterRequest>) -> impl IntoResponse {
    if is_empty_field(&payload.username) || is_empty_field(&payload.email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(UserRegisterResponse {
                user_id: "null".to_string(),
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
                        user_id: "null".to_string(),
                        is_created: false,
                    }),
                );
            }
        }
        Err(_e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UserRegisterResponse {
                    user_id: "null".to_string(),
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
                user_id: "null".to_string(),
                is_created: false,
            }),
        ),
    }
}

async fn auth_verify(Json(_payload): Json<VerifyEmailRequest>) -> impl IntoResponse {
    (StatusCode::OK, "Verify Endpoint")
}

fn is_empty_field(field: &str) -> bool {
    if field.trim().is_empty() { true } else { false }
}
