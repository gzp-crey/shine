#![cfg(feature = "cook")]
use crate::assets::{
    cooker::CookingError, vertex, AssetError, AssetIO, AssetId, ContentHash, CookedModel, IndexData, MeshData, Url,
    VertexData,
};
use gltf::{accessor::Dimensions, buffer, Document, Gltf, Primitive, Semantic};
use itertools::izip;

pub struct GltfSource {
    pub source_id: AssetId,
    pub source_url: Url,
    pub document: Document,
    pub buffers: Vec<buffer::Data>,
}

impl GltfSource {
    pub async fn load(
        io: &AssetIO,
        source_id: &AssetId,
        source_url: &Url,
    ) -> Result<(GltfSource, ContentHash), AssetError> {
        log::debug!("[{}] Downloading from {} ...", source_id, source_url);
        let data = io.download_binary(&source_url).await?;

        let Gltf { document, blob } = Gltf::from_slice(&data).map_err(|err| AssetError::load_failed(source_id, err))?;
        let buffers = import_buffer_data(source_id, &document, blob)?;

        let gltf = GltfSource {
            source_id: source_id.clone(),
            source_url: source_url.clone(),
            document,
            buffers,
        };
        let source_hash = ContentHash::from_bytes(&data);

        Ok((gltf, source_hash))
    }

    pub async fn cook(self) -> Result<CookedModel, CookingError> {
        log::debug!("[{}] Compiling...", self.source_id);

        let GltfSource {
            source_id,
            document,
            buffers,
            ..
        } = self;

        //log::trace!("[{}] Gltf document: \n{:#?}", source_id, document);

        let mut model = CookedModel::default();
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
                    log::info!("[{}] vertex format: {:?}", source_id, format);
                    match &format {
                        (Vec3, Some(Vec3), None) | (Vec3, Some(Vec4), None) => create_vertex_p3c4(&buffers, &primitive),
                        _ => {
                            log::warn!("[{}] Unsupported vertex format: {:?}", source_id, format);
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
}

///Load data from url
fn load_source(source_id: &AssetId, uri: &str) -> Result<Vec<u8>, AssetError> {
    if let Some(stripped) = uri.strip_prefix("data:") {
        let mut split = stripped.split(";base64,");
        let match0 = split.next();
        let match1 = split.next();
        if let Some(data) = match1 {
            base64::decode(&data).map_err(|err| AssetError::load_failed(source_id, err))
        } else if let Some(data) = match0 {
            base64::decode(&data).map_err(|err| AssetError::load_failed(source_id, err))
        } else {
            Err(AssetError::load_failed_str(source_id, "Unsupported data scheme"))
        }
    } else {
        Err(AssetError::load_failed_str(source_id, "Unsupported external data"))
    }
}

/// Import the buffer data referenced by a gltf document.
fn import_buffer_data(
    source_id: &AssetId,
    document: &Document,
    mut blob: Option<Vec<u8>>,
) -> Result<Vec<buffer::Data>, AssetError> {
    let mut buffers = Vec::new();
    for buffer in document.buffers() {
        let mut data = match buffer.source() {
            buffer::Source::Uri(uri) => load_source(source_id, uri),
            buffer::Source::Bin => blob
                .take()
                .ok_or_else(|| AssetError::load_failed_str(source_id, "Gltf error: missing blob")),
        }?;
        if data.len() < buffer.length() {
            return Err(AssetError::load_failed_str(source_id, "Insufficient buffer length"));
        }
        data.resize(((data.len() + 3) / 4) * 4, 0);
        buffers.push(buffer::Data(data));
    }
    Ok(buffers)
}

/// Create Pos3fCol4f vertex data from a buffer
fn create_vertex_p3c4(buffers: &[buffer::Data], primitive: &Primitive<'_>) -> VertexData {
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
