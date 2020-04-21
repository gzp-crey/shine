use std::path::PathBuf;
use url;

pub use url::{ParseError, Position};

///A wrapper around url as it has some strange sematics due to the web sepcification.
pub struct Url {
    inner: url::Url,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        Ok(Url {
            inner: url::Url::parse(input)?,
        })
    }

    pub fn to_file_path(&self) -> PathBuf {
        PathBuf::from(&self.inner[url::Position::BeforeHost..url::Position::BeforeQuery])
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    pub fn extension(&self) -> &str {
        let path = &self.inner[url::Position::BeforeHost..url::Position::BeforeQuery];
        let mut parts = path.rsplitn(2, ".");
        let first = parts.next();
        let second = parts.next();
        second.and(first).unwrap_or("")
    }

    pub fn set_extension(&self, ext: &str) -> Result<Url, ParseError> {
        let path = &self.inner[url::Position::BeforeHost..url::Position::BeforeQuery];
        let mut parts = path.rsplitn(2, ".");
        let first = parts.next().unwrap_or("");

        Url::parse(&format!(
            "{}{}.{}{}",
            &self.inner[..url::Position::BeforeHost],
            first,
            ext,
            &self.inner[url::Position::BeforeQuery..]
        ))
    }

    pub fn join(&self, path: &str) -> Result<Url, ParseError> {
        Url::parse(&format!(
            "{}{}{}",
            &self.inner[..url::Position::BeforeQuery],
            path,
            &self.inner[url::Position::BeforeQuery..]
        ))
    }

    /*pub fn set_extension(&mut self, ext: &str) {

    }*/
}
