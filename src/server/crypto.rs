use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct APIKey {
    pub user_name: String,
    pub user_email: String,
    pub api_key_hash: String,
    pub key_prefix: String,
    pub is_revoked: bool,
    pub created_at: String,
}

impl APIKey {
    /// Generates a new APIKey for the given username and email.
    /// Returns (APIKey with hash, plain_text_key for one-time display)
    pub async fn get_new_key(user_name: &str, user_email: &str) -> (Self, String) {
        let plain_key = generate_api_key(user_name, user_email).await;
        let key_hash = hash_api_key(&plain_key).await;
        let prefix = plain_key.chars().take(12).collect::<String>() + "...";

        let api_key = APIKey {
            user_name: user_name.to_string(),
            user_email: user_email.to_string(),
            api_key_hash: key_hash,
            key_prefix: prefix,
            is_revoked: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        (api_key, plain_key)
    }

    /// Revokes the API key.
    pub async fn revoke(&mut self) {
        self.is_revoked = true;
    }

    /// Verifies if the provided plain API key matches this stored hash
    pub async fn verify(&self, plain_key: &str) -> bool {
        if self.is_revoked {
            return false;
        }
        let key_hash = hash_api_key(plain_key).await;
        key_hash == self.api_key_hash
    }
}

/// Generates a cryptographic salt of the specified length in bytes.
pub async fn generate_salt(len: usize) -> Vec<u8> {
    use rand::RngCore;
    let mut salt = vec![0u8; len];
    rand::rng().fill_bytes(&mut salt);
    salt
}

/// Generates a secure key using PBKDF2 with HMAC-SHA256.
/// The key is derived from the user's name and email, combined with a provided salt.
pub async fn generate_key(user_name: &str, user_email: &str, salt: &[u8]) -> Vec<u8> {
    let mut key = vec![0u8; 16];
    let password = format!("{}:{}", user_name, user_email);
    pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        salt,
        100_000, // Number of iterations
        &mut key,
    );
    key
}

/// Generates an API key for the user by combining a derived key and a salt.
/// The API key is formatted as "blz_<key>_<salt>"
pub async fn generate_api_key(user_name: &str, user_email: &str) -> String {
    let salt = generate_salt(20).await;
    let key = generate_key(user_name, user_email, &salt).await;
    format!("blz_{}_{}", hex::encode(&key), hex::encode(&salt))
}

/// Hashes the provided one-time password (OTP) using SHA-256.
pub async fn hash_otp(otp: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(otp.as_bytes());
    hasher.finalize().to_vec()
}

/// Hashes the provided API key using SHA-256 and returns hex-encoded string
pub async fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Verifies the provided OTP against the stored hash.
pub async fn verify_otp(otp: &str, hash: &[u8]) -> bool {
    let otp_hash = hash_otp(otp).await;
    otp_hash == hash
}

#[tokio::test]
async fn test_api_key_generation() -> anyhow::Result<()> {
    let user_name = "ronakgh97";
    let user_email = "ronakgh999@gmail.com";

    let api_key = generate_api_key(user_name, user_email).await;
    println!("Generated API Key: {}", api_key);

    assert!(api_key.len() > 20);
    assert!(api_key.contains("blz"));

    Ok(())
}
