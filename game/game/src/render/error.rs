use shine_ecs::core::error::ErrorString;
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Device error: {}", message)]
    Device {
        message: String,
        source: Box<dyn 'static + StdError + Send + Sync>,
    },

    #[error("Render resource compilation failed: {}", message)]
    Compile { message: String },
}

impl RenderError {
    pub fn device_error_str<S: ToString>(err: S) -> Self {
        RenderError::Device {
            message: "Device error".to_owned(),
            source: Box::new(ErrorString(err.to_string())),
        }
    }

    pub fn device_error<S: ToString, E: 'static + StdError + Send + Sync>(message: S, err: E) -> Self {
        RenderError::Device {
            message: message.to_string(),
            source: Box::new(err),
        }
    }
}
