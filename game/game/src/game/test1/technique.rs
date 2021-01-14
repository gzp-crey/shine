use crate::{game::test1::TestPass, render::FrameTarget};
use shine_ecs::{
    scheduler::{Res, ResMut, Task, TaskGroup},
    ECSError,
};

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

pub fn render(tech: ResMut<Technique>, target: Res<FrameTarget>) -> Result<TaskGroup, ECSError> {
    {
        let mut system = tech.test_pass.system()?;
        system.set_render_state(&target)?;
    }
    Ok(TaskGroup::from(Some(&tech.test_pass)))
}
