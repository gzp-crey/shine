use crate::input::{Guesture, InputId, InputState, InputValue};

/// Convert two button into an axis movement.
pub struct ButtonAxis {
    pos_axis: InputId,
    neg_axis: InputId,
    out: InputId,
}

impl ButtonAxis {
    pub fn new(pos_axis: InputId, neg_axis: InputId, out: InputId) -> ButtonAxis {
        ButtonAxis {
            pos_axis,
            neg_axis,
            out,
        }
    }
}

impl Guesture for ButtonAxis {
    fn inputs(&self) -> Vec<InputId> {
        vec![self.pos_axis, self.neg_axis]
    }

    fn outputs(&self) -> Vec<InputId> {
        vec![self.out]
    }

    fn on_update(&mut self, _prev_state: &InputState, state: &mut InputState) {
        let is_pos = state.get_input(self.pos_axis).as_button().unwrap();
        let is_neg = state.get_input(self.neg_axis).as_button().unwrap();

        if is_pos && !is_neg {
            state.set_input(self.out, InputValue::D1(1.), true);
        } else if !is_pos && is_neg {
            state.set_input(self.out, InputValue::D1(-1.), true);
        }
    }
}
