use data_encoding::HEXLOWER;
use ring::digest::{Context, SHA256};
use shine_game::utils::{assets, url::Url};

pub fn sha256_bytes(data: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(data);
    let hash = context.finish();
    HEXLOWER.encode(hash.as_ref())
}

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

pub fn hash_to_path(hash: &str) -> String {
    format!("{}/{}", &hash[..4], &hash[4..])
}

pub async fn upload_cooked_binary(target_base: &Url, ext: &str, content: &[u8]) -> Result<String, String> {
    let hash = sha256_bytes(&content);
    let hash = hash_to_path(&hash);
    let target_id = format!("{}.{}", hash, ext);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;

    log::trace!("Uploading binary [{}]", target_url.as_str());
    assets::upload_binary(&target_url, &content)
        .await
        .map_err(|err| format!("Failed to upload [{}]: {:?}", target_url.as_str(), err))?;

    Ok(target_id)
}
