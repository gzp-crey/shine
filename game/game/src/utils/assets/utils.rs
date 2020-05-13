use crate::utils::assets::{upload_binary, AssetError, HashableContent};
use crate::utils::url::Url;

/// Upload a binary data based on it's hashes_path
pub async fn upload_cooked_binary(target_base: &Url, ext: &str, content: &[u8]) -> Result<String, AssetError> {
    let hashed_path = content.hashed_path();
    let target_file = format!("{}.{}", hashed_path, ext);
    let target_url = target_base.join(&target_file)?;

    upload_binary(&target_url, &content).await?;

    Ok(target_file)
}
