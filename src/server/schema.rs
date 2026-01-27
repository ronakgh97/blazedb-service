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
    pub api_key: Option<APIKey>,
    pub is_verified: bool,
    pub plans: Plans,
    pub instance_url: String,
    pub created_at: String,
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
}

impl Plans {
    pub fn free_plan() -> Self {
        Plans {
            name: "Free".to_string(),
            price_per_month: 0,
            features: Feature {
                database_no: 1,
                vector_per_db: 10000,
            },
        }
    }

    pub fn starter_plan() -> Self {
        Plans {
            name: "Starter".to_string(),
            price_per_month: 9,
            features: Feature {
                database_no: 10,
                vector_per_db: 100000,
            },
        }
    }

    pub fn pro_plan() -> Self {
        Plans {
            name: "Pro".to_string(),
            price_per_month: 29,
            features: Feature {
                database_no: 100,
                vector_per_db: 1000000,
            },
        }
    }
}
