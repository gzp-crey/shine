use crate::assets::{AssetError, ContentHash, Url};
use reqwest::{self, Client, Response};
use tokio::fs;
use tokio::io::AsyncReadExt;

pub struct AssetLowIO {
    pub client: Client,
}

impl AssetLowIO {
    pub fn new() -> Result<AssetLowIO, AssetError> {
        let client = Client::builder()
            .gzip(true)
            .build()
            .map_err(|err| AssetError::other("Failed to initialize reqwest client", err))?;
        Ok(AssetLowIO { client })
    }

    pub async fn check_response(url: &Url, response: Response) -> Result<Response, AssetError> {
        let status = response.status();
        if !status.is_success() {
            let err = response.text().await.unwrap_or_else(|_| "".to_owned());
            Err(AssetError::source_error_str(
                url.as_str(),
                format!("Unexpected status code ({}): {}", status, err),
            ))
        } else {
            Ok(response)
        }
    }

    pub async fn download_hash(&self, url: &Url) -> Result<ContentHash, AssetError> {
        log::debug!("Downloading etag from {}", url.as_str());
        match url.scheme() {
            "file" => {
                let mut data = Vec::new();
                let _ = fs::File::open(&url.to_file_path())
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?
                    .read_to_end(&mut data)
                    .await
                    .map_err(|err| AssetError::load_failed(url.as_str(), err))?;
                Ok(ContentHash::from_bytes(&data))
            }
            "http" | "https" => unimplemented!(),
            "blobs" => unimplemented!(),
            sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
        }
    }

    pub async fn download_binary(&self, url: &Url) -> Result<Vec<u8>, AssetError> {
        log::debug!("Downloading data from {}", url.as_str());
        match url.scheme() {
            "file" => {
                let mut data = Vec::new();
                let _ = fs::File::open(&url.to_file_path())
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?
                    .read_to_end(&mut data)
                    .await
                    .map_err(|err| AssetError::load_failed(url.as_str(), err))?;
                Ok(data)
            }
            "http" | "https" => {
                let response = self
                    .client
                    .get(url.as_str())
                    .send()
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?;
                Self::check_response(url, response)
                    .await?
                    .bytes()
                    .await
                    .map_err(|err| AssetError::load_failed(url.as_str(), err))
                    .map(|d| d.to_vec())
            }
            "blobs" => {
                let translated_url = url.set_scheme("https")?;
                let response = self
                    .client
                    .get(translated_url.as_str())
                    .send()
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?;
                Self::check_response(url, response)
                    .await?
                    .bytes()
                    .await
                    .map_err(|err| AssetError::load_failed(url.as_str(), err))
                    .map(|d| d.to_vec())
            }
            sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
        }
    }

    pub async fn upload_binary(&self, url: &Url, data: &[u8]) -> Result<(), AssetError> {
        log::debug!("Uploading data to {}", url.as_str());
        match url.scheme() {
            "file" => {
                fs::create_dir_all(url.to_file_folder())
                    .await
                    .map_err(|err| AssetError::save_failed(url.as_str(), err))?;
                fs::write(&url.to_file_path(), data)
                    .await
                    .map_err(|err| AssetError::save_failed(url.as_str(), err))?;
                Ok(())
            }
            "http" | "https" => {
                let response = self
                    .client
                    .put(url.as_str())
                    .body(data.to_vec())
                    .send()
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?;
                let _ = Self::check_response(url, response).await?;
                Ok(())
            }
            "blobs" => {
                let translated_url = url.set_scheme("https")?;
                let response = self
                    .client
                    .put(translated_url.as_str())
                    .header("x-ms-blob-type", "BlockBlob")
                    .body(data.to_vec())
                    .send()
                    .await
                    .map_err(|err| AssetError::source_error(url.as_str(), err))?;
                let _ = Self::check_response(url, response).await?;
                Ok(())
            }
            sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
        }
    }
}
