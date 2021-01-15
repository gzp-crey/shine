use crate::{game::test1::TestPass, render::FrameTarget};
use shine_ecs::{
    scheduler::{Res, ResMut, Task, TaskGroup},
    ECSError,
};
use std::sync::Arc;

pub struct Technique {
    test_pass: Arc<Task<TestPass>>,
}

impl Technique {
    pub fn new(pipeline: String) -> Technique {
        Technique {
            test_pass: Task::new(TestPass::new(pipeline)),
        }
    }
}

pub fn render(tech: ResMut<Technique>, target: Res<FrameTarget>) -> Result<TaskGroup, ECSError> {
    tech.test_pass.system()?.set_render_state(&target);
    Ok(TaskGroup::from_task(tech.test_pass.clone()))
}
