use crate::input::{Guesture, InputMapper, InputState};
use std::mem;

pub struct InputManager {
    time: u128,
    guestures: Vec<Box<dyn Guesture>>,
    guestures_order: Vec<usize>,
    state: InputState,
    previous_state: InputState,
}

impl InputManager {
    fn now() -> u128 {
        use std::time::SystemTime;
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
            state: InputState::new(),
            previous_state: InputState::new(),
        }
    }

    pub fn add_guesture<G: 'static + Guesture>(&mut self, guesture: G) {
        self.guestures.push(Box::new(guesture));
        self.guestures_order.clear();
    }

    pub fn state(&self) -> &InputState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &InputState {
        &mut self.state
    }

    pub fn prepare(&mut self) {
        self.time = Self::now();
        mem::swap(&mut self.previous_state, &mut self.state);
        self.state.prepare(&self.previous_state, self.time);
    }

    fn update_guesture_order(&mut self) {
        //todo: topo order by input/output
        if self.guestures_order.is_empty() {
            self.guestures_order = (0..self.guestures.len()).collect();
        }
    }

    pub fn update(&mut self) {
        self.update_guesture_order();
        for i in &self.guestures_order {
            let guesture = &mut self.guestures[*i];
            guesture.on_update(&self.previous_state, &mut self.state);
        }
    }

    pub fn handle_input<M: InputMapper>(&mut self, mapper: &M, event: &M::InputEvent) {
        mapper.map_event(event, &mut self.state);
    }
}

impl Default for InputManager {
    fn default() -> InputManager {
        InputManager::new()
    }
}
