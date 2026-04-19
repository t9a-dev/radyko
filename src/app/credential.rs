use secrecy::SecretString;
use std::env;
use tracing::{info, warn};
pub struct RadikoCredential {
    pub email_address: SecretString,
    pub password: SecretString,
}

impl RadikoCredential {
    pub fn load_credential() -> Option<RadikoCredential> {
        let dotenv_path = ".env";
        let _ = dotenvy::from_path(dotenv_path);
        let mail = env::var("RADIKO_AREA_FREE_MAIL");
        let password = env::var("RADIKO_AREA_FREE_PASSWORD");
        match (mail, password) {
            (Ok(mail), Ok(password)) => {
                info!("success load radiko credential from environment");
                Some(RadikoCredential {
                    email_address: SecretString::new(mail.into()),
                    password: SecretString::new(password.into()),
                })
            }
            _ => {
                warn!("failed load radiko credential from environment");
                None
            }
        }
    }
}
