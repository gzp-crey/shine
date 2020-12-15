use data_encoding::HEXLOWER;
use ring::digest::{Context as RingContext, SHA256};

pub struct ContentHash {
    hash: String,
}

impl ContentHash {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(data: &str) -> ContentHash {
        let mut context = RingContext::new(&SHA256);
        context.update(data.as_bytes());
        let hash = context.finish();
        ContentHash {
            hash: HEXLOWER.encode(hash.as_ref()),
        }
    }

    pub fn from_bytes(data: &[u8]) -> ContentHash {
        let mut context = RingContext::new(&SHA256);
        context.update(data);
        let hash = context.finish();
        ContentHash {
            hash: HEXLOWER.encode(hash.as_ref()),
        }
    }

    pub fn from_multiple_bytes(data: &[&[u8]]) -> ContentHash {
        let mut context = RingContext::new(&SHA256);
        for d in data {
            context.update(d);
        }
        let hash = context.finish();
        ContentHash {
            hash: HEXLOWER.encode(hash.as_ref()),
        }
    }

    pub fn builder() -> ContentHashBuilder {
        ContentHashBuilder(RingContext::new(&SHA256))
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Create a storage compatible path from a hash
    pub fn to_path(&self) -> String {
        let hash = self.hash();
        format!("{}/{}", &hash[..4], &hash[4..32])
    }

    pub fn into_hash(self) -> String {
        self.hash
    }
}

/// Helper to hash multi-part content
pub struct ContentHashBuilder(RingContext);

impl ContentHashBuilder {
    pub fn add(&mut self, data: &[u8]) -> &mut Self {
        self.0.update(data);
        self
    }

    pub fn build(self) -> ContentHash {
        let hash = self.0.finish();
        ContentHash {
            hash: HEXLOWER.encode(hash.as_ref()),
        }
    }
}
