use bevy::{
    color::palettes::css::{BLUE, GREEN, RED, WHITE}, 
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, 
    prelude::* 
};

use crate::{
    Debug, 
    FontScaleANDLineHeight,
    Frames,
    GlyphData,
    GlyphSpaces,
    GlyphUnicode,
    frame::Frame
};

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


pub fn render_text(
    mut gizmos: Gizmos,
    window: Single<&Window>, 
    camera: Single<(&GlobalTransform, &Camera), With<Camera>>,

    diagnostics: Res<DiagnosticsStore>,
    
    mut frames: ResMut<Frames>,
    glyph_data: Res<GlyphData>,
    glyph_unicodes: Res<GlyphUnicode>,
    glyph_spaces: Res<GlyphSpaces>,
    fontscale_and_lineheight: Res<FontScaleANDLineHeight>,
    debugging: Res<Debug>,
) {
    let min = Vec2::new(-125.0,-125.0);
    let max = Vec2::new(window.width()*1.4, window.height()*1.4);
    let world_min = camera.1.viewport_to_world_2d(camera.0, min).unwrap();
    let world_max = camera.1.viewport_to_world_2d(camera.0, max).unwrap();
    let (x_min, y_min, x_max, y_max) = (world_min.x, world_max.y, world_max.x, world_min.y); // weird as fuck i know

    if frames.0.is_empty() {
        super::setup_frames(camera, frames, window);
        return;
    }

    for frame in frames.0.iter_mut() {
        frame.show(&mut gizmos);

        let font_scale = fontscale_and_lineheight.0 * frame.frame_scale;
        let line_height = fontscale_and_lineheight.1 * frame.frame_scale;

        let w_size = frame.t_right.x - frame.b_left.x;
        let mut padding = Vec2::new(0.0, line_height);
        let frame_offset = frame.t_left;

        for word in frame.text.split_whitespace() {
            let mut total_width_needed: f32 = 0.0;
    
            for char in word.chars().into_iter() {
                let unicode = char as u32;
                let glyph_index = glyph_unicodes.0[&unicode];
                let glyph_advanced_width = &glyph_spaces.0[glyph_index];
                total_width_needed += *glyph_advanced_width as f32 * font_scale;
            }

            if padding.x + total_width_needed > w_size*0.95 {
                padding.x = 0.0;
                padding.y += line_height;
            }

            for char in word.chars().into_iter() {
                let unicode = char as u32;
                let glyph_index = glyph_unicodes.0[&unicode];
                let contour_coordinates = &glyph_data.0[glyph_index].contour_coordinates;
                let glyph_advanced_width = &glyph_spaces.0[glyph_index];
                let bounding_box = &glyph_data.0[glyph_index].bounding_box; // (x_min, y_min, x_max, y_max)
               
               let bb_x_min = bounding_box[0] * frame.frame_scale + padding.x + frame_offset.x;
               let bb_y_min = bounding_box[1] * frame.frame_scale + padding.y + frame_offset.y;
               let bb_x_max = bounding_box[2] * frame.frame_scale + padding.x + frame_offset.x;
               let bb_y_max = bounding_box[3] * frame.frame_scale + padding.y + frame_offset.y;
                for contour_with_implied_points in contour_coordinates {
                    if 
                        bb_x_min < x_min 
                        // || bb_x_min < frame.b_left.x 
                        || bb_y_min < y_min 
                        || bb_y_min < frame.b_left.y // any glyphs below the frame wont render (im letting the ones who MIGHT overflow (slightly) from the sides render)
                        ||bb_x_max > x_max 
                        // || bb_x_max > frame.t_right.x 
                        || bb_y_max > y_max 
                        // || bb_y_max > frame.t_right.y
                    {
                        break;
                    }

                    let mut i = 0;
                    let length = contour_with_implied_points.len();
                    while i < length {
                        let a = contour_with_implied_points[i];
                        let b = contour_with_implied_points[(i + 1) % length];
                        let c =contour_with_implied_points[(i + 2) % length];
                        let (p1,p2,p3) = 
                        (
                            a.0*frame.frame_scale + padding + frame_offset, 
                            b.0*frame.frame_scale + padding + frame_offset,
                            c.0*frame.frame_scale + padding + frame_offset
                        );
                    
                        draw_curve(p1, p2, p3, &mut gizmos);
                        if debugging.0 {
                            gizmos.circle_2d(p1, 0.5, if a.1==0 { RED } else if a.1==1 { GREEN } else {BLUE});
                            gizmos.circle_2d(p2, 0.5, if b.1==0 { RED } else if b.1==1 { GREEN } else {BLUE});
                            gizmos.circle_2d(p3, 0.5, if c.1==0 { RED } else if c.1==1 { GREEN } else {BLUE});
                        }
                        
                        i += 2;
                    }
            }

                padding.x += glyph_advanced_width * font_scale;
                if padding.x > w_size*0.95 {
                    padding.x = 0.0;
                    padding.y += line_height;
                }
            }
            padding.x += 30.0 * frame.frame_scale; // whitespace for each word
        }
    }

    if let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    {
        let fps_frame = frames.0.get_frame_by_name(String::from("fps")).ok_or_else(|| format!("FPS Frame not found!")).unwrap();
        fps_frame.text = format!("FPS: {:.0}", fps);
    }
}