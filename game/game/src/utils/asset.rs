use url::Url;

#[derive(Debug)]
pub enum AssetError {
    AssetNotFound,
}

#[cfg(feature = "native")]
pub async fn download_vec_u32(url: &Url) -> Result<Vec<u32>, AssetError> {
    //use tokio::fs;
    unimplemented!()
}
