mod font_reader;
mod font_table_parser;

use font_reader::FontReader;
use font_table_parser::FontTableParser;

use bevy::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = FontReader::new("JetBrainsMono-Bold.ttf")?; 

    let mut table_parser = FontTableParser {
        reader,
        ..default()
    };
    table_parser.get_lookup_table()?;
    table_parser.get_glyph_location()?;
    table_parser.get_glyph_data()?;


    Ok(())
}
