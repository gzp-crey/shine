use super::AntiForgerySession;
use crate::serde_with;
use actix_web::Error as ActixError;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::{self, seq::SliceRandom};
use serde::{Deserialize, Serialize};

const TOKEN_LEN: usize = 8;
const TOKEN_ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub enum AntiForgeryError {
    Missing,
    Expired,
    InvalidToken,
}

#[derive(Debug, Serialize, Deserialize)]
struct AntiForgeryData {
    token: String,
    #[serde(with = "serde_with::datetime")]
    issued: DateTime<Utc>,
}

impl AntiForgeryData {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let token = String::from_utf8(TOKEN_ABC.choose_multiple(&mut rng, TOKEN_LEN).cloned().collect::<Vec<_>>()).unwrap();

        AntiForgeryData {
            token,
            issued: Utc::now(),
        }
    }
}

pub struct AntiForgeryIssuer<'a> {
    data: AntiForgeryData,
    session: &'a AntiForgerySession,
}

impl<'a> AntiForgeryIssuer<'a> {
    pub fn new<'b>(session: &'b AntiForgerySession) -> AntiForgeryIssuer<'b> {
        AntiForgeryIssuer {
            data: AntiForgeryData::new(),
            session: session,
        }
    }
}

impl<'a> Drop for AntiForgeryIssuer<'a> {
    fn drop(&mut self) {
        if let Err(err) = self.session.set("d", &self.data) {
            log::error!("Failed to set AF cookie: {}", err);
        }
    }
}

pub struct AntiForgeryValidator<'a> {
    data: Option<AntiForgeryData>,
    session: &'a AntiForgerySession,
}

impl<'a> AntiForgeryValidator<'a> {
    pub fn new<'b>(session: &'b AntiForgerySession) -> Result<AntiForgeryValidator<'b>, ActixError> {
        let data = session.get::<AntiForgeryData>("d")?;
        Ok(AntiForgeryValidator { data, session })
    }

    fn ttl(&self) -> ChronoDuration {
        self.session.config().time_to_live
    }

    pub fn validate(&self, token: &str) -> Result<(), AntiForgeryError> {
        if let Some(ref data) = self.data {
            if data.token != token {
                Err(AntiForgeryError::InvalidToken)
            } else if data.issued + self.ttl() < Utc::now() {
                Err(AntiForgeryError::Expired)
            } else {
                Ok(())
            }
        } else {
            Err(AntiForgeryError::Missing)
        }
    }
}

impl<'a> Drop for AntiForgeryValidator<'a> {
    fn drop(&mut self) {
        self.session.clear();
    }
}
