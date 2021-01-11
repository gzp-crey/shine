use crate::{game::test1::TestPass, render::FrameTarget};
use shine_ecs::scheduler::{ResMut, Task};

pub struct Technique {
    test_pass: Task,
}

impl Technique {
    pub fn new(pipeline: String) -> Technique {
        Technique {
            test_pass: Task::new(TestPass::new(pipeline)),
        }
    }
}

pub fn render(tech: ResMut<Technique>, target: ResMut<FrameTarget>) {
    unimplemented!()
}
