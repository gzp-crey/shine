use reqwest::Client;

pub enum RecaptchaError {
    Internal(String),
    Rejected,
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
            .body(format!("secret={}&response={}", self.secret, response))
            .send()
            .await;

        log::info!("recaptcha: {:?}", response);
        Ok(())
    }
}
