use crate::input::{Guesture, InputId, InputState, InputValue};

/// Trigger when all the input buttons are triggered.
pub struct ButtonCombo {
    inputs: Vec<(InputId, bool)>,
    out: InputId,
}

impl ButtonCombo {
    pub fn new(inputs: Vec<(InputId, bool)>, out: InputId) -> ButtonCombo {
        ButtonCombo { inputs, out }
    }
}

impl Guesture for ButtonCombo {
    fn inputs(&self) -> Vec<InputId> {
        self.inputs.iter().map(|a| a.0).collect()
    }

    fn outputs(&self) -> Vec<InputId> {
        vec![self.out]
    }

    fn on_update(&mut self, _prev_state: &InputState, state: &mut InputState) {
        for (i, b) in self.inputs.iter() {
            if state.get_input(*i).as_button().unwrap() != *b {
                return;
            }
        }
        state.set_input(self.out, InputValue::D0, true);
    }
}
