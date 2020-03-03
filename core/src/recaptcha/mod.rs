use reqwest::{Client, Error as ReqwestError};
use serde::Deserialize;

pub enum RecaptchaError {
    Internal(String),
    Rejected(String),
}

impl From<ReqwestError> for RecaptchaError {
    fn from(err: ReqwestError) -> RecaptchaError {
        log::info!("{:?}", err);
        RecaptchaError::Internal(format!("Communication error: {}", err))
    }
}

#[derive(Debug, Deserialize)]
struct RecaptchaResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct Recaptcha {
    client: Client,
    secret: String,
    site_key: String,
}

impl Recaptcha {
    pub fn new(secret: String, site_key: String) -> Recaptcha {
        Recaptcha {
            client: Client::new(),
            secret,
            site_key,
        }
    }

    pub fn site_key(&self) -> &str {
        &self.site_key
    }

    pub async fn check_response(&self, response: &str) -> Result<(), RecaptchaError> {
        let response = self
            .client
            .post("https://www.google.com/recaptcha/api/siteverify")
            .form(&[("secret", self.secret.as_str()), ("response", response)])
            .send()
            .await?;

        let response = response.json::<RecaptchaResponse>().await?;
        if response.success {
            Ok(())
        } else {
            let error = response
                .error_codes
                .map(|e| e.join(", "))
                .unwrap_or("".to_owned());
            log::info!("recaptcha failed: {:?}", error);
            Err(RecaptchaError::Rejected(error))
        }
    }
}
