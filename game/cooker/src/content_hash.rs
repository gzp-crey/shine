use data_encoding;
use ring;

use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA256};

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(data);
    let hash = context.finish();
    HEXLOWER.encode(hash.as_ref())
}
