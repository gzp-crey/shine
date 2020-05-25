#[derive(Debug)]
pub enum GameError {
    Config(String),
    Setup(String),
    RenderContext(String),
    Shader(String),
}
