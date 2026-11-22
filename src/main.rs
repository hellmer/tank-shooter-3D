mod audio;
mod enemy_tank;
mod enemy_tank_layout;
mod input;
mod math;
mod menu;
mod model_layout;
mod models;
mod props_layout;
mod render;
mod settings;
mod tank;
mod turret_layout;
mod world;

use input::Controls;
use menu::{Action, Menu, MenuInput, Screen};
use raylib::prelude::*;
use world::World;

/// Scripted input for `--screenshot`: drive, turn and fire so the image shows the game in motion.
fn scripted_controls(frame: u32) -> Controls {
    Controls {
        throttle: if frame < 80 { 1.0 } else { 0.0 },
        steer: if (30..60).contains(&frame) { 0.6 } else { 0.0 },
        look_yaw: if (20..50).contains(&frame) { 0.01 } else { 0.0 },
        look_pitch: 0.0,
        fire: frame > 70 && frame.is_multiple_of(50),
        restart: false,
        mute: false,
        pause: false,
        gamepad_connected: false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .title("Tank Shooter 3D")
        .msaa_4x()
        .resizable()
        .build();
    rl.set_target_fps(60);

    let mut models = match models::Models::load(&mut rl, &thread) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("Could not load the 3D models, drawing boxes instead: {e}");
            None
        }
    };

    // No sound in screenshot mode, so automated runs stay silent, unless --sound asks for it.
    let mut audio = if screenshot.is_none() || args.iter().any(|a| a == "--sound") {
        audio::Audio::new()
    } else {
        None
    };

    let mut world = World::new(0x5eed);
    // Optional starting camera angles (degrees), handy for screenshots: --yaw 90 --pitch -20
    let arg_f32 = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse::<f32>().ok())
    };
    if let Some(yaw) = arg_f32("--yaw") {
        world.camera.yaw = yaw.to_radians();
    }
    let arg_str = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    // Frame at which --screenshot saves the image.
    let shot_frame = arg_f32("--frames").map(|f| f as u32).unwrap_or(108);
    if let Some(pitch) = arg_f32("--pitch") {
        world.camera.pitch = pitch.to_radians();
    }
    let mut frame: u32 = 0;
    let mut mouse_look = input::MouseLook::new();
    let mut nav = input::MenuNav::default();
    let mut settings = settings::Settings::load();
    if let Some(a) = audio.as_mut() {
        a.set_levels(&settings);
    }
    // Esc opens the pause menu instead of closing the window.
    rl.set_exit_key(None);

    // The game opens on the title screen; screenshots start playing unless --screen says otherwise.
    let start = match (screenshot.is_some(), arg_str("--screen").as_deref()) {
        (_, Some("title")) => Screen::Title,
        (_, Some("pause")) => Screen::Paused,
        (_, Some("options")) => Screen::Options { from_pause: true },
        (true, _) => Screen::Playing,
        (false, _) => Screen::Title,
    };
    let mut menu = Menu::new(start);
    if screenshot.is_none() && menu.screen == Screen::Playing {
        mouse_look.grab(&mut rl);
    }
    // After leaving a menu, the shot button is ignored until it is let go, so the click or
    // button that closed the menu does not also fire.
    let mut hold_fire = false;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(1.0 / 20.0);
        // Fixed step in screenshot mode so the image is the same on every machine.
        let step = if screenshot.is_some() { 1.0 / 60.0 } else { dt };
        let gamepad = rl.is_gamepad_available(0);
        let mut quit = false;

        if menu.screen == Screen::Playing {
            let mut controls = match screenshot {
                Some(_) => scripted_controls(frame),
                None => {
                    let mouse = mouse_look.delta(&mut rl);
                    Controls::read(&rl, mouse, &settings, dt)
                }
            };
            if hold_fire {
                hold_fire = controls.fire;
                controls.fire = false;
                controls.restart = false;
            }
            // Start restarts after dying; otherwise it (and Esc) pauses.
            let restarting = !world.player_alive() && controls.restart;
            if controls.pause && !restarting {
                menu.pause();
                mouse_look.release(&mut rl);
            } else {
                world.update(&controls, step);
            }
            if let Some(a) = audio.as_mut()
                && controls.mute
            {
                a.toggle_mute();
            }
        } else {
            let input = if screenshot.is_some() {
                MenuInput::default()
            } else {
                nav.read(&rl)
            };
            let (sw, sh) = (rl.get_screen_width() as f32, rl.get_screen_height() as f32);
            match menu.update(&input, sw, sh, &mut settings) {
                // The title already shows a fresh world: play in it, from the usual camera angle.
                Action::Play => world.camera.pitch = -0.1,
                Action::Restart => world = world.restarted(),
                Action::ToTitle => world = world.restarted(),
                Action::SettingsChanged => {
                    if let Some(a) = audio.as_mut() {
                        a.set_levels(&settings);
                    }
                }
                Action::SettingsDone => settings.save(),
                Action::Quit => quit = true,
                Action::Resume | Action::None => {}
            }
            if menu.screen == Screen::Playing {
                hold_fire = true;
                if screenshot.is_none() {
                    mouse_look.grab(&mut rl);
                }
            }
            // Behind the title the camera slowly circles the tank.
            // Looking a little up keeps the tank below the menu.
            if menu.screen == Screen::Title {
                world.camera.yaw += 0.12 * dt;
                world.camera.pitch = 0.3;
            }
        }
        if quit {
            break;
        }
        let playing = menu.screen == Screen::Playing;

        let cam = render::camera(&world);
        if let Some(a) = audio.as_mut() {
            a.update(&world, cam.position, cam.target, playing, step);
        }
        let muted = audio.as_ref().is_some_and(|a| a.muted());
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(models::FOG_COLOR);
        // Sky: darker blue overhead fading into the fog color at the horizon.
        let (sw, sh) = (d.get_screen_width(), d.get_screen_height());
        d.draw_rectangle_gradient_v(0, 0, sw, sh / 2, models::SKY_TOP, models::FOG_COLOR);
        if let Some(m) = models.as_mut() {
            m.set_view(cam.position);
        }
        {
            let mut d3 = d.begin_mode3D(cam);
            render::draw_world(&mut d3, &world, models.as_mut());
        }
        if menu.screen == Screen::Playing {
            render::draw_hud(&mut d, &world, cam, gamepad, muted);
        }
        menu.draw(&mut d, &settings, gamepad);
        drop(d);

        frame += 1;
        if let Some(path) = &screenshot
            && frame == shot_frame
        {
            rl.take_screenshot(&thread, path);
            break;
        }
    }
}
