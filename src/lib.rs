pub mod server;

pub mod prelude {
    pub use crate::server::crypto::{
        generate_api_key, generate_key, generate_salt, hash_otp, verify_otp,
    };
    pub use crate::server::log;
    pub use crate::server::schema::{
        Feature, OtpRecord, Plans, User, UserRegisterRequest, UserRegisterResponse,
        VerifyEmailRequest, VerifyEmailResponse, VerifyOtpRequest, VerifyOtpResponse,
    };
    pub use crate::server::service::{
        cleanup_expired_otps, create_dirs, create_logs_dir, get_billing_path, get_data_path,
        get_logs_path, verify_otp as verify_otp_service,
    };
}
