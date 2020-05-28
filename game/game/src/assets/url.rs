use std::path::PathBuf;
pub use url::ParseError as UrlError;
pub use url::Position;

///A wrapper around url as it has some strange sematics due to the web sepcification.
#[derive(Clone)]
pub struct Url {
    inner: url::Url,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        Ok(Url {
            inner: url::Url::parse(input)?,
        })
    }

    pub fn from_base_or_current(base: &Url, current: &Url, input: &str) -> Result<Self, UrlError> {
        if input.starts_with('/') {
            // input is relative to the base
            base.join(input)
        } else {
            // input is relative to the current
            current.to_folder().and_then(|url| url.join(input))
        }
    }

    pub fn to_folder(&self) -> Result<Url, UrlError> {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '/');
        let first = parts.next();
        let second = parts.next();
        let folder = first.and(second).unwrap_or("");

        Url::parse(&format!(
            "{}{}/{}",
            &self.inner[..url::Position::BeforeHost],
            folder,
            &self.inner[url::Position::AfterPath..]
        ))
    }

    pub fn path(&self) -> &str {
        &self.inner[url::Position::BeforePath..url::Position::AfterPath]
    }

    pub fn relative_path(&self, base: &Url) -> Option<&str> {
        let path = &self.inner[..url::Position::AfterPath];
        let prefix = base.as_str();
        path.strip_prefix(prefix)
    }

    pub fn to_file_path(&self) -> PathBuf {
        PathBuf::from(&self.inner[url::Position::BeforeHost..url::Position::AfterPath])
    }

    pub fn to_file_folder(&self) -> PathBuf {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '/');
        let first = parts.next();
        let second = parts.next();
        let folder = first.and(second).unwrap_or("");
        PathBuf::from(folder)
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    pub fn set_scheme(&self, scheme: &str) -> Result<Url, UrlError> {
        Url::parse(&format!("{}{}", scheme, &self.inner[url::Position::AfterScheme..]))
    }

    pub fn extension(&self) -> &str {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        second.and(first).unwrap_or("")
    }

    pub fn set_extension(&self, ext: &str) -> Result<Url, UrlError> {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        let base = second.or(first).unwrap_or("");

        Url::parse(&format!(
            "{}{}.{}{}",
            &self.inner[..url::Position::BeforeHost],
            base,
            ext,
            &self.inner[url::Position::AfterPath..]
        ))
    }

    pub fn join(&self, path: &str) -> Result<Url, UrlError> {
        Url::parse(&format!(
            "{}{}{}",
            &self.inner[..url::Position::AfterPath],
            path,
            &self.inner[url::Position::AfterPath..]
        ))
    }
}
