use crate::input::{CurrentInputState, GameInput, InputEvent};
use shine_input::{guestures, GuestureManager, InputId, InputIdGenerator, InputState, InputValue};
use std::any::Any;

pub struct FirstPersonShooter {
    move_pos_x: InputId,
    move_neg_x: InputId,
    move_x: InputId,
    move_x_scale: f32,

    move_pos_y: InputId,
    move_neg_y: InputId,
    move_y: InputId,
    move_y_scale: f32,

    move_pos_z: InputId,
    move_neg_z: InputId,
    move_z: InputId,
    move_z_scale: f32,

    roll_pos: InputId,
    roll_neg: InputId,
    roll: InputId,
    roll_scale: f32,

    pitch_pos: InputId,
    pitch_neg: InputId,
    pitch: InputId,
    pitch_scale: f32,

    yaw_pos: InputId,
    yaw_neg: InputId,
    yaw: InputId,
    yaw_scale: f32,
}

impl FirstPersonShooter {
    pub fn new() -> FirstPersonShooter {
        let mut gen_id = InputIdGenerator::new();
        FirstPersonShooter {
            move_pos_x: gen_id.next(),
            move_neg_x: gen_id.next(),
            move_x: gen_id.next(),
            move_x_scale: 1.,

            move_pos_y: gen_id.next(),
            move_neg_y: gen_id.next(),
            move_y: gen_id.next(),
            move_y_scale: 1.,

            move_pos_z: gen_id.next(),
            move_neg_z: gen_id.next(),
            move_z: gen_id.next(),
            move_z_scale: 1.,

            roll_pos: gen_id.next(),
            roll_neg: gen_id.next(),
            roll: gen_id.next(),
            roll_scale: 1.,

            pitch_pos: gen_id.next(),
            pitch_neg: gen_id.next(),
            pitch: gen_id.next(),
            pitch_scale: 1.,

            yaw_pos: gen_id.next(),
            yaw_neg: gen_id.next(),
            yaw: gen_id.next(),
            yaw_scale: 1.,
        }
    }
}

impl FirstPersonShooter {
    pub fn x(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.move_x).as_offset1().unwrap_or(0.) * self.move_x_scale
    }

    pub fn y(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.move_y).as_offset1().unwrap_or(0.) * self.move_y_scale
    }

    pub fn z(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.move_z).as_offset1().unwrap_or(0.) * self.move_z_scale
    }

    pub fn roll(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.roll).as_offset1().unwrap_or(0.) * self.roll_scale
    }

    pub fn yaw(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.yaw).as_offset1().unwrap_or(0.) * self.yaw_scale
    }

    pub fn pitch(&self, state: &CurrentInputState) -> f32 {
        state.get_input(self.pitch).as_offset1().unwrap_or(0.) * self.pitch_scale
    }
}

impl GameInput for FirstPersonShooter {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn init_guestures(&self, guestures: &mut GuestureManager) {
        guestures.add_guesture(guestures::ButtonAxis::new(
            self.move_pos_x,
            self.move_neg_x,
            self.move_x,
        ));
        guestures.add_guesture(guestures::ButtonAxis::new(
            self.move_pos_y,
            self.move_neg_y,
            self.move_y,
        ));
        guestures.add_guesture(guestures::ButtonAxis::new(
            self.move_pos_z,
            self.move_neg_z,
            self.move_z,
        ));
        guestures.add_guesture(guestures::ButtonAxis::new(self.roll_pos, self.roll_neg, self.roll));
        guestures.add_guesture(guestures::ButtonAxis::new(self.yaw_pos, self.yaw_neg, self.yaw));
        guestures.add_guesture(guestures::ButtonAxis::new(self.pitch_pos, self.pitch_neg, self.pitch));
    }

    fn update_state<'e>(&self, event: InputEvent<'e>, input_state: &mut InputState) {
        match event {
            #[cfg(feature = "native")]
            InputEvent::Winit(input) => {
                use winit::event::ElementState;
                match (input.scancode, input.state) {
                    //W
                    (17, ElementState::Pressed) => input_state.set_input(self.move_pos_z, InputValue::D0, false),
                    (17, ElementState::Released) => input_state.clear_input(self.move_pos_z),
                    //S
                    (31, ElementState::Pressed) => input_state.set_input(self.move_neg_z, InputValue::D0, false),
                    (31, ElementState::Released) => input_state.clear_input(self.move_neg_z),
                    //A
                    (30, ElementState::Pressed) => input_state.set_input(self.move_neg_x, InputValue::D0, false),
                    (30, ElementState::Released) => input_state.clear_input(self.move_neg_x),
                    //D
                    (32, ElementState::Pressed) => input_state.set_input(self.move_pos_x, InputValue::D0, false),
                    (32, ElementState::Released) => input_state.clear_input(self.move_pos_x),
                    //R
                    (19, ElementState::Pressed) => input_state.set_input(self.move_pos_y, InputValue::D0, false),
                    (19, ElementState::Released) => input_state.clear_input(self.move_pos_y),
                    //F
                    (33, ElementState::Pressed) => input_state.set_input(self.move_neg_y, InputValue::D0, false),
                    (33, ElementState::Released) => input_state.clear_input(self.move_neg_y),

                    //Q
                    (16, ElementState::Pressed) => input_state.set_input(self.roll_pos, InputValue::D0, false),
                    (16, ElementState::Released) => input_state.clear_input(self.roll_pos),
                    //E
                    (18, ElementState::Pressed) => input_state.set_input(self.roll_neg, InputValue::D0, false),
                    (18, ElementState::Released) => input_state.clear_input(self.roll_neg),

                    //I
                    (23, ElementState::Pressed) => input_state.set_input(self.pitch_pos, InputValue::D0, false),
                    (23, ElementState::Released) => input_state.clear_input(self.pitch_pos),
                    //K
                    (37, ElementState::Pressed) => input_state.set_input(self.pitch_neg, InputValue::D0, false),
                    (37, ElementState::Released) => input_state.clear_input(self.pitch_neg),
                    //J
                    (36, ElementState::Pressed) => input_state.set_input(self.yaw_pos, InputValue::D0, false),
                    (36, ElementState::Released) => input_state.clear_input(self.yaw_pos),
                    //L
                    (38, ElementState::Pressed) => input_state.set_input(self.yaw_neg, InputValue::D0, false),
                    (38, ElementState::Released) => input_state.clear_input(self.yaw_neg),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
