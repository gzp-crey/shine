use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA256};

/// Helper to construct hash for content
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(data);
    let hash = context.finish();
    HEXLOWER.encode(hash.as_ref())
}

/// Helper to hash multi-part content
pub struct ContentHasher(Context);

impl ContentHasher {
    pub fn new() -> ContentHasher {
        ContentHasher(Context::new(&SHA256))
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
    format!("{}/{}", &hash[..4], &hash[4..])
}

/// Implement the trait to generate hash-ed content path
pub trait HashableContent {
    fn hash(self) -> String;

    fn hashed_path(self) -> String
    where
        Self: Sized,
    {
        hash_to_path(&self.hash())
    }
}

impl HashableContent for &[u8] {
    fn hash(self) -> String {
        sha256_bytes(self)
    }
}

impl HashableContent for ContentHasher {
    fn hash(self) -> String {
        ContentHasher::hash(self)
    }
}
