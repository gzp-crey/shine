use crate::InputId;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InputValue {
    Off,
    D0,
    D1(f32),
    D2(f32, f32),
    D3(f32, f32, f32),
}

impl InputValue {
    pub fn as_button(&self) -> Option<bool> {
        match *self {
            InputValue::Off => Some(false),
            InputValue::D0 => Some(true),
            _ => None,
        }
    }

    pub fn as_offset1(&self) -> Option<f32> {
        match *self {
            InputValue::Off => Some(0.),
            InputValue::D1(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_offset2(&self) -> Option<(f32, f32)> {
        match *self {
            InputValue::Off => Some((0., 0.)),
            InputValue::D2(v1, v2) => Some((v1, v2)),
            _ => None,
        }
    }

    pub fn as_offset3(&self) -> Option<(f32, f32, f32)> {
        match *self {
            InputValue::Off => Some((0., 0., 0.)),
            InputValue::D3(v1, v2, v3) => Some((v1, v2, v3)),
            _ => None,
        }
    }

    pub fn as_position1(&self) -> Option<f32> {
        match *self {
            InputValue::D1(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_position2(&self) -> Option<(f32, f32)> {
        match *self {
            InputValue::D2(v1, v2) => Some((v1, v2)),
            _ => None,
        }
    }

    pub fn as_position3(&self) -> Option<(f32, f32, f32)> {
        match *self {
            InputValue::D3(v1, v2, v3) => Some((v1, v2, v3)),
            _ => None,
        }
    }
}

impl Default for InputValue {
    fn default() -> Self {
        InputValue::Off
    }
}

/// State of a button
#[derive(Clone, Default, Debug)]
struct InputData {
    auto_reset: bool,
    value: InputValue,
}

/// Store the current input state
pub struct InputState {
    time: u128,                           // The last update time
    cursore_position: Option<(f32, f32)>, // Cursore poisition on the normalize [0,1]^2 screen
    inputs: HashMap<InputId, InputData>,  // State of the input
}

impl InputState {
    pub fn new() -> InputState {
        InputState {
            time: 0,
            cursore_position: None,
            inputs: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.inputs.clear();
        self.time = 0;
        self.cursore_position = None;
    }

    /// Copy the previous state
    pub fn init_from(&mut self, prev: &InputState, time: u128) {
        self.clear();

        self.time = time;
        self.cursore_position = prev.cursore_position;

        // copy input from previous state
        self.inputs.clear();
        for (k, j) in prev.inputs.iter() {
            if j.auto_reset {
                continue;
            }
            self.inputs.insert(*k, j.clone());
        }
    }

    pub fn get_time(&self) -> u128 {
        self.time
    }

    pub fn set_cursore_position(&mut self, pos: Option<(f32, f32)>) {
        self.cursore_position = pos
    }

    pub fn get_cursore_position(&self) -> Option<(f32, f32)> {
        self.cursore_position
    }

    pub fn set_input(&mut self, id: InputId, value: InputValue, auto_reset: bool) {
        if value == InputValue::Off {
            self.clear_input(id);
        } else {
            let entry = self.inputs.entry(id);
            #[cfg(debug_assertions)]
            {
                use std::collections::hash_map::Entry;
                let changed = match entry {
                    Entry::Occupied(ref e) if e.get().value != value => true,
                    Entry::Vacant(_) => true,
                    _ => false,
                };
                if changed {
                    log::trace!("set input {:?} to {:?}; autoreset: {}", id, value, auto_reset);
                }
            }

            let mut state = entry.or_insert_with(InputData::default);
            state.value = value;
            state.auto_reset = auto_reset;
        }
    }

    pub fn clear_input(&mut self, id: InputId) {
        #[cfg(debug_assertions)]
        {
            if self.inputs.contains_key(&id) {
                log::trace!("remove input {:?}", id);
            }
        }

        let _ = self.inputs.remove(&id);
    }

    pub fn get_input(&self, id: InputId) -> InputValue {
        self.inputs.get(&id).map(|a| a.value).unwrap_or(InputValue::Off)
    }
}

impl Default for InputState {
    fn default() -> Self {
        InputState::new()
    }
}
