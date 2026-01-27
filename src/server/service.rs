use crate::prelude::{
    Plans, User, UserRegisterRequest, UserRegisterResponse, VerifyEmailRequest, VerifyEmailResponse,
};
use crate::server::storage::DataStore;
use crate::{error, info};
use anyhow::Result;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::path::PathBuf;

/// Creates necessary directories for the service: data, logs, and billing.
pub async fn create_dirs() -> Result<()> {
    let data_path = get_data_path().await;
    let logs_path = get_logs_path().await;
    let billing_path = get_billing_path().await;

    tokio::fs::create_dir_all(&data_path).await?;
    tokio::fs::create_dir_all(&logs_path).await?;
    tokio::fs::create_dir_all(&billing_path).await?;
    Ok(())
}

/// Creates a daily log directory based on the current date.
pub async fn create_logs_dir() -> Result<PathBuf> {
    let logs_path = get_logs_path().await;
    let server_time = chrono::Local::now();
    let daily_log_path = logs_path.join(server_time.format("%Y-%m-%d").to_string());
    tokio::fs::create_dir_all(&daily_log_path).await?;
    Ok(daily_log_path)
}

pub async fn get_data_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("data")
}

pub async fn get_logs_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("logs")
}

pub async fn get_billing_path() -> PathBuf {
    let data_path = get_data_path().await;
    data_path.join("blz_service").join("billing")
}

/// Saves a new user to the datastore and returns a response indicating success.
pub async fn save_user(user_data: UserRegisterRequest) -> Result<UserRegisterResponse> {
    let datastore = DataStore::<String, User>::new(get_data_path().await.join("users.json"))?;

    let id = uuid::Uuid::new_v4().to_string();

    // Created a user
    let user = User {
        user_id: id.clone(),
        username: user_data.username,
        email: user_data.email,
        api_key: None,
        is_verified: false,
        plans: Plans::free_plan(),
        instance_url: "".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    datastore.insert(id.clone(), user)?;

    let response = UserRegisterResponse {
        user_id: id.clone(),
        is_created: true,
    };

    Ok(response)
}

/// Checks if a user with the given email exists in the datastore.
pub async fn is_user_exists(email: &String) -> Result<bool> {
    let datastore = DataStore::<String, User>::new(get_data_path().await.join("users.json"))?;
    if let Some(_user) = datastore.get(email)? {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub async fn send_verification_code(data: VerifyEmailRequest) -> Result<VerifyEmailResponse> {
    // Generate a random 6-digit OTP
    let otp: String = (0..6)
        .map(|_| rand::random::<u8>() % 10)
        .map(|digit| char::from(b'0' + digit))
        .collect();

    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{ font-family: monospace; background: #000; color: #0f0; }}
                .otp {{ font-size: 32px; letter-spacing: 5px; padding: 20px; background: #111; }}
            </style>
        </head>
        <body>
            <h1>BlazeDB Login</h1>
            <p>Your one-time password:</p>
            <div class="otp">{}</div>
            <p>This code expires in 5 minutes.</p>
            <p>If you didn't request this, ignore this email.</p>
        </body>
        </html>
        "#,
        otp
    );

    let plain_body = format!("Your BlazeDB OTP: {}\n\nExpires in 5 minutes.", otp);

    // Get app_passwords from env
    let app_password = std::env::var("APP_PASSWORDS")?;

    let email = Message::builder()
        .from("noreply.blz.service@gmail.com".parse()?)
        .to(data.email.parse()?)
        .subject("Email Verification Code")
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(plain_body))
                .singlepart(SinglePart::html(html_body)),
        )?;

    let creds = Credentials::new("noreply.blz.service@gmail.com".parse()?, app_password);

    let mailer = SmtpTransport::relay("smtp.gmail.com")?
        .credentials(creds)
        .build();

    let response: VerifyEmailResponse = match mailer.send(&email) {
        Ok(_) => {
            info!("Verification email sent successfully!");

            VerifyEmailResponse { is_code_sent: true }
        }
        Err(e) => {
            error!("Could not send email: {:?}", e);
            VerifyEmailResponse {
                is_code_sent: false,
            }
        }
    };

    Ok(response)
}
