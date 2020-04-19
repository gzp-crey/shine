#[derive(Debug)]
pub enum GameError {
    Config(String),
    RenderContext(String),
    Shader(String),
}
