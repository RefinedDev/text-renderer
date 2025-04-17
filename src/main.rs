mod font_reader;
mod font_table_parser;

use core::f32;

use font_reader::FontReader;
use font_table_parser::{FontTableParser, Glyph};

use bevy::{
    color::palettes::css::WHITE,
    input::mouse::AccumulatedMouseScroll,
    prelude::*,
};

#[derive(Resource)]
struct GlyphData(Vec<Glyph>);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_window, spawn).chain())
        .add_systems(Update, (render_text, zoom_cam, go_to_cursor).chain())
        .insert_resource(ClearColor(Color::BLACK))
        .run();

    Ok(())
}

const CURVE_RES: usize = 10;
fn quadratic_curve(a: Vec2, b: Vec2, c: Vec2, alpha: f32) -> Vec2 {
    let p0 = a.lerp(b, alpha);
    let p1 = b.lerp(c, alpha);
    p0.lerp(p1, alpha)
}

fn draw_bezier(a: Vec2, b: Vec2, c: Vec2, gizmos: &mut Gizmos) {
    let mut previous_point = a;
    for i in 0..CURVE_RES {
        let alpha = (i+1) as f32/CURVE_RES as f32;
        let next_point = quadratic_curve(a, b, c, alpha);
        gizmos.line_2d(previous_point, next_point, WHITE);
        previous_point = next_point;
    }
}

fn render_text(mut gizmos: Gizmos, glyph_data: Res<GlyphData>) {
    let mut padding = Vec2::new(0.0, 0.0);

    for glyph_index in 0..150 {
        let glyph_coords = &glyph_data.0[glyph_index].coordinates;
        let glyph_contours = &glyph_data.0[glyph_index].contour_end_pts;

        let mut contour_start = 0;
        let mut new_contours: Vec<Vec<Vec2>> = Vec::with_capacity(glyph_contours.len());

        for contour_end in glyph_contours.iter() {
            let old_contour = glyph_coords[contour_start..(*contour_end as usize + 1)].to_vec();

            let mut first_oncurve_offset = 0;
            while first_oncurve_offset < old_contour.len() {
                if old_contour[first_oncurve_offset].1 {
                    break;
                }
                first_oncurve_offset += 1;
            }

            let mut contour_with_implied_points: Vec<Vec2> = Vec::with_capacity(old_contour.len());

            let mut i = 0;
            while i < old_contour.len() {
                let a = old_contour[i+first_oncurve_offset];
                let b = old_contour[(i+first_oncurve_offset+1)%old_contour.len()];

                contour_with_implied_points.push(a.0);
                if a.1 == b.1 {
                    contour_with_implied_points.push(a.0.midpoint(b.0));   
                }
                
                i += 1;
            }

            contour_start = *contour_end as usize + 1;
            new_contours.push(contour_with_implied_points);
        }
        
        for contour in new_contours.iter() {
            let mut i = 0;
            while i < contour.len() {
                let a = contour[i];
                let b = contour[(i+1)%contour.len()];
                let c = contour[(i+2)%contour.len()];
                draw_bezier(a+padding, b+padding, c+padding, &mut gizmos);
                i+=2;
            }
        }

        padding.x += 100.0;
        if (glyph_index + 1) % 10 == 0 {
            padding.x = 0.0;
            padding.y -= 100.0;
        }
    }
}

fn spawn(window: Single<&Window>, mut commands: Commands) {
    commands.spawn(Camera2d);

    let reader = FontReader::new("JetBrainsMono-Bold.ttf").unwrap();
    let mut table_parser = FontTableParser {
        reader,
        ..default()
    };
    table_parser.get_lookup_table().unwrap();
    table_parser.get_glyph_location().unwrap();
    table_parser.get_glyphs(window.size()).unwrap();
    commands.insert_resource(GlyphData(table_parser.glyphs));
}

fn zoom_cam(
    mut camera: Single<&mut OrthographicProjection, With<Camera>>,
    mouse_wheel: Res<AccumulatedMouseScroll>,
) {
    let delta_zoom = -mouse_wheel.delta.y * 0.2;
    let multiplicative_zoom = 1. + delta_zoom;
    camera.scale = (camera.scale * multiplicative_zoom).clamp(f32::MIN, 1.0);
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

fn setup_window(mut window: Single<&mut Window>) {
    window.title = String::from("Text Rendering");
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}
