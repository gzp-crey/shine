pub struct FrameGraphLoader {
    descriptor: Option<Result<FrameGraphDescriptor, FrameGraphLoadError>>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,
u::CommandBuffer>>,
}

impl FrameGraphLoader {
    pub fn new() -> FrameGraphLoader {
        FrameGraphLoader {
            descriptor: Some(Ok(FrameGraphDescriptor::single_pass())),
            descriptor_loader: None,}
        }

         /*pub fn load_graph(&mut self, assetio: AssetIO, descriptor: String) {
        self.passes = FramePasses::new();
        self.targets.clear_targets();
        //let task = async move { assetio.load_frame_graph(descriptor).await };
        //self.descriptor_loader = Some(AsyncTask::start(task));
        //self.descriptor = None;
    }*/


        /*pub fn set_graph(&mut self, descriptor: Result<FrameGraphDescriptor, FrameGraphLoadError>) {
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
        self.descriptor_loader = None;
        self.descriptor = Some(descriptor);
    }*/

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