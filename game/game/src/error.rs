#[derive(Debug)]
pub enum GameError {
    RenderContext(String),
    Shader(String),
}
