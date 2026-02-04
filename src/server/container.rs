use hex::encode;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha512;
#[inline]
pub fn get_unique_instance_id(email: String) -> String {
    let mut instance_id = [0u8; 16];

    dotenv::dotenv().ok();

    let super_secret =
        std::env::var("BLAZE_INSTANCE_SECRET").expect("BLAZE_INSTANCE_SECRET must be set in env");

    let super_secret = super_secret.as_bytes();

    let email = email.trim().to_lowercase();

    pbkdf2_hmac::<Sha512>(email.as_bytes(), super_secret, 100_000, &mut instance_id);
    encode(instance_id)
}
