use std::sync::Mutex;

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct Frame {
    frame: Option<FrameOutput>,
    //start: Instant,
    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            frame: None,
            buffers: Mutex::new(Vec::new()),
        }
    }

    pub fn start(&mut self, frame_output: FrameOutput) {
        self.frame = Some(frame_output);
    }

    pub fn end(&mut self, queue: &wgpu::Queue) {
        {
            let mut buffers = self.buffers.lock().unwrap();
            queue.submit(buffers.drain(..));
        }
        self.frame = None;
    }

    pub fn output(&self) -> &FrameOutput {
        self.frame.as_ref().unwrap()
    }

    pub fn descriptor(&self) -> &wgpu::SwapChainDescriptor {
        &self.output().descriptor
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.output().frame.view
    }

    pub fn add_command(&self, commands: wgpu::CommandBuffer) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(commands);
    }
}

impl Default for Frame {
    fn default() -> Frame {
        Frame::new()
    }
}
