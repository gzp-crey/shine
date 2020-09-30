use crate::{InputId, InputState};

pub trait Guesture: Send + Sync {
    fn inputs(&self) -> Vec<InputId>;
    fn outputs(&self) -> Vec<InputId>;
    fn on_update(&mut self, prev_state: &InputState, state: &mut InputState);
}

/// Handle multiple guestures and their dependencies
#[derive(Default)]
pub struct GuestureManager {
    guestures: Vec<Box<dyn Guesture>>,
    guestures_order: Vec<usize>,
}

impl GuestureManager {
    pub fn add_guesture<G: 'static + Guesture>(&mut self, guesture: G) {
        self.guestures.push(Box::new(guesture));
        self.guestures_order.clear();
    }

    fn update_guesture_order(&mut self) {
        //todo: topo order by input/output
        if self.guestures_order.is_empty() {
            self.guestures_order = (0..self.guestures.len()).collect();
        }
    }

    /// Perform the guesture handling based on previous and current states
    pub fn process_guestures(&mut self, previous: &InputState, current: &mut InputState) {
        log::trace!("processing guestures");
        self.update_guesture_order();
        for i in &self.guestures_order {
            let guesture = &mut self.guestures[*i];
            guesture.on_update(previous, current);
        }
    }
}
