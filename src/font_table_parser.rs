use bevy::math::Vec2;
use std::collections::HashMap;

use crate::font_reader::FontReader;

// https://developer.apple.com/fonts/TrueType-Reference-Manual/
// https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6glyf.html
fn bit_is_set(flag: u8, bit: u8) -> bool {
    return (flag >> bit) & 1 == 1;
}

fn get_coordinates(
    reader: &mut FontReader,
    flags: &Vec<u8>,
) -> Result<Vec<Vec2>, Box<dyn std::error::Error>> {
    let mut short_vector_bit = 1;
    let mut sign_or_skip_bit = 4;
    let mut coordinates: Vec<Vec2> = vec![Vec2::ZERO; flags.len()];

    // FOR X
    for i in 0..coordinates.capacity() {
        coordinates[i].x = coordinates[i16::max(0, (i as i16) - 1) as usize].x;
        let flag = flags[i];
        let _on_curve = bit_is_set(flag, 0);

        if bit_is_set(flag, short_vector_bit) {
            let coordinate = reader.read_byte()? as f32;
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].x += coordinate * sign;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].x += reader.read_i16()? as f32;
        }
    }

    short_vector_bit = 2;
    sign_or_skip_bit = 5;

    // FOR Y
    for i in 0..coordinates.capacity() {
        coordinates[i].y = coordinates[i16::max(0, (i as i16) - 1) as usize].y;
        let flag = flags[i];
        let _on_curve = bit_is_set(flag, 0);

        if bit_is_set(flag, short_vector_bit) {
            let coordinate = reader.read_byte()? as f32;
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].y += coordinate * sign;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].y += reader.read_i16()? as f32;
        }
    }

    Ok(coordinates)
}

#[derive(Default)]
pub struct FontTableParser {
    pub reader: FontReader,
    pub font_table: HashMap<String, u64>,
    pub glyph_locations: Vec<u64>,
    pub glyphs_data: Vec<Vec<Vec2>>,
}

impl FontTableParser {
    pub fn get_lookup_table(&mut self) -> std::io::Result<()> {
        self.reader.skip_bytes(4); // skip scaler type
        let n_tables = self.reader.read_u16()?;
        self.reader.skip_bytes(6); // skip searchRange, entrySelector and rangeShift

        let mut table_data: HashMap<String, u64> = HashMap::with_capacity(n_tables as usize);
        for _ in 0..n_tables {
            let tag = self.reader.read_tag()?;
            let _checksum = self.reader.read_u32()?;
            let offset = self.reader.read_u32()?;
            let _length = self.reader.read_u32()?;
            table_data.insert(tag, offset as u64);
        }

        self.font_table = table_data;
        Ok(())
    }

    pub fn get_glyph_location(&mut self) -> std::io::Result<()> {
        let loca_table_loc = self.font_table["loca"];
        let glyf_table_loc = self.font_table["glyf"];
        
        self.reader.go_to(self.font_table["maxp"] + 4); // skip version
        let num_glyphs = self.reader.read_u16()? as usize;

        self.reader.go_to(self.font_table["head"] + 50); // skip version, fontRevision .... till fontDirectionHint
        let is_two_byte_entry = self.reader.read_i16()? == 0; // 0 is short (2 byte) offset, 1 is long (4 byte) (indexToLocFormat)
        
        self.reader.go_to(loca_table_loc);
        for _ in 0..num_glyphs {
            let glyph_offset = if is_two_byte_entry {
                (self.reader.read_u16()? * 2) as u64 // two byte format has halved offset so we multiply by 2
            } else {
                self.reader.read_u32()? as u64
            };
            
            self.glyph_locations.push(glyf_table_loc + glyph_offset);
        }
    
        Ok(())
    }

    pub fn get_glyph_data(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for glyf_location in self.glyph_locations.iter() {
            self.reader.go_to(*glyf_location);

            let n_contours = self.reader.read_i16()? as usize;
            if n_contours == usize::MAX {
                // compound glyph
                continue;
            }

            let mut contour_end_pts = Vec::with_capacity(n_contours);
            self.reader.skip_bytes(8); // skip the FWord bounding boxes (each one is 2 bytes)

            for _ in 0..contour_end_pts.capacity() {
                contour_end_pts.push(self.reader.read_u16()?);
            }

            let instructions_length = self.reader.read_u16()?;
            self.reader.skip_bytes(instructions_length as u64); // skip instructions 

            let flag_capacity: usize = *contour_end_pts.last().unwrap_or(&0) as usize + 1;
            let mut flags: Vec<u8> = vec![0; flag_capacity];

            for mut i in 0..flag_capacity {
                let flag = self.reader.read_byte()?;
                flags[i] = flag;

                if bit_is_set(flag, 3) {
                    for _ in 0..self.reader.read_byte()? {
                        flags[i] = flag;
                        i += 1;
                        if flags.capacity() == flags.len() {
                            break;
                        }
                    }
                }
            }

            let coordinates = get_coordinates(&mut self.reader, &flags)?;
            self.glyphs_data.push(coordinates);
        }

        Ok(())
    }
}
