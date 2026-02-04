pub use crate::prelude::{
    Plans, User, UserRegisterRequest, UserRegisterResponse, VerifyEmailRequest, VerifyEmailResponse,
};
use crate::server::crypto::{APIKey, hash_otp, verify_otp as crypto_verify_otp};
pub use crate::server::schema::{OtpRecord, UserStats, VerifyOtpRequest, VerifyOtpResponse};
use crate::server::storage::DataStore;
use crate::{error, info};
use anyhow::Result;
use chrono::{Duration, Utc};
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

static OTP_CACHE: std::sync::OnceLock<Arc<RwLock<HashMap<String, OtpRecord>>>> =
    std::sync::OnceLock::new();
const OTP_COOLDOWN_SECONDS: i64 = 30; // 30 seconds cooldown between OTP requests
static OTP_RATE_LIMIT: std::sync::OnceLock<Arc<RwLock<HashMap<String, i64>>>> =
    std::sync::OnceLock::new();
static USER_STORE: std::sync::OnceLock<DataStore<String, User>> = std::sync::OnceLock::new();
fn get_otp_cache() -> Arc<RwLock<HashMap<String, OtpRecord>>> {
    OTP_CACHE
        .get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
        .clone()
}
fn get_rate_limit_cache() -> Arc<RwLock<HashMap<String, i64>>> {
    OTP_RATE_LIMIT
        .get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
        .clone()
}
async fn get_user_store() -> DataStore<String, User> {
    USER_STORE
        .get_or_init(|| {
            let path = get_data_path().join("users.json");
            DataStore::<String, User>::new(path).unwrap()
        })
        .clone()
}

/// Creates necessary directories for the service: data, logs, and billing.
pub async fn create_dirs() -> Result<()> {
    let data_path = get_data_path();
    let logs_path = get_logs_path();
    let billing_path = get_billing_path();

    tokio::fs::create_dir_all(&data_path).await?;
    tokio::fs::create_dir_all(&logs_path).await?;
    tokio::fs::create_dir_all(&billing_path).await?;
    Ok(())
}

/// Creates a daily log directory based on the current date.
pub async fn create_logs_dir() -> Result<PathBuf> {
    let logs_path = get_logs_path();
    let server_time = chrono::Local::now();
    let daily_log_path = logs_path.join(server_time.format("%Y-%m-%d").to_string());
    tokio::fs::create_dir_all(&daily_log_path).await?;
    Ok(daily_log_path)
}

pub fn get_data_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("data")
}

pub fn get_logs_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("logs")
}

pub fn get_billing_path() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home_dir.join("blz_service").join("billings")
}

/// Saves a new user to the datastore (disk) and returns a response indicating success.
pub async fn save_user(user_data: &UserRegisterRequest) -> Result<UserRegisterResponse> {
    let user_store = get_user_store().await;

    // Create a user with email as the key
    let user = User {
        username: user_data.username.clone(),
        email: user_data.email.clone(),
        api_key: Vec::new(),
        is_verified: false,
        plans: Plans::free_plan(),
        instance_url: "".to_string(),
        created_at: Utc::now().to_rfc3339(),
    };

    // Insert in memory only - periodic background task will save to disk
    user_store.insert_mem(user_data.email.clone(), user)?;

    let response = UserRegisterResponse {
        email: user_data.email.clone(),
        is_created: true,
        error: "null".to_string(),
    };

    Ok(response)
}

/// Checks if a user with the given email exists in the datastore.
pub async fn is_user_exists(email: &String) -> Result<bool> {
    let datastore = get_user_store().await;
    if let Some(_user) = datastore.get(email)? {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Checks if the user with the given email is verified
pub async fn is_user_verified(email: &String) -> Result<bool> {
    let datastore = get_user_store().await;
    if let Some(user) = datastore.get(email)? {
        Ok(user.is_verified)
    } else {
        Ok(false)
    }
}

/// Initiates the email verification process by sending a verification code to the user's email
pub async fn verify_user(data: &VerifyEmailRequest) -> Result<VerifyEmailResponse> {
    match send_verification_code(&data.email).await {
        Ok(is_sent) => Ok(VerifyEmailResponse {
            is_code_sent: is_sent,
            error: "".to_string(),
        }),
        Err(e) => Ok(VerifyEmailResponse {
            is_code_sent: false,
            error: format!("Failed to send verification code: {}", e),
        }),
    }
}

// TODO: Decouple the checks for explicit error status code
/// Verifies the OTP code provided by the user and updates their verification status
pub async fn verify_otp(data: &VerifyOtpRequest) -> Result<VerifyOtpResponse> {
    let otp_cache = get_otp_cache();

    // Check if OTP record exists for this email
    let otp_record = {
        let cache_read = otp_cache.read().await;
        cache_read.get(&data.email).cloned()
    };

    let otp_record = match otp_record {
        Some(record) => record,
        None => {
            return Ok(VerifyOtpResponse {
                is_verified: false,
                message: "No verification code found for this email".to_string(),
                api_key: None,
                instance_id: None,
            });
        }
    };

    // Check if OTP has expired
    let now = Utc::now();
    let expires_at =
        chrono::DateTime::parse_from_rfc3339(&otp_record.expires_at)?.with_timezone(&Utc);

    if now > expires_at {
        // Clean up expired OTP
        let mut cache_write = otp_cache.write().await;
        cache_write.remove(&data.email);
        return Ok(VerifyOtpResponse {
            is_verified: false,
            message: "Verification code has expired".to_string(),
            api_key: None,
            instance_id: None,
        });
    }

    // Verify the OTP
    let otp_hash_bytes = hex::decode(&otp_record.otp_hash)?;
    let is_valid = crypto_verify_otp(&data.otp, &otp_hash_bytes).await;

    if !is_valid {
        return Ok(VerifyOtpResponse {
            is_verified: false,
            message: "Invalid verification code".to_string(),
            api_key: None,
            instance_id: None,
        });
    }

    // OTP is valid - update user verification status
    let user_datastore = get_user_store().await;

    // Get user by email (direct lookup since email is the key)
    let mut user = match user_datastore.get(&data.email)? {
        Some(u) => u,
        None => {
            return Ok(VerifyOtpResponse {
                is_verified: false,
                message: "User not found".to_string(),
                api_key: None,
                instance_id: None,
            });
        }
    };

    // Update verification status (memory only)
    user.is_verified = true;
    user_datastore.insert_mem(data.email.clone(), user)?;

    // Clean up used OTP from memory cache
    {
        let mut cache_write = otp_cache.write().await;
        cache_write.remove(&data.email);
    }

    info!("User {} successfully verified", data.email);

    // Assign API key upon successful verification (memory only)
    let mut user = user_datastore.get(&data.email)?.unwrap();
    let (api_key_struct, plain_key) = APIKey::get_new_key(&user.username, &user.email).await;
    user.api_key.push(api_key_struct.clone());
    user_datastore.insert_mem(data.email.clone(), user.clone())?;

    Ok(VerifyOtpResponse {
        is_verified: true,
        message: "Email verified successfully".to_string(),
        api_key: Some(plain_key), // Return plain key ONLY this once
        instance_id: Some("TODO".to_string()), // TODO: Generate and return user-specific instance URL
    })
}

/// Verifies an API key and returns the associated user email if valid
/// Returns None if the key is invalid or revoked
pub async fn verify_api_key(api_key: &str) -> Result<Option<String>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;

    // Search through all users for a matching API key
    for user in all_users {
        for key in &user.api_key {
            if key.verify(api_key).await {
                return Ok(Some(user.email.clone()));
            }
        }
    }

    Ok(None)
}

/// Just Sends a verification code (OTP) to the specified email address and stores the hashed OTP in the datastore
pub async fn send_verification_code(email: &str) -> Result<bool> {
    let rate_limit_cache = get_rate_limit_cache();
    let now_timestamp = Utc::now().timestamp();

    // Check rate limiting
    {
        let rate_read = rate_limit_cache.read().await;
        if let Some(&last_request) = rate_read.get(email) {
            let elapsed = now_timestamp - last_request;
            if elapsed < OTP_COOLDOWN_SECONDS {
                let remaining = OTP_COOLDOWN_SECONDS - elapsed;
                info!(
                    "Rate limit hit for {}: {} seconds remaining",
                    email, remaining
                );
                return Err(anyhow::anyhow!(
                    "Please wait {} seconds before requesting a new code",
                    remaining
                ));
            }
        }
    }

    // Generate a random 6-digit OTP
    let otp: String = (0..6)
        .map(|_| rand::random::<u8>() % 10)
        .map(|digit| char::from(b'0' + digit))
        .collect();

    let otp_hash = hash_otp(&otp).await;
    let otp_hash_hex = hex::encode(&otp_hash);

    let now = Utc::now();
    let expires_at = now + Duration::minutes(1); // OTP valid for 1 minute

    let otp_record = OtpRecord {
        email: email.to_string(),
        otp_hash: otp_hash_hex,
        created_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
    };

    // Store OTP in thread-safe in-memory cache
    let otp_cache = get_otp_cache();
    {
        let mut cache_write = otp_cache.write().await;
        cache_write.insert(email.to_string(), otp_record.clone());
    }

    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {{
                    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
                    background-color: #f6f9fc;
                    margin: 0;
                    padding: 0;
                    color: #333;
                }}
                .container {{
                    max-width: 600px;
                    margin: 40px auto;
                    background: #ffffff;
                    border-radius: 8px;
                    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
                    overflow: hidden;
                }}
                .header {{
                    background: linear-gradient(135deg, #0052cc 0%, #007bff 100%);
                    padding: 30px;
                    text-align: center;
                }}
                .header h1 {{
                    color: white;
                    margin: 0;
                    font-size: 24px;
                    font-weight: 600;
                }}
                .content {{
                    padding: 40px;
                    text-align: center;
                }}
                .otp {{
                    font-family: monospace;
                    font-size: 32px;
                    letter-spacing: 8px;
                    font-weight: bold;
                    color: #0052cc;
                    background: #eef2f7;
                    padding: 24px;
                    border-radius: 6px;
                    margin: 30px 0;
                    display: inline-block;
                }}
                .footer {{
                    background-color: #f8f9fa;
                    padding: 20px;
                    text-align: center;
                    font-size: 12px;
                    color: #6c757d;
                    border-top: 1px solid #eee;
                }}
            </style>
        </head>
        <body>
            <div class="container">
                <div class="header">
                    <h1>BlazeDB Verification</h1>
                </div>
                <div class="content">
                    <p style="font-size: 16px;">Please use the verification code below to get your Free API KEY.</p>
                    <div class="otp">{}</div>
                    <p style="color: #666; font-size: 14px;">This code will expire in 5 minutes.</p>
                </div>
                <div class="footer">
                    <p>If you didn't request this code, you can safely ignore this email.</p>
                </div>
            </div>
        </body>
        </html>
        "#,
        otp
    );

    let plain_body = format!("Your BlazeDB OTP: {}\n\nExpires in 5 minutes.", otp);

    dotenv::dotenv().ok();

    // Get app_passwords from env
    let app_password = std::env::var("APP_PASSWORD").expect("APP_PASSWORD must be set 🤬");

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
            // Update rate limit timestamp on successful send
            let mut rate_write = rate_limit_cache.write().await;
            rate_write.insert(email.to_string(), now_timestamp);
            info!("OTP sent to {} (rate limit updated)", email);
            true
        }
        Err(e) => {
            error!("Could not send email: {:?}", e);
            // Clean up OTP record from memory cache if email fails
            let otp_cache = get_otp_cache();
            let mut cache_write = otp_cache.write().await;
            cache_write.remove(&email.to_string());
            false
        }
    };

    Ok(response)
}

/// Cleans up expired OTP records from the in-memory cache
/// This is called periodically via a background task
pub async fn cleanup_expired_otps() -> Result<usize> {
    let otp_cache = get_otp_cache();
    let rate_limit_cache = get_rate_limit_cache();
    let now = Utc::now();
    let mut removed_count = 0;

    let expired_emails: Vec<String> = {
        let cache_read = otp_cache.read().await;
        cache_read
            .iter()
            .filter_map(|(email, record)| {
                if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&record.expires_at) {
                    if now > expires_at.with_timezone(&Utc) {
                        return Some(email.clone());
                    }
                }
                None
            })
            .collect()
    };

    // Remove expired OTPs
    if !expired_emails.is_empty() {
        let mut cache_write = otp_cache.write().await;
        for email in &expired_emails {
            cache_write.remove(email);
            removed_count += 1;
            info!("Cleaned up expired OTP for {}", email);
        }
    }

    let mut rate_write = rate_limit_cache.write().await;
    rate_write.clear();
    Ok(removed_count)
}

/// Periodically saves user data from memory to disk
pub async fn periodic_save_users() -> Result<()> {
    let user_store = get_user_store().await;
    user_store.save_to_disk()?;
    Ok(())
}

/// Retrieves all users from the datastore
pub async fn get_all_users() -> Result<Vec<User>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;
    Ok(all_users)
}

/// Retrieves all users who are not verified
pub async fn get_unverified_users() -> Result<Vec<User>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;

    let unverified_users: Vec<User> = all_users
        .par_iter()
        .filter(|user| !user.is_verified)
        .cloned()
        .collect();

    Ok(unverified_users)
}

/// Retrieves all users who are on the free plan
pub async fn get_all_free_users() -> Result<Vec<User>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;

    let free_users: Vec<User> = all_users
        .par_iter()
        .filter(|user| user.plans.name == "Free")
        .cloned()
        .collect();

    Ok(free_users)
}

/// Retrieves all users who are on the starter plan
pub async fn get_all_starter_users() -> Result<Vec<User>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;

    let starter_users: Vec<User> = all_users
        .par_iter()
        .filter(|user| user.plans.name == "Starter")
        .cloned()
        .collect();

    Ok(starter_users)
}

/// Retrieves all users who are on the pro plan
pub async fn get_all_pro_users() -> Result<Vec<User>> {
    let user_datastore = get_user_store().await;
    let all_users = user_datastore.values()?;

    let pro_users: Vec<User> = all_users
        .par_iter()
        .filter(|user| user.plans.name == "Pro")
        .cloned()
        .collect();

    Ok(pro_users)
}
