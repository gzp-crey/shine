#![cfg(feature = "wasm")]

use crate::utils::assets::AssetError;
use crate::utils::url::Url;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

pub async fn get_response(url: &Url) -> Result<Response, AssetError> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url.as_str(), &opts)
        .map_err(|err| AssetError::AssetProvider(format!("Failed to download {}: {:?}", url.as_str(), err)))?;

    let window = web_sys::window().unwrap();
    let resp = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|err| AssetError::AssetProvider(format!("Failed to download {}: {:?}", url.as_str(), err)))?
        .dyn_into::<Response>()
        .unwrap();

    if !resp.ok() {
        let err = match resp.text() {
            Ok(promise) => JsFuture::from(promise).await.unwrap().as_string().unwrap(),
            Err(_) => "".to_owned(),
        };
        Err(AssetError::AssetProvider(format!(
            "Unexpected status code ({}) for {}: {}",
            resp.status(),
            url.as_str(),
            err
        )))
    } else {
        Ok(resp)
    }
}

pub async fn download_string(url: &Url) -> Result<String, AssetError> {
    match url.scheme() {
        "http" | "https" => match get_response(url).await?.text() {
            Ok(promise) => dbg!(Ok(JsFuture::from(promise).await.unwrap().as_string().unwrap())),
            Err(err) => Err(AssetError::ContentLoad(format!("Failed to parse response: {:?}", err))),
        },
        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

pub async fn download_binary(url: &Url) -> Result<Vec<u8>, AssetError> {
    match url.scheme() {
        "http" | "https" => match get_response(url).await?.array_buffer() {
            Ok(promise) => {
                let array_buffer = JsFuture::from(promise).await.unwrap();
                Ok(Uint8Array::new_with_byte_offset(&array_buffer, 0).to_vec())
            }
            Err(err) => Err(AssetError::ContentLoad(format!(
                "Failed to parse response for {}: {:?}",
                url.as_str(),
                err
            ))),
        },

        sch => Err(AssetError::UnsupportedScheme(sch.to_owned())),
    }
}

pub async fn upload_binary(url: &Url, data: &[u8]) -> Result<(), AssetError> {
    unimplemented!()
}

pub async fn upload_string(url: &Url, data: &str) -> Result<(), AssetError> {
    unimplemented!()
}
