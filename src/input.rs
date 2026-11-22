//! Reads keyboard, mouse and gamepad into one set of controls, and into menu input.

use crate::menu::MenuInput;
use crate::settings::Settings;
use raylib::prelude::*;

const DEADZONE: f32 = 0.15;
const MOUSE_SENSITIVITY: f32 = 0.0035;
const STICK_LOOK_SPEED: f32 = 2.6; // radians per second at full tilt
const GAMEPAD: i32 = 0;
/// A mouse move bigger than this in one frame (pixels) is the cursor being grabbed or
/// warped by the window system, not the player: it is ignored so the camera does not jump.
const MOUSE_JUMP: f32 = 300.0;
/// Frames to ignore the mouse after the window gets focus or the first click.
const MOUSE_SETTLE_FRAMES: u32 = 3;

/// Keeps the camera still while the window system grabs the mouse. On some desktops (Wayland,
/// some X11 window managers) the pointer is only locked on the first click or when the window
/// gets focus, and that frame reports a huge mouse move.
pub struct MouseLook {
    settle: u32,
    was_focused: bool,
    clicked: bool,
    /// Whether the game wants the mouse (playing) or a visible cursor (menus).
    grabbed: bool,
}

impl MouseLook {
    pub fn new() -> MouseLook {
        MouseLook {
            settle: MOUSE_SETTLE_FRAMES,
            was_focused: false,
            clicked: false,
            grabbed: false,
        }
    }

    /// Hides and locks the cursor for playing; the next few frames of mouse movement are ignored.
    pub fn grab(&mut self, rl: &mut RaylibHandle) {
        rl.disable_cursor();
        self.grabbed = true;
        self.clicked = false;
        self.settle = MOUSE_SETTLE_FRAMES;
    }

    /// Shows the cursor again, for the menus.
    pub fn release(&mut self, rl: &mut RaylibHandle) {
        rl.enable_cursor();
        self.grabbed = false;
    }

    /// Mouse movement to use this frame (zero while the grab is settling).
    pub fn delta(&mut self, rl: &mut RaylibHandle) -> Vector2 {
        let focused = rl.is_window_focused();
        if focused && !self.was_focused && self.grabbed {
            rl.disable_cursor();
            self.settle = MOUSE_SETTLE_FRAMES;
        }
        self.was_focused = focused;
        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        if click && !self.clicked {
            self.clicked = true;
            self.settle = self.settle.max(MOUSE_SETTLE_FRAMES);
        }
        let md = rl.get_mouse_delta();
        if !focused || self.settle > 0 {
            self.settle = self.settle.saturating_sub(1);
            return Vector2::zero();
        }
        if md.x.abs() > MOUSE_JUMP || md.y.abs() > MOUSE_JUMP {
            return Vector2::zero();
        }
        md
    }
}

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
    /// Start again after the tank is destroyed.
    pub restart: bool,
    /// Turn the sound off or back on.
    pub mute: bool,
    /// Esc or Start: open the pause menu.
    pub pause: bool,
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
    /// `mouse` is this frame's mouse movement, already filtered by `MouseLook`; `settings` scales
    /// how fast the mouse and the stick turn the camera.
    pub fn read(rl: &RaylibHandle, mouse: Vector2, settings: &Settings, dt: f32) -> Controls {
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
        let md = mouse;
        let ms = MOUSE_SENSITIVITY * settings.mouse_sensitivity;
        c.look_yaw -= md.x * ms;
        c.look_pitch -= md.y * ms;
        c.fire |= rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT)
            || rl.is_key_down(KeyboardKey::KEY_SPACE);

        // Start again: R, Enter or a click (the world only takes it a moment after dying).
        c.restart |= rl.is_key_pressed(KeyboardKey::KEY_R)
            || rl.is_key_pressed(KeyboardKey::KEY_ENTER)
            || rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        c.mute |= rl.is_key_pressed(KeyboardKey::KEY_M);
        c.pause |=
            rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) || rl.is_key_pressed(KeyboardKey::KEY_P);

        // Gamepad: left stick drives, right stick aims, right trigger (or RB) fires.
        if rl.is_gamepad_available(GAMEPAD) {
            c.gamepad_connected = true;
            // A (bottom face button) or Start restart after dying; Start also pauses.
            c.pause |=
                rl.is_gamepad_button_pressed(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_MIDDLE_RIGHT);
            c.restart |= rl
                .is_gamepad_button_pressed(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN)
                || rl
                    .is_gamepad_button_pressed(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_MIDDLE_RIGHT);
            // Select / Back / View.
            c.mute |=
                rl.is_gamepad_button_pressed(GAMEPAD, GamepadButton::GAMEPAD_BUTTON_MIDDLE_LEFT);
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
            let look = STICK_LOOK_SPEED * settings.stick_sensitivity * dt;
            c.look_yaw -= rx * look;
            c.look_pitch -= ry * look;
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

/// Turns the left stick into single menu steps, like key presses.
#[derive(Default)]
pub struct MenuNav {
    /// Direction the stick was held in last frame: (x, y) each -1, 0 or 1.
    held: (i8, i8),
}

impl MenuNav {
    pub fn read(&mut self, rl: &RaylibHandle) -> MenuInput {
        let key = |k| rl.is_key_pressed(k);
        let mut i = MenuInput {
            up: key(KeyboardKey::KEY_UP) || key(KeyboardKey::KEY_W),
            down: key(KeyboardKey::KEY_DOWN) || key(KeyboardKey::KEY_S),
            left: key(KeyboardKey::KEY_LEFT) || key(KeyboardKey::KEY_A),
            right: key(KeyboardKey::KEY_RIGHT) || key(KeyboardKey::KEY_D),
            select: key(KeyboardKey::KEY_ENTER) || key(KeyboardKey::KEY_SPACE),
            back: key(KeyboardKey::KEY_BACKSPACE),
            pause: key(KeyboardKey::KEY_ESCAPE) || key(KeyboardKey::KEY_P),
            mouse: rl.get_mouse_position(),
            // A huge jump is the cursor being placed or released, not the player moving it.
            mouse_moved: (0.5..MOUSE_JUMP).contains(&rl.get_mouse_delta().length()),
            click: rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT),
        };
        if rl.is_gamepad_available(GAMEPAD) {
            let btn = |b| rl.is_gamepad_button_pressed(GAMEPAD, b);
            i.up |= btn(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP);
            i.down |= btn(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN);
            i.left |= btn(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT);
            i.right |= btn(GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT);
            i.select |= btn(GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN);
            i.back |= btn(GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_RIGHT);
            i.pause |= btn(GamepadButton::GAMEPAD_BUTTON_MIDDLE_RIGHT);
            // Left stick: one step each time it is pushed past halfway.
            let dir = |v: f32| {
                if v > 0.5 {
                    1
                } else if v < -0.5 {
                    -1
                } else {
                    0
                }
            };
            let x = dir(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_LEFT_X));
            let y = dir(rl.get_gamepad_axis_movement(GAMEPAD, GamepadAxis::GAMEPAD_AXIS_LEFT_Y));
            if x != self.held.0 {
                i.left |= x < 0;
                i.right |= x > 0;
            }
            if y != self.held.1 {
                i.up |= y < 0;
                i.down |= y > 0;
            }
            self.held = (x, y);
        }
        i
    }
}
