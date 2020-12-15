use crate::assets::{AssetError, ContentHash, Url};
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

pub struct AssetLowIO {}

impl AssetLowIO {
    pub fn new() -> Result<AssetLowIO, AssetError> {
        Ok(AssetLowIO {})
    }

    fn create_request(method: &str, url: &Url) -> Result<Request, AssetError> {
        let mut opts = RequestInit::new();
        opts.method(method);
        opts.mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(url.as_str(), &opts)
            .map_err(|err| AssetError::source_error(url.as_str(), err))?;
        Ok(request)
    }

    async fn wait_response(request: Request) -> Result<Response, AssetError> {
        let window = web_sys::window().unwrap();
        let resp = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|err| AssetError::source_error(url.as_str(), err))?
            .dyn_into::<Response>()
            .unwrap();

        if !resp.ok() {
            let err = match resp.text() {
                Ok(promise) => JsFuture::from(promise).await.unwrap().as_string().unwrap(),
                Err(_) => "".to_owned(),
            };
            Err(AssetError::source_error_str(
                url.as_str(),
                format!("Unexpected status code ({}): {}", resp.status(), err),
            ))
        } else {
            Ok(resp)
        }
    }

    async fn get_response_content(url: &Url, response: Response) -> Result<Vec<u8>, AssetError> {
        match response.array_buffer() {
            Ok(promise) => {
                let array_buffer = JsFuture::from(promise).await.unwrap();
                Ok(Uint8Array::new_with_byte_offset(&array_buffer, 0).to_vec())
            }
            Err(err) => Err(AssetError::load_failed(url.as_str(), err)),
        }
    }

    pub async fn download_hash(&self, url: &Url) -> Result<ContentHash, AssetError> {
        log::debug!("Downloading etag from {}", url.as_str());
        unimplemented!()
    }

    pub async fn download_binary(&self, url: &Url) -> Result<Vec<u8>, AssetError> {
        match url.scheme() {
            "http" | "https" => {
                let request = Self::create_request("GET", url)?;
                let resp = Self::wait_response(request).await?;
                Self::get_response_content(url, resp).await
            }
            "blobs" => {
                let url = url.set_scheme("https")?;
                let request = Self::create_request("GET", &url)?;
                let resp = Self::wait_response(request).await?;
                Self::get_response_content(url, resp).await
            }
            sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
        }
    }

    pub async fn upload_binary(&self, url: &Url, data: &[u8]) -> Result<(), AssetError> {
        match url.scheme() {
            "http" | "https" => unimplemented!(),
            "blobs" => {
                let url = url.set_scheme("https")?;
                unimplemented!()
            }
            sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
        }
    }
}
