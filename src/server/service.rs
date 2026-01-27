pub use crate::prelude::{
    Plans, User, UserRegisterRequest, UserRegisterResponse, VerifyEmailRequest, VerifyEmailResponse,
};
use crate::server::crypto::{hash_otp, verify_otp as crypto_verify_otp};
pub use crate::server::schema::{OtpRecord, VerifyOtpRequest, VerifyOtpResponse};
use crate::server::storage::DataStore;
use crate::{error, info};
use anyhow::Result;
use chrono::{Duration, Utc};
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
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("billings")
}

/// Saves a new user to the datastore and returns a response indicating success.
pub async fn save_user(user_data: UserRegisterRequest) -> Result<UserRegisterResponse> {
    let datastore = DataStore::<String, User>::new(get_data_path().await.join("users.json"))?;

    // Create a user with email as the key
    let user = User {
        username: user_data.username,
        email: user_data.email.clone(),
        api_key: None,
        is_verified: false,
        plans: Plans::free_plan(),
        instance_url: "".to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    
    datastore.insert(user_data.email.clone(), user)?;

    let response = UserRegisterResponse {
        email: user_data.email,
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

/// Initiates the email verification process by sending a verification code to the user's email
pub async fn verify_user(data: VerifyEmailRequest) -> Result<VerifyEmailResponse> {
    // Send verification code and return response
    match send_verification_code(&data.email).await {
        Ok(is_sent) => Ok(VerifyEmailResponse {
            is_code_sent: is_sent,
        }),
        Err(e) => {
            error!("Error sending verification code: {:?}", e);
            Ok(VerifyEmailResponse {
                is_code_sent: false,
            })
        }
    }
}

// TODO: Decouple the checks for explicit error status code
/// Verifies the OTP code provided by the user and updates their verification status
pub async fn verify_otp(data: VerifyOtpRequest) -> Result<VerifyOtpResponse> {
    let otp_datastore =
        DataStore::<String, OtpRecord>::new(get_data_path().await.join("otps.json"))?;

    // Check if OTP record exists for this email
    let otp_record = match otp_datastore.get(&data.email)? {
        Some(record) => record,
        None => {
            return Ok(VerifyOtpResponse {
                is_verified: false,
                message: "No verification code found for this email".to_string(),
            });
        }
    };

    // Check if OTP has expired
    let now = Utc::now();
    let expires_at =
        chrono::DateTime::parse_from_rfc3339(&otp_record.expires_at)?.with_timezone(&Utc);

    if now > expires_at {
        // Clean up expired OTP
        otp_datastore.delete(&data.email)?;
        return Ok(VerifyOtpResponse {
            is_verified: false,
            message: "Verification code has expired".to_string(),
        });
    }

    // Verify the OTP
    let otp_hash_bytes = hex::decode(&otp_record.otp_hash)?;
    let is_valid = crypto_verify_otp(&data.otp, &otp_hash_bytes).await;

    if !is_valid {
        return Ok(VerifyOtpResponse {
            is_verified: false,
            message: "Invalid verification code".to_string(),
        });
    }

    // OTP is valid - update user verification status
    let user_datastore = DataStore::<String, User>::new(get_data_path().await.join("users.json"))?;

    // Get user by email (direct lookup since email is the key)
    let mut user = match user_datastore.get(&data.email)? {
        Some(u) => u,
        None => {
            return Ok(VerifyOtpResponse {
                is_verified: false,
                message: "User not found".to_string(),
            });
        }
    };

    // Update verification status
    user.is_verified = true;
    user_datastore.insert(data.email.clone(), user)?;

    // Clean up used OTP
    otp_datastore.delete(&data.email)?;

    info!("User {} successfully verified", data.email);

    Ok(VerifyOtpResponse {
        is_verified: true,
        message: "Email verified successfully".to_string(),
    })
}

/// Just Sends a verification code (OTP) to the specified email address and stores the hashed OTP in the datastore
pub async fn send_verification_code(email: &str) -> Result<bool> {
    // Generate a random 6-digit OTP
    let otp: String = (0..6)
        .map(|_| rand::random::<u8>() % 10)
        .map(|digit| char::from(b'0' + digit))
        .collect();

    // Hash the OTP before storing
    let otp_hash = hash_otp(&otp).await;
    let otp_hash_hex = hex::encode(&otp_hash);

    // Create OTP record with expiration (5 minutes from now)
    let now = Utc::now();
    let expires_at = now + Duration::minutes(5);

    let otp_record = OtpRecord {
        email: email.to_string(),
        otp_hash: otp_hash_hex,
        created_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    };

    // Store OTP in datastore
    let otp_datastore =
        DataStore::<String, OtpRecord>::new(get_data_path().await.join("otps.json"))?;
    otp_datastore.insert(email.to_string(), otp_record)?;

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

    let email_message = Message::builder()
        .from("noreply.blz.service@gmail.com".parse()?)
        .to(email.parse()?)
        .subject("Email Verification Code")
        .multipart(
            MultiPart::alternative()
                .singlepart(SinglePart::plain(plain_body))
                .singlepart(SinglePart::html(html_body)),
        )?;

    let creds = Credentials::new("noreply.blz.service@gmail.com".to_string(), app_password);

    let mailer = SmtpTransport::relay("smtp.gmail.com")?
        .credentials(creds)
        .build();

    let response: bool = match mailer.send(&email_message) {
        Ok(_) => {
            info!("Verification email sent successfully to {}", email);
            true
        }
        Err(e) => {
            error!("Could not send email: {:?}", e);
            // Clean up OTP record if email fails
            let _ = otp_datastore.delete(&email.to_string());
            false
        }
    };

    Ok(response)
}

/// Cleans up expired OTP records from the datastore
/// This should be called periodically (e.g., via a background task)
pub async fn cleanup_expired_otps() -> Result<usize> {
    let otp_datastore =
        DataStore::<String, OtpRecord>::new(get_data_path().await.join("otps.json"))?;

    let now = Utc::now();
    let entries = otp_datastore.entries()?;
    let mut removed_count = 0;

    for (email, record) in entries {
        if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&record.expires_at) {
            if now > expires_at.with_timezone(&Utc) {
                otp_datastore.delete(&email)?;
                removed_count += 1;
                info!("Cleaned up expired OTP for {}", email);
            }
        }
    }

    if removed_count > 0 {
        info!("Cleaned up {} expired OTP(s)", removed_count);
    }

    Ok(removed_count)
}
