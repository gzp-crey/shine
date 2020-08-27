use crate::{
    assets::{AssetIO, FrameGraphDescriptor},
    render::FrameGraphLoadError,
};
use shine_ecs::core::async_task::AsyncTask;

/// Manage the frame graph lifecycle for static graphs
pub struct FrameGraphLoader {
    descriptor: Option<Result<FrameGraphDescriptor, FrameGraphLoadError>>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,
    is_activated: bool,
}

impl FrameGraphLoader {
    pub fn new() -> FrameGraphLoader {
        FrameGraphLoader {
            descriptor: None,
            descriptor_loader: None,
            is_activated: false,
        }
    }

    pub fn request_single_pass(&mut self) {
        self.descriptor_loader = None;
        self.descriptor = Some(Ok(FrameGraphDescriptor::single_pass()));
        self.is_activated = false;
    }

    pub fn request_asset(&mut self, assetio: AssetIO, descriptor: String) {
        let task = async move { assetio.load_frame_graph(descriptor).await };
        self.descriptor_loader = Some(AsyncTask::start(task));
        self.descriptor = None;
        self.is_activated = false;
    }

    fn try_get_new_descriptor(&mut self) -> Option<Result<FrameGraphDescriptor, FrameGraphLoadError>> {
        if let Some(loader) = self.descriptor_loader.as_mut() {
            log::error!("Checking loader for frame graph");
            match loader.try_get() {
                Err(_) => {
                    log::warn!("Frame graph load canceled");
                    Some(Err(FrameGraphLoadError::Canceled))
                }
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    Some(Err(err))
                }
                Ok(Some(Ok(descriptor))) => Some(Ok(descriptor)),
                Ok(None) => None,
            }
        } else {
            None
        }
    }
}
