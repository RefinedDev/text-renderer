use bevy::{input::mouse::AccumulatedMouseScroll, prelude::*};

use crate::{Frames, frame::Frame, Debug};

pub fn zoom_cam(
    mut camera: Single<&mut OrthographicProjection, With<Camera>>,
    mouse_wheel: Res<AccumulatedMouseScroll>,
) {
    let delta_zoom = -mouse_wheel.delta.y * 0.2;
    let multiplicative_zoom = 1. + delta_zoom;
    camera.scale = (camera.scale * multiplicative_zoom).clamp(0.15, 2.0);
}

pub fn go_to_cursor(
    buttons: Res<ButtonInput<MouseButton>>,
    mut camera: Single<(&mut Transform, &GlobalTransform, &Camera), With<Camera>>,
    window: Single<&Window>,
    time: Res<Time>,
    mut looking_at: Local<Vec3>,
    mut frames: ResMut<Frames>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        let cursor_position = window.cursor_position().expect("could not get cursor pos");
        let wrt_world = camera
            .2
            .viewport_to_world_2d(camera.1, cursor_position)
            .unwrap();
        *looking_at = Vec3::new(wrt_world.x, wrt_world.y, camera.0.translation.z);
    }

    camera
        .0
        .translation
        .smooth_nudge(&looking_at, 15., time.delta_secs());

    let global_t = GlobalTransform::from(*camera.0);
    for frame in frames.0.iter_mut() {
        if !frame.locked {
            continue;
        }
        frame.update(window.size(), &global_t, camera.2); // cant use camera.1 instead of global_t because it isnt updated till PostUpdate
    }
}

fn keycode_to_string(key: &KeyCode) -> String {
    let debug = format!("{:?}", key);
    if let Some(s) = debug.strip_prefix("Key") {
        s.to_string()
    } else if let Some(s) = debug.strip_prefix("Digit") {
        s.to_string()
    } else {
        String::from("skip")
    }
}

pub fn input_stuff(
    mut debug: ResMut<Debug>, 
    mut writing: Local<bool>, 
    keyboard_input: Res<ButtonInput<KeyCode>>,

    mut frames: ResMut<Frames>,
    mut current_frame_name: Local<String>,
    mut frame_index: Local<usize>,
) {    
    if *frame_index == 0 {
        *frame_index = 2; // ignore fps and current_frame_display
        *current_frame_name = frames.0[*frame_index].name.clone();
    } else {
        let c_frame_display = frames.0.get_frame_by_name(String::from("current_frame")).ok_or_else(|| format!("Current Frame Display not found!")).unwrap();
        if c_frame_display.text != *current_frame_name { 
            c_frame_display.text = current_frame_name.clone();
        }
    }
    
    let current_frame = &mut frames.0[*frame_index];
    if keyboard_input.just_pressed(KeyCode::CapsLock) {
        debug.0 = !debug.0;
    } else if keyboard_input.just_pressed(KeyCode::Tab) {
        *writing = !*writing;
    } else if keyboard_input.just_pressed(KeyCode::ArrowRight) {
        *frame_index += 1;
        if *frame_index == frames.0.len() {
            *frame_index = 2; // // ignore fps and current_frame_display
        }
        *current_frame_name = frames.0[*frame_index].name.clone();
    } else if *writing && keyboard_input.just_pressed(KeyCode::Backspace) && !current_frame.text.is_empty() {
        current_frame.text.pop();
    } else if *writing && keyboard_input.just_pressed(KeyCode::Space) {
        current_frame.text.push(' ');
    } else if *writing && keyboard_input.pressed(KeyCode::ShiftRight) { // funny
        current_frame.text.push('E');
    } else if *writing {
        let just_pressed = keyboard_input.get_just_pressed();
        let holding_shift = keyboard_input.pressed(KeyCode::ShiftLeft);
        for key in just_pressed.into_iter() {
            let s = keycode_to_string(key);
            if s == "skip" { continue }
            let cased = if holding_shift {&s} else {&(s.to_lowercase())};
            current_frame.text.push_str(cased);
        }
    }
}