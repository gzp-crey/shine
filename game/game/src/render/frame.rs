use crate::render::FrameGraph;
use std::sync::Mutex;

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct Frame {
    frame: Option<FrameOutput>,
    //start: Instant,
    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
    graph: Option<FrameGraph>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            frame: None,
            buffers: Mutex::new(Vec::new()),
            graph: None,
        }
    }

    pub fn set_frame_graph(&mut self, graph: Option<FrameGraph>) {
        self.graph = graph;
    }

    pub fn start(&mut self, frame_output: FrameOutput) {
        self.frame = Some(frame_output);
        if let Some(graph) = &mut self.graph {
            graph.update();
            graph.start_frame();
        }
    }

    pub fn end(&mut self, queue: &wgpu::Queue) {
        if let Some(graph) = &mut self.graph {
            graph.end_frame();
        }

        {
            let mut buffers = self.buffers.lock().unwrap();
            queue.submit(buffers.drain(..));
        }
        self.frame = None;
    }

    pub fn output(&self) -> &FrameOutput {
        self.frame.as_ref().unwrap()
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
