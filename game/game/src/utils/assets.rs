use crate::utils::url::Url;

#[derive(Debug)]
pub enum AssetError {
    AssetNotFound,
    UnsupportedScheme(String),
    ContentLoad(String),
    ContentSave(String),
    TODO,
}

#[cfg(feature = "native")]
pub async fn download_string(url: &Url) -> Result<String, AssetError> {
    match url.scheme() {
        "file" => {
            let data = tokio::fs::read_to_string(&url.to_file_path())
                .await
                .map_err(|_err| AssetError::AssetNotFound)?;
            Ok(data)
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

#[cfg(feature = "native")]
pub async fn download_binary(url: &Url) -> Result<Vec<u8>, AssetError> {
    match url.scheme() {
        "file" => {
            use tokio::io::AsyncReadExt;
            let mut file = tokio::fs::File::open(&url.to_file_path())
                .await
                .map_err(|_err| AssetError::AssetNotFound)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .await
                .map_err(|err| AssetError::ContentLoad(format!("Load failed: {:?}", err)))?;
            Ok(data)
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

#[cfg(feature = "native")]
pub async fn upload_binary(url: &Url, data: &[u8]) -> Result<(), AssetError> {
    match url.scheme() {
        "file" => {
            tokio::fs::create_dir_all(url.to_file_folder())
                .await
                .map_err(|err| AssetError::ContentSave(format!("Failed to create folder: {:?}", err)))?;
            tokio::fs::write(&url.to_file_path(), data)
                .await
                .map_err(|err| AssetError::ContentSave(format!("Save failed: {:?}", err)))?;
            Ok(())
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

#[cfg(feature = "native")]
pub async fn upload_string(url: &Url, data: &str) -> Result<(), AssetError> {
    match url.scheme() {
        "file" => {
            tokio::fs::create_dir_all(url.to_file_folder())
                .await
                .map_err(|err| AssetError::ContentSave(format!("Failed to create folder: {:?}", err)))?;
            tokio::fs::write(&url.to_file_path(), data)
                .await
                .map_err(|err| AssetError::ContentSave(format!("Save failed: {:?}", err)))?;
            Ok(())
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}
