use crate::{Guesture, InputState};
use std::mem;

#[cfg(target_arch = "wasm32")]
use wasm_timer::SystemTime;

#[cfg(not(target_arch = "wasm32"))]
use std::time::SystemTime;

pub struct InputManager {
    time: u128,
    guestures: Vec<Box<dyn Guesture>>,
    guestures_order: Vec<usize>,
}

impl InputManager {
    fn now() -> u128 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
    }

    pub fn new() -> InputManager {
        InputManager {
            time: 0,
            guestures: Vec::new(),
            guestures_order: Vec::new(),
        }
    }

    pub fn add_guesture<G: 'static + Guesture>(&mut self, guesture: G) {
        self.guestures.push(Box::new(guesture));
        self.guestures_order.clear();
    }

    /// Prepare for the next input frame. The current state is made the previous state
    /// and the current state is prepared to accept new inputs.
    pub fn advance_states(&mut self, previous: &mut InputState, current: &mut InputState) {
        self.time = Self::now();
        self.process_guestures(previous, current);
        mem::swap(previous, current);
        current.init_from(previous, self.time);
    }

    fn update_guesture_order(&mut self) {
        //todo: topo order by input/output
        if self.guestures_order.is_empty() {
            self.guestures_order = (0..self.guestures.len()).collect();
        }
    }

    /// Perform the guesture handling based on previous and current states
    fn process_guestures(&mut self, previous: &InputState, current: &mut InputState) {
        self.update_guesture_order();
        for i in &self.guestures_order {
            let guesture = &mut self.guestures[*i];
            guesture.on_update(previous, current);
        }
    }
}

impl Default for InputManager {
    fn default() -> InputManager {
        InputManager::new()
    }
}
