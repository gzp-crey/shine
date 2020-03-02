use super::AntiForgerySession;
use actix_web::Error as ActixError;
use rand::{self, seq::SliceRandom};
use serde::{Deserialize, Serialize};

const TOKEN_LEN: usize = 8;
const TOKEN_ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub enum AntiForgeryError {
    Missing,
    Expired,
    InvalidToken,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum AntiForgeryIdentity {
    Ignore,
    None,
    Identity(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct AntiForgeryData {
    token: String,
    identity: AntiForgeryIdentity,
}

impl AntiForgeryData {
    fn new(identity: AntiForgeryIdentity) -> Self {
        let mut rng = rand::thread_rng();
        let token = String::from_utf8(
            TOKEN_ABC
                .choose_multiple(&mut rng, TOKEN_LEN)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .unwrap();

        AntiForgeryData { token, identity }
    }
}

pub struct AntiForgeryIssuer<'a> {
    data: AntiForgeryData,
    session: &'a AntiForgerySession,
}

impl<'a> AntiForgeryIssuer<'a> {
    pub fn new<'b>(session: &'b AntiForgerySession, identity: Option<String>) -> AntiForgeryIssuer<'b> {
        AntiForgeryIssuer {
            data: AntiForgeryData::new(
                identity
                    .map(|i| AntiForgeryIdentity::Identity(i))
                    .unwrap_or(AntiForgeryIdentity::None),
            ),
            session: session,
        }
    }

    pub fn token(&self) -> &str {
        &self.data.token
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
    identity: AntiForgeryIdentity,
    _session: &'a AntiForgerySession,
}

impl<'a> AntiForgeryValidator<'a> {
    pub fn new<'b>(
        session: &'b AntiForgerySession,
        identity: AntiForgeryIdentity,
    ) -> Result<AntiForgeryValidator<'b>, ActixError> {
        let data = session.get::<AntiForgeryData>("d")?;
        Ok(AntiForgeryValidator {
            data,
            _session: session,
            identity,
        })
    }

    pub fn validate(&self, token: &str) -> Result<&str, AntiForgeryError> {
        if let Some(ref data) = self.data {
            if self.identity != AntiForgeryIdentity::Ignore && self.identity != data.identity {
                // either user existance is ignored or it must match the AF cookie
                log::info!("AF id missmatch: {:?}, {:?}", self.identity, data.identity);
                Err(AntiForgeryError::InvalidToken)
            } else if data.token != token {
                log::info!("AF token missmatch: {:?}, {:?}", token, data.token);
                Err(AntiForgeryError::InvalidToken)
            } else {
                Ok(&data.token)
            }
        } else {
            Err(AntiForgeryError::Missing)
        }
    }
}
