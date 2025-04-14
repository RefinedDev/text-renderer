mod font_reader;
mod font_table_parser;

use font_reader::FontReader;
use font_table_parser::FontTableParser;

use bevy::{
    asset::RenderAssetUsages, color::palettes::css::{RED, WHITE}, prelude::*, render::render_resource::{Extent3d, TextureDimension, TextureFormat}
};

#[derive(Resource)]
struct BlackBoard(Handle<Image>);

#[derive(Resource)]
struct GlyphData(Vec<(Vec<Vec2>, Vec<u16>)>);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_window, spawn).chain())
        .add_systems(Update, render_text)
        .run();

    Ok(())
}

fn render_text(mut gizmos: Gizmos, glyph_data: Res<GlyphData>) {
    let mut padding = Vec2::new(0.0,0.0);

    for glyph_index in 0..50 {
        let glyph_coords = &glyph_data.0[glyph_index].0;
        let glyph_contours = &glyph_data.0[glyph_index].1;
    
        // for glyph_point in glyph_coords.iter() {
        //     gizmos.circle_2d(glyph_point.clone() + padding, 1.0, WHITE);
        // }
        
        let mut starting_point = 0;
        for contour_end_point in glyph_contours.iter() {
            let final_point = *contour_end_point as usize;  
            gizmos.line_2d(glyph_coords[final_point] + padding, glyph_coords[starting_point] + padding, RED); // to complete the loop
            while starting_point < final_point {
                gizmos.line_2d(glyph_coords[starting_point] + padding, glyph_coords[starting_point+1] + padding, RED);
                starting_point += 1;
            }
            starting_point += 1; // final point + 1
        }

        padding.x += 100.0;
        if (glyph_index+1) % 10 == 0 {
            padding.x = 0.0;
            padding.y -= 100.0;
        }
    }
}

fn spawn(window: Single<&Window>, mut commands: Commands, mut images: ResMut<Assets<Image>>)  {
    commands.spawn(Camera2d);
    let image = Image::new_fill(
        Extent3d {
            width: window.size().x as u32,
            height: window.size().y as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &(Color::linear_rgb(0.0, 0.0, 0.0).to_srgba().to_u8_array()),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(image);
    commands.spawn(Sprite::from_image(handle.clone()));
    commands.insert_resource(BlackBoard(handle));

    let reader = FontReader::new("JetBrainsMono-Bold.ttf").unwrap();
    let mut table_parser = FontTableParser {
        reader,
        ..default()
    };
    table_parser.get_lookup_table().unwrap();
    table_parser.get_glyph_location().unwrap();
    table_parser.get_glyph_data(window.size()).unwrap();
    commands.insert_resource(GlyphData(table_parser.glyph_data));

}

fn setup_window(mut window: Single<&mut Window>) {
    window.title = String::from("Stroke Recognizer");
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}
