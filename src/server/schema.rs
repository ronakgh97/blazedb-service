use crate::server::crypto::APIKey;
use serde::{Deserialize, Serialize};

/// Request structure for user registration
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserRegisterRequest {
    pub username: String,
    pub email: String,
}

/// Response structure for user registration
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserRegisterResponse {
    pub email: String,
    pub is_created: bool,
    pub error: String,
}

/// Request structure for email verification
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyEmailRequest {
    pub email: String,
}

/// Response structure if verification code is sent
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyEmailResponse {
    pub is_code_sent: bool,
    pub error: String,
}

/// Request structure for OTP verification
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyOtpRequest {
    pub email: String,
    pub otp: String,
}

/// Response structure for OTP verified or not
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyOtpResponse {
    pub is_verified: bool,
    pub message: String,
    pub api_key: Option<String>, // Return plain API key ONLY once after verification
    // pub instance_url: Option<String>, // Return instance URL ONLY once after verification
}

/// Structure representing an OTP record
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OtpRecord {
    pub email: String,
    pub otp_hash: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Structure representing a user
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub email: String,
    pub api_key: Vec<APIKey>,
    pub is_verified: bool,
    pub plans: Plans,
    pub instance_url: String,
    pub created_at: String,
}

/// Safe user stats structure for public endpoints
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserStats {
    pub username: String,
    pub email: String,
    pub api_keys_count: usize,
    pub api_key_prefixes: Vec<String>, // Only show prefixes like "blz_abc123..."
    pub is_verified: bool,
    pub plans: Plans,
    pub created_at: String,
}

impl From<User> for UserStats {
    fn from(user: User) -> Self {
        UserStats {
            username: user.username,
            email: user.email,
            api_keys_count: user.api_key.len(),
            api_key_prefixes: user.api_key.iter().map(|k| k.key_prefix.clone()).collect(),
            is_verified: user.is_verified,
            plans: user.plans,
            created_at: user.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Plans {
    pub name: String,
    pub price_per_month: u32,
    pub features: Feature,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Feature {
    pub database_no: u32,
    pub vector_per_db: u32,
    pub demo_datasets_included: bool,
    pub dedicated_server_instance: bool,
    pub embedding_api_access: bool,
}

impl Plans {
    pub fn free_plan() -> Self {
        Plans {
            name: "Free".to_string(),
            price_per_month: 0,
            features: Feature {
                database_no: 1,
                vector_per_db: 10_000,
                demo_datasets_included: true,
                dedicated_server_instance: false,
                embedding_api_access: false,
            },
        }
    }

    pub fn starter_plan() -> Self {
        Plans {
            name: "Starter".to_string(),
            price_per_month: 9,
            features: Feature {
                database_no: 10,
                vector_per_db: 100_000,
                demo_datasets_included: true,
                dedicated_server_instance: true,
                embedding_api_access: true,
            },
        }
    }

    pub fn pro_plan() -> Self {
        Plans {
            name: "Pro".to_string(),
            price_per_month: 29,
            features: Feature {
                database_no: 100,
                vector_per_db: 10_00_000,
                demo_datasets_included: true,
                dedicated_server_instance: true,
                embedding_api_access: true,
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserData {
    pub unverified_users: Vec<UserStats>,
    pub free_users: Vec<UserStats>,
    pub stater_users: Vec<UserStats>,
    pub pro_users: Vec<UserStats>,
}
