#[derive(Clone, Debug)]
pub struct Config {
    pub bind_host: String,
    pub bind_port: u16,
    pub worker_count: usize,
}

impl Config {
    pub fn get_bind_address(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            bind_host: "0.0.0.0".to_string(),
            bind_port: 12345,
            worker_count: 4,
        }
    }
}
