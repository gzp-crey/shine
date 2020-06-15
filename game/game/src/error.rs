#[derive(Debug)]
pub enum GameError {
    Config(String),
    Setup(String),
    Render(String),
}
