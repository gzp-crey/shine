use crate::content_hash::upload_cooked_binary;
use gltf::{binary, Document, Gltf};
use shine_game::utils::{assets, url::Url};
use std::borrow::Cow;

fn align_to_multiple_of_four(n: usize) -> usize {
    (n + 3) & !3
}

pub fn serialize_gltf(document: Document, mut blob: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let json = gltf_json::serialize::to_string(&document.into_json())
        .map_err(|err| format!("Failed to serialize gltf: {:?}", err))?;
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
    glb.to_vec()
        .map_err(|err| format!("Failed to serialize gltf: {:?}", err))
}

pub async fn cook_gltf(source_base: &Url, target_base: &Url, gltf_url: &Url) -> Result<String, String> {
    log::trace!("Downloading gltf [{}]", gltf_url.as_str());
    let data = assets::download_binary(&gltf_url)
        .await
        .map_err(|err| format!("Failed to get gltf descriptor [{}]: {:?}", gltf_url.as_str(), err))?;

    let Gltf { document, blob } =
        Gltf::from_slice(&data).map_err(|err| format!("Failed to parse gltf [{}]: {:?}", gltf_url.as_str(), err))?;

    // parse, cook references

    let cooked_gltf = serialize_gltf(document, blob)?;

    let target_id = upload_cooked_binary(&target_base, "glb", &cooked_gltf).await?;
    Ok(target_id)
}
