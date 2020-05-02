use crate::utils::assets::AssetError;
use crate::utils::url::Url;
use reqwest::{self, Response, StatusCode};
use tokio::fs;
use tokio::io::AsyncReadExt;

pub async fn get_response(url: &Url) -> Result<Response, AssetError> {
    let response = reqwest::get(url.as_str())
        .await
        .map_err(|err| AssetError::AssetProvider(format!("Failed to download {}: {}", url.as_str(), err)))?;

    let status = response.status();
    if status.is_success() {
        let err = response.text().await.unwrap_or("".to_owned());
        Err(AssetError::AssetProvider(format!(
            "Unexpected status code ({}) for {}: {}",
            status,
            url.as_str(),
            err
        )))
    } else {
        Ok(response)
    }
}

pub async fn download_string(url: &Url) -> Result<String, AssetError> {
    match url.scheme() {
        "file" => {
            let mut data = String::new();
            let _ = fs::File::open(&url.to_file_path())
                .await
                .map_err(|err| AssetError::AssetProvider(format!("Failed to open file {}: {}", url.as_str(), err)))?
                .read_to_string(&mut data)
                .await
                .map_err(|err| AssetError::ContentLoad(format!("Failed to read from {}: {}", url.as_str(), err)))?;
            Ok(data)
        }
        "http" | "https" => {
            get_response(url).await?.text().await.map_err(|err| {
                AssetError::ContentLoad(format!("Failed to parse response for {}: {}", url.as_str(), err))
            })
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

pub async fn download_binary(url: &Url) -> Result<Vec<u8>, AssetError> {
    match url.scheme() {
        "file" => {
            let mut data = Vec::new();
            let _ = fs::File::open(&url.to_file_path())
                .await
                .map_err(|err| AssetError::AssetProvider(format!("Failed to open file {}: {}", url.as_str(), err)))?
                .read_to_end(&mut data)
                .await
                .map_err(|err| AssetError::ContentLoad(format!("Failed to read from {}: {}", url.as_str(), err)))?;
            Ok(data)
        }
        "http" | "https" => get_response(url)
            .await?
            .bytes()
            .await
            .map_err(|err| AssetError::ContentLoad(format!("Failed to parse response for {}: {}", url.as_str(), err)))
            .map(|d| d.to_vec()),
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

pub async fn upload_binary(url: &Url, data: &[u8]) -> Result<(), AssetError> {
    match url.scheme() {
        "file" => {
            fs::create_dir_all(url.to_file_folder())
                .await
                .map_err(|err| AssetError::ContentSave(format!("Failed to create folder: {:?}", err)))?;
            fs::write(&url.to_file_path(), data)
                .await
                .map_err(|err| AssetError::ContentSave(format!("Save failed: {:?}", err)))?;
            Ok(())
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

pub async fn upload_string(url: &Url, data: &str) -> Result<(), AssetError> {
    match url.scheme() {
        "file" => {
            fs::create_dir_all(url.to_file_folder())
                .await
                .map_err(|err| AssetError::ContentSave(format!("Failed to create folder: {:?}", err)))?;
            fs::write(&url.to_file_path(), data)
                .await
                .map_err(|err| AssetError::ContentSave(format!("Save failed: {:?}", err)))?;
            Ok(())
        }
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}
