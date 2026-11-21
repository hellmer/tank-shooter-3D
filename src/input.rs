//! Reads keyboard, mouse and gamepad into one set of controls.

use raylib::prelude::*;

const DEADZONE: f32 = 0.15;
const MOUSE_SENSITIVITY: f32 = 0.0035;
const STICK_LOOK_SPEED: f32 = 2.6; // radians per second at full tilt
const GAMEPAD: i32 = 0;

/// What the player wants to do this frame, whatever device it came from.
#[derive(Default, Clone, Copy, Debug)]
pub struct Controls {
    /// -1 (reverse) .. 1 (forward).
    pub throttle: f32,
    /// -1 (turn right) .. 1 (turn left).
    pub steer: f32,
    /// Camera yaw change this frame, in radians (positive turns left).
    pub look_yaw: f32,
    /// Camera pitch change this frame, in radians (positive looks up).
    pub look_pitch: f32,
    pub fire: bool,
    pub gamepad_connected: bool,
}

fn deadzone(v: f32) -> f32 {
    if v.abs() < DEADZONE {
        0.0
    } else {
        (v - DEADZONE * v.signum()) / (1.0 - DEADZONE)
    }
}

impl Controls {
    pub fn read(rl: &RaylibHandle, dt: f32) -> Controls {
        let mut c = Controls::default();

        // Keyboard and mouse.
        if rl.is_key_down(KeyboardKey::KEY_W) || rl.is_key_down(KeyboardKey::KEY_UP) {
            c.throttle += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) || rl.is_key_down(KeyboardKey::KEY_DOWN) {
            c.throttle -= 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) || rl.is_key_down(KeyboardKey::KEY_LEFT) {
            c.steer += 1.0;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) || rl.is_key_down(KeyboardKey::KEY_RIGHT) {
            c.steer -= 1.0;
        }
        let md = rl.get_mouse_delta();
        c.look_yaw -= md.x * MOUSE_SENSITIVITY;
        c.look_pitch -= md.y * MOUSE_SENSITIVITY;
        c.fire |= rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
            || rl.is_key_down(KeyboardKey::KEY_SPACE);

        // Gamepad: left stick drives, right stick aims, right trigger (or RB) fires.
        if rl.is_gamepad_available(GAMEPAD) {
            c.gamepad_connected = true;
            let lx =
                deadzone(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_LEFT_X));
            let ly =
                deadzone(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_LEFT_Y));
            let rx =
                deadzone(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_RIGHT_X));
            let ry =
                deadzone(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_RIGHT_Y));
            let rt = rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_RIGHT_TRIGGER);
            c.throttle -= ly;
            c.steer -= lx;
            c.look_yaw -= rx * STICK_LOOK_SPEED * dt;
            c.look_pitch -= ry * STICK_LOOK_SPEED * dt;
            // Triggers rest at -1 on most pads and reach +1 when fully pressed.
            c.fire |= rt > 0.3
                || rl
                    .is_gamepad_button_down(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_2)
                || rl
                    .is_gamepad_button_down(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_1);
        }

        c.throttle = c.throttle.clamp(-1.0, 1.0);
        c.steer = c.steer.clamp(-1.0, 1.0);
        c
    }
}
