pub mod server;

pub mod prelude {
    pub use crate::server::crypto::{
        generate_api_key, generate_key, generate_salt, hash_otp, verify_otp,
    };
    pub use crate::server::log;
    pub use crate::server::schema::{
        Feature, Plans, User, UserLoginRequest, UserLoginResponse, VerifyEmailRequest,
        VerifyEmailResponse,
    };
}
