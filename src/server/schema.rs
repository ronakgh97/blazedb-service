use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserLoginRequest {
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]

pub struct UserLoginResponse {
    pub is_created: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub otp: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VerifyEmailResponse {
    pub is_verified: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    username: String,
    email: String,
    api_key: Option<String>,
    is_verified: bool,
    plans: Plans,
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
