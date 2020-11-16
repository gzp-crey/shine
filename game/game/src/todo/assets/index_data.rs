pub struct IndexData(Vec<u16>);

impl IndexData {
    pub fn new(data: Vec<u16>) -> IndexData {
        IndexData(data)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get_raw_buffer(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }
}
