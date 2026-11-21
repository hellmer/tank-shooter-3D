mod input;
mod math;
mod render;
mod tank;
mod world;

use input::Controls;
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
    if screenshot.is_none() {
        rl.disable_cursor();
    }

    let mut world = World::new(0x5eed);
    let mut frame: u32 = 0;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time().min(1.0 / 20.0);
        let controls = match screenshot {
            Some(_) => scripted_controls(frame),
            None => Controls::read(&rl, dt),
        };
        // Fixed step in screenshot mode so the image is the same on every machine.
        let step = if screenshot.is_some() { 1.0 / 60.0 } else { dt };
        world.update(&controls, step);

        let cam = render::camera(&world);
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(120, 140, 160, 255));
        {
            let mut d3 = d.begin_mode3D(cam);
            render::draw_world(&mut d3, &world);
        }
        render::draw_hud(&mut d, &world, cam, controls.gamepad_connected);
        drop(d);

        frame += 1;
        if let Some(path) = &screenshot
            && frame == 108
        {
            rl.take_screenshot(&thread, path);
            break;
        }
    }
}
