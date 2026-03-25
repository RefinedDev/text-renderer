mod font_reader;
mod font_table_parser;
mod frame;
mod input_handle;
mod renderer;

use frame::TextFrame;
use core::f32;
use std::collections::HashMap;

use font_reader::FontReader;
use font_table_parser::{FontData, Glyph};
use renderer::render_text;

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, 
    prelude::*
};

use crate::input_handle::{go_to_cursor, zoom_cam, input_stuff};

#[derive(Resource)]
struct GlyphData(Vec<Glyph>);

#[derive(Resource)]
struct GlyphUnicode(HashMap<u32, usize>);

#[derive(Resource)]
struct GlyphSpaces(Vec<f32>);

#[derive(Resource)]
struct FontScaleANDLineHeight(f32, f32);

#[derive(Resource)]
struct Frames(Vec<TextFrame>);

#[derive(Resource)]
struct Debug(bool); // RED MEANS ONCURVE; GREEN MEANS OFFCURVE; BLUE MEANS IMPLIED POINT

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, (setup_window, load_assets).chain())
        .add_systems(Update, (go_to_cursor, zoom_cam, render_text, input_stuff).chain())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Debug(false))
        .insert_resource(Frames(Vec::new()))
        .run();

    Ok(())
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

            /*
            found this bug with JETBRAINS MONO at the moment
            BUG: some glyphs like Ŀ ŀ, their dot contour is just a single point, not a set of points so gotta hardcode a dot i guess
            well thats what i hope so and its not a parsing issue.

            therefore im usize:min(ing) contour_end for now because it leads to index error
            */
            let contour_end = usize::min(*contour_end as usize + 1, glyph_coords.len()); 
            
            let old_contour = &glyph_coords[contour_start..(contour_end)];
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
            contour_start = contour_end;
        }
        glyph.coordinates = Vec::new();
        glyph.contour_end_pts = Vec::new(); 
        // they are dead weight
    }
}

fn load_assets(mut commands: Commands) {
    commands.spawn(Camera2d);

    let reader = FontReader::new("fonts/using.ttf").unwrap();
    let mut font_data_parser = FontData {
        reader,
        ..default()
    };

    font_data_parser.get_lookup_table().unwrap();
    font_data_parser.get_glyph_location().unwrap();
    font_data_parser.get_glyphs().unwrap();
    font_data_parser.map_glyph_to_unicode().unwrap();
    font_data_parser.get_glyph_spacings().unwrap();
    setup_implied_points(&mut font_data_parser.glyphs);
    
    commands.insert_resource(FontScaleANDLineHeight(font_data_parser.font_scale, font_data_parser.line_height));
    commands.insert_resource(GlyphData(font_data_parser.glyphs));
    commands.insert_resource(GlyphUnicode(font_data_parser.unicodes_to_index));
    commands.insert_resource(GlyphSpaces(font_data_parser.glyph_spaces));
}

fn setup_frames(
    camera: Single<(&GlobalTransform, &Camera), With<Camera>>, 
    mut frames: ResMut<Frames>,
    window: Single<&Window>,
) {
    let screen_dimensions = window.size();
    let fps_display = TextFrame::new("fps".to_string(),"FPS: 212".to_string(), Vec2::new(0.1,0.05), Vec2::new(0.05,0.025), true, Some(0.3)).setup_bounds(screen_dimensions, &camera);
    let current_frame_display = TextFrame::new("current_frame".to_string(),"big chungus is big hot".to_string(), Vec2::new(0.2,0.05), Vec2::new(0.9,0.025), true, Some(0.3)).setup_bounds(screen_dimensions, &camera);
    let screen = TextFrame::new("screen".to_string(),"The naïve Noël café-owner’s façade was façade-ish; he créped his crêpes with brio while his learnèd, résumé-wielding pâtissier, Zoë, façaded a façade in the Hôtel de Ville.".to_string(), Vec2::new(1.0,0.9), Vec2::new(0.5,0.5), true, Some(0.5)).setup_bounds(screen_dimensions, &camera);
    let m = TextFrame::new("m".to_string(),"big money".to_string(), Vec2::new(0.6,0.3), Vec2::new(0.6,0.8), false, None).setup_bounds(screen_dimensions, &camera);
    frames.0.push(fps_display);
    frames.0.push(current_frame_display);
    frames.0.push(screen);
    frames.0.push(m)
}

fn setup_window(mut window: Single<&mut Window>) {
    window.title = String::from("Text Rendering");
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}

// fn on_window_resize(
//     mut resize_events: EventReader<bevy::window::WindowResized>,
//     mut frames: ResMut<Frames>,
//     camera: Single<(&GlobalTransform, &Camera), With<Camera>>,
//     window: Single<&Window>,
// ) {
//     if resize_events.read().len() != 0 {
//         for frame in frames.0.iter_mut() {
//             frame.update(window.size(), camera.0, camera.1);
//         }
//     }
// }