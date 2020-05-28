use crate::{Context, CookingError};
use gltf::{binary, Document, Gltf};
use shine_game::assets::{AssetError, Url};
use std::borrow::Cow;

impl From<gltf::Error> for CookingError {
    fn from(err: gltf::Error) -> CookingError {
        AssetError::Content(format!("Gltf error: {}", err)).into()
    }
}

fn align_to_multiple_of_four(n: usize) -> usize {
    (n + 3) & !3
}

pub fn serialize_gltf(document: Document, mut blob: Option<Vec<u8>>) -> Result<Vec<u8>, CookingError> {
    let json = gltf_json::serialize::to_string(&document.into_json())?;
    let json_offset = align_to_multiple_of_four(json.len());

    blob.as_mut().map(|v| v.resize(align_to_multiple_of_four(v.len()), 0));
    let buffer_length = blob.as_ref().map(|v| v.len()).unwrap_or(0);

    let glb = binary::Glb {
        header: binary::Header {
            magic: b"glTF".clone(),
            version: 2,
            length: (json_offset + buffer_length) as u32,
        },
        bin: blob.map(|v| Cow::from(v)),
        json: Cow::Owned(json.into_bytes()),
    };
    let data = glb.to_vec()?;
    Ok(data)
}

pub async fn cook_gltf(
    context: &Context,
    _source_base: &Url,
    target_base: &Url,
    gltf_url: &Url,
) -> Result<String, CookingError> {
    log::info!("[{}] Cooking...", gltf_url.as_str());

    log::debug!("[{}] Downloading...", gltf_url.as_str());
    let data = context.assetio.download_binary(&gltf_url).await?;
    let Gltf { document, blob } = Gltf::from_slice(&data)?;

    // parse, cook external, referenced resources

    log::debug!("[{}] Creating binary gltf...", gltf_url.as_str());
    let cooked_gltf = serialize_gltf(document, blob)?;

    log::debug!("[{}] Uploading...", gltf_url.as_str());
    let target_id = context
        .assetio
        .upload_cooked_binary(&target_base, "glb", &cooked_gltf)
        .await?;

    Ok(target_id)
}
