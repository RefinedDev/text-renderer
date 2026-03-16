mod font_reader;
mod font_table_parser;

use core::f32;
use std::collections::HashMap;

use font_reader::FontReader;
use font_table_parser::{FontData, Glyph};

use bevy::{
    color::palettes::css::{BLUE, GREEN, RED, WHITE}, 
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin}, 
    input::mouse::AccumulatedMouseScroll, 
    prelude::*
};

#[derive(Resource)]
struct GlyphData(Vec<Glyph>);

#[derive(Resource)]
struct GlyphUnicode(HashMap<u32, usize>);

#[derive(Resource)]
struct GlyphSpaces(Vec<f32>);

#[derive(Resource)]
struct Debug(bool); // RED MEANS ONCURVE; GREEN MEANS OFFCURVE; BLUE MEANS IMPLIED POINT

#[derive(Resource)]
struct ScreenText(String, bool);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    text_color: Color::linear_rgb(0.0, 255.0, 0.0),
                    enabled: true,
                    ..default()
                },
            },
        ))
        .add_systems(Startup, (setup_window, spawn).chain())
        .add_systems(Update, (render_text, zoom_cam, go_to_cursor, input_stuff).chain())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Debug(false))
        .insert_resource(ScreenText(String::from("Àáâäãå āăą çćč ďđ èéêë ēėę ìíîï īį ñń òóôöõ ø ō ő ùúûü ū ů ýÿ žźż."), false))
        .run();

    Ok(())
}

const CURVE_RES: usize = 3;
fn quadratic_curve(a: Vec2, b: Vec2, c: Vec2, alpha: f32) -> Vec2 {
    let p0 = a.lerp(b, alpha);
    let p1 = b.lerp(c, alpha);
    p0.lerp(p1, alpha)
}

fn draw_curve(a: Vec2, b: Vec2, c: Vec2, gizmos: &mut Gizmos) {
    let mut previous_point = a;
    for i in 0..CURVE_RES {
        let alpha = (i+1) as f32/CURVE_RES as f32;
        let next_point = quadratic_curve(a, b, c, alpha);
        gizmos.line_2d(previous_point, next_point, WHITE);
        previous_point = next_point;
    }
}

fn setup_implied_points(glyph_data: &mut Vec<Glyph>) {
    for glyph in glyph_data.iter_mut() {
        let (glyph_coords, contour_end_points) = (&glyph.coordinates, &glyph.contour_end_pts);
        
        let mut contour_start = 0;
        for contour_end in contour_end_points.iter() {
            /*
                first we loop over the points in the contour and if two consecutive points are oncurve or offcurve, we insert
                an implied offcurve or oncurve point which will help us control the bezier curve
            */
            let old_contour = &glyph_coords[contour_start..(*contour_end as usize + 1)];
            let oc_size = old_contour.len();

            let mut first_oncurve_offset = 0; // sometimes the first point isnt on_curve
            while first_oncurve_offset < oc_size {
                if old_contour[first_oncurve_offset].1 {
                    break;
                }
                first_oncurve_offset += 1;
            }

            let mut contour_with_implied_points: Vec<(Vec2, u8)> = Vec::with_capacity(oc_size);

            let mut i = 0;
            while i < oc_size {
                let a = old_contour[(i + first_oncurve_offset) % oc_size];
                let b = old_contour[(i + first_oncurve_offset + 1) % oc_size];

                contour_with_implied_points.push((a.0, if a.1 {0} else {1})); // 0 MEANS ONCURVE 1 MEANS OFFCURVE
                if a.1 == b.1 {
                    // both points either on or off curve, then we insert a midpoint as a control point for bezier
                    contour_with_implied_points.push((a.0.midpoint(b.0),2)); // 2 means INSERTED POINT
                }
                i += 1;
            }

            glyph.contour_coordinates.push(contour_with_implied_points);
            contour_start = *contour_end as usize + 1;
        }
        glyph.coordinates = Vec::new();
        glyph.contour_end_pts = Vec::new(); 
        // they are dead weight
    }
}

fn render_text(
    mut gizmos: Gizmos,
    window: Single<&Window>,
    glyph_data: Res<GlyphData>,
    glyph_unicodes: Res<GlyphUnicode>,
    glyph_spaces: Res<GlyphSpaces>,
    debugging: Res<Debug>,
    screen_text: Res<ScreenText>
) {
    let mut padding = Vec2::new(0.0, 0.0);
    let w_size = window.size().x;
    let mut font_size: &f32 = &0.0;
    for word in screen_text.0.split_ascii_whitespace().into_iter() {
        let mut total_width_needed: f32 = 0.0;

        for char in word.chars().into_iter() {
            let unicode = char as u32;
            let glyph_index = glyph_unicodes.0[&unicode];
            let glyph_advanced_width = &glyph_spaces.0[glyph_index];
            font_size = &glyph_data.0[glyph_index].font_size;
            total_width_needed += *glyph_advanced_width as f32 * font_size;
        }

        if padding.x + total_width_needed > w_size * 0.9 {
            padding.x = 0.0;
            padding.y -= font_size*2000.0;
        }

        for char in word.chars().into_iter() {
            let unicode = char as u32;
            let glyph_index = glyph_unicodes.0[&unicode];
            let contour_coordinates = &glyph_data.0[glyph_index].contour_coordinates;
            let glyph_advanced_width = &glyph_spaces.0[glyph_index];
       
            for contour_with_implied_points in contour_coordinates {
                let mut i = 0;
                let length = contour_with_implied_points.len();
                while i < length {
                    let a = contour_with_implied_points[i];
                    let b = contour_with_implied_points[(i + 1) % length];
                    let c =contour_with_implied_points[(i + 2) % length];
                    draw_curve(a.0 + padding, b.0 + padding, c.0 + padding, &mut gizmos);
                    if debugging.0 { // this gets really laggy if many letters i could just check if the glyph is in the viewport and then render these but who cares!
                        gizmos.circle_2d(a.0 + padding, 0.5, if a.1==0 { RED } else if a.1==1 { GREEN } else {BLUE});
                        gizmos.circle_2d(b.0 + padding, 0.5, if b.1==0 { RED } else if b.1==1 { GREEN } else {BLUE});
                        gizmos.circle_2d(c.0 + padding, 0.5, if c.1==0 { RED } else if c.1==1 { GREEN } else {BLUE});
                    }
                    i += 2;
                }
           }

            padding.x += *glyph_advanced_width * font_size;
            if padding.x > w_size*0.9 {
                padding.x = 0.0;
                padding.y -= font_size*2000.0;
            }
        }
        padding.x += 30.0; // whitespace
    }
}

fn spawn(window: Single<&Window>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let reader = FontReader::new("using.ttf").unwrap();
    let mut font_data_parser = FontData {
        reader,
        ..default()
    };

    font_data_parser.get_lookup_table().unwrap();
    font_data_parser.get_glyph_location().unwrap();
    font_data_parser.get_glyphs(window.size()).unwrap();
    font_data_parser.map_glyph_to_unicode().unwrap();
    font_data_parser.get_glyph_spacings().unwrap();
    setup_implied_points(&mut font_data_parser.glyphs);
    
    commands.insert_resource(GlyphData(font_data_parser.glyphs));
    commands.insert_resource(GlyphUnicode(font_data_parser.unicodes_to_index));
    commands.insert_resource(GlyphSpaces(font_data_parser.glyph_spaces));
}

fn zoom_cam(
    mut camera: Single<&mut OrthographicProjection, With<Camera>>,
    mouse_wheel: Res<AccumulatedMouseScroll>,
) {
    let delta_zoom = -mouse_wheel.delta.y * 0.2;
    let multiplicative_zoom = 1. + delta_zoom;
    camera.scale = (camera.scale * multiplicative_zoom).clamp(f32::MIN, 2.0);
}

fn go_to_cursor(
    buttons: Res<ButtonInput<MouseButton>>,
    mut camera: Single<(&mut Transform, &GlobalTransform, &Camera), With<Camera>>,
    window: Single<&Window>,
    time: Res<Time>,
    mut looking_at: Local<Vec3>,
) {
    camera
        .0
        .translation
        .smooth_nudge(&looking_at, 15., time.delta_secs());

    if buttons.just_pressed(MouseButton::Left) {
        let cursor_position = window.cursor_position().expect("could not get cursor pos");
        let wrt_world = camera
            .2
            .viewport_to_world_2d(camera.1, cursor_position)
            .unwrap();
        *looking_at = Vec3::new(wrt_world.x, wrt_world.y, camera.0.translation.z);
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

fn input_stuff(mut debug: ResMut<Debug>, mut screentext: ResMut<ScreenText>, keyboard_input: Res<ButtonInput<KeyCode>>) {
    if keyboard_input.just_pressed(KeyCode::CapsLock) {
        debug.0 = !debug.0;
    } else if keyboard_input.just_pressed(KeyCode::Tab) {
        screentext.1 = !screentext.1;
    } else if screentext.1 && keyboard_input.just_pressed(KeyCode::Backspace) && !screentext.0.is_empty() {
        screentext.0.pop();
    } else if screentext.1 && keyboard_input.just_pressed(KeyCode::Space) {
        screentext.0.push(' ');
    } else if screentext.1 {
        let just_pressed = keyboard_input.get_just_pressed();
        let holding_shift = keyboard_input.pressed(KeyCode::ShiftLeft);
        for key in just_pressed.into_iter() {
            let s = keycode_to_string(key);
            if s == "skip" { continue }
            let cased = if holding_shift {&s} else {&(s.to_lowercase())};
            screentext.0.push_str(cased);
        }
    }
}

fn setup_window(mut window: Single<&mut Window>) {
    window.title = String::from("Text Rendering");
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}
