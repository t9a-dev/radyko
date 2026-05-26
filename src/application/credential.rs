use std::{env, path::PathBuf};

use secrecy::{ExposeSecret, SecretString};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RadikoCredential {
    email_address: SecretString,
    password: SecretString,
}

impl RadikoCredential {
    pub fn load_from_env_file() -> Option<RadikoCredential> {
        let env_file_path = PathBuf::from(".env");
        let _ = dotenvy::from_path(&env_file_path);
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
                warn!(
                    "failed load radiko credential from environment env_file_path: {env_file_path:#?}"
                );
                None
            }
        }
    }

    pub fn expose_email_address(&self) -> String {
        self.email_address.expose_secret().to_string()
    }

    pub fn expose_password(&self) -> String {
        self.password.expose_secret().to_string()
    }
}
