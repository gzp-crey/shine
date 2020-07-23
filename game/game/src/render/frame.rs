use crate::render::FrameGraph;
use std::sync::Mutex;

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct Frame {
    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
    graph: FrameGraph,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            buffers: Mutex::new(Vec::new()),
            graph: FrameGraph::default(),
        }
    }

    pub fn set_frame_graph(&mut self, graph: FrameGraph) {
        self.graph = graph;
    }

    pub fn start(&mut self, frame_output: FrameOutput) {
        self.graph.start_frame(Some(frame_output));
    }

    pub fn end(&mut self, queue: &wgpu::Queue) {
        self.graph.end_frame();
        {
            let mut buffers = self.buffers.lock().unwrap();
            queue.submit(buffers.drain(..));
        }
    }

    pub fn output(&self) -> &FrameOutput {
        self.graph.frame_output().unwrap()
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
