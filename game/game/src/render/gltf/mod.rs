use crate::render::{vertex, IndexData, MeshData, ModelData, VertexData};
use crate::utils::assets::{self, AssetError};
use crate::utils::url::Url;
use base64;
use gltf::{accessor::Dimensions, buffer, Document, Gltf, Primitive, Semantic};
use itertools::izip;

///Load data from url
pub fn load_source(uri: &str) -> Result<Vec<u8>, AssetError> {
    if uri.starts_with("data:") {
        let match0 = &uri["data:".len()..].split(";base64,").nth(0);
        let match1 = &uri["data:".len()..].split(";base64,").nth(1);
        if let Some(data) = match1 {
            base64::decode(&data).map_err(|err| AssetError::ContentLoad(format!("Embedded data error: {:?}", err)))
        } else if let Some(data) = match0 {
            base64::decode(&data).map_err(|err| AssetError::ContentLoad(format!("Embedded data error: {:?}", err)))
        } else {
            Err(AssetError::ContentLoad("Unsupported embedded data scheme".to_owned()))
        }
    } else {
        Err(AssetError::ContentLoad("Unsupported external data".to_owned()))
    }
}

/// Import the buffer data referenced by a gltf document.
pub fn import_buffer_data(document: &Document, mut blob: Option<Vec<u8>>) -> Result<Vec<buffer::Data>, AssetError> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            buffer::Source::Uri(uri) => load_source(uri),
            buffer::Source::Bin => blob
                .take()
                .ok_or_else(|| AssetError::ContentLoad("Gltf error: missing blob".to_owned())),
        }?;
        if data.len() < buffer.length() {
            return Err(AssetError::ContentLoad("Insufficient buffer length".to_owned()));
        }
        data.resize(((data.len() + 3) / 4) * 4, 0);
        buffers.push(buffer::Data(data));
    }
    Ok(buffers)
}

pub fn create_vertex_p3c4(buffers: &Vec<buffer::Data>, primitive: &Primitive<'_>) -> VertexData {
    let vertex_count = primitive.get(&Semantic::Positions).map(|a| a.count()).unwrap();
    let mut vertices: Vec<vertex::Pos3fCol4f> = Vec::with_capacity(vertex_count);

    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions = reader.read_positions().unwrap();
    let colors = reader.read_colors(0).unwrap().into_rgba_f32();

    for (position, color) in izip!(positions, colors) {
        vertices.push(vertex::Pos3fCol4f { position, color })
    }

    VertexData::from_vec(vertices)
}

pub async fn load_from_url(url: &Url) -> Result<ModelData, AssetError> {
    let data = assets::download_binary(&url).await?;
    let Gltf { document, blob } =
        Gltf::from_slice(&data).map_err(|err| AssetError::ContentLoad(format!("Failed to parse gltf: {:?}", err)))?;

    let buffers = import_buffer_data(&document, blob)?;

    let mut model = ModelData::new();
    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let vertex_data = {
                use Dimensions::*;
                let positions = match primitive.get(&Semantic::Positions).map(|a| a.dimensions()) {
                    None => {
                        log::warn!("Skipping primitive, no position information");
                        continue;
                    }
                    Some(dim) => dim,
                };
                let colors_0 = primitive.get(&Semantic::Colors(0)).map(|a| a.dimensions());
                let colors_1 = primitive.get(&Semantic::Colors(1)).map(|a| a.dimensions());
                let format = (positions, colors_0, colors_1);
                log::info!("vertex format: {:?}", format);
                match &format {
                    (Vec3, Some(Vec3), None) | (Vec3, Some(Vec4), None) => create_vertex_p3c4(&buffers, &primitive),
                    _ => {
                        log::warn!("Unsupported vertex format: {:?}", format);
                        continue;
                    }
                }
            };

            let index_data = {
                use gltf::mesh::util::ReadIndices;
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(indices) = reader.read_indices() {
                    match indices {
                        ReadIndices::U8(iter) => Some(IndexData::new(iter.map(|i| i as u16).collect())),
                        ReadIndices::U16(iter) => Some(IndexData::new(iter.collect())),
                        ReadIndices::U32(iter) => Some(IndexData::new(iter.map(|i| i as u16).collect())),
                    }
                } else {
                    None
                }
            };

            let mesh = if let Some(index_data) = index_data {
                MeshData::with_vertices_and_indices(vertex_data, index_data)
            } else {
                MeshData::with_vertices(vertex_data)
            };
            model.meshes.push(mesh);
        }
    }
    Ok(model)
}
