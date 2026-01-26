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
