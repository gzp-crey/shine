use data_encoding::HEXLOWER;
use ring::digest::{Context as RingContext, SHA256};

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut context = RingContext::new(&SHA256);
    context.update(data);
    let hash = context.finish();
    HEXLOWER.encode(hash.as_ref())
}

pub fn sha256_multiple_bytes(data: &[&[u8]]) -> String {
    let mut context = RingContext::new(&SHA256);
    for d in data {
        context.update(d);
    }
    let hash = context.finish();
    HEXLOWER.encode(hash.as_ref())
}

/// Helper to hash multi-part content
pub struct ContentHasher(RingContext);

impl Default for ContentHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentHasher {
    pub fn new() -> Self {
        Self(RingContext::new(&SHA256))
    }

    pub fn add(&mut self, data: &[u8]) -> &mut Self {
        self.0.update(data);
        self
    }

    pub fn hash(self) -> String {
        let hash = self.0.finish();
        HEXLOWER.encode(hash.as_ref())
    }
}

/// Create a storage compatible path from a hash
pub fn hash_to_path(hash: &str) -> String {
    format!("{}/{}", &hash[..4], &hash[4..32])
}

/// Implement the trait to generate hash-ed content path
pub trait HashableContent {
    fn content_hash(self) -> String;

    fn content_hash_path(self) -> String
    where
        Self: Sized,
    {
        hash_to_path(&self.content_hash())
    }
}

impl HashableContent for &[u8] {
    fn content_hash(self) -> String {
        sha256_bytes(self)
    }
}

impl HashableContent for &str {
    fn content_hash(self) -> String {
        sha256_bytes(self.as_bytes())
    }
}

impl HashableContent for ContentHasher {
    fn content_hash(self) -> String {
        ContentHasher::hash(self)
    }
}
