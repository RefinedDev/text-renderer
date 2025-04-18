use bevy::math::Vec2;
use std::collections::HashMap;

use crate::font_reader::FontReader;

// https://developer.apple.com/fonts/TrueType-Reference-Manual/
// https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6glyf.html
fn bit_is_set(flag: u8, bit: u8) -> bool {
    return (flag >> bit) & 1 == 1;
}

const FONT_SIZE_FACTOR: f32 = 10.0; // larger means the smaller font
fn get_coordinates(
    reader: &mut FontReader,
    flags: &Vec<u8>,
    window_size: Vec2,
) -> Result<Vec<(Vec2, bool)>, Box<dyn std::error::Error>> {
    let mut short_vector_bit = 1;
    let mut sign_or_skip_bit = 4;
    let mut coordinates: Vec<(Vec2, bool)> = vec![(Vec2::ZERO, false); flags.len()];

    // FOR X
    for i in 0..coordinates.capacity() {
        coordinates[i].0.x = coordinates[i16::max(0, (i as i16) - 1) as usize].0.x;
        let flag = flags[i];
        let on_curve = bit_is_set(flag, 0);
        coordinates[i].1 = on_curve;

        if bit_is_set(flag, short_vector_bit) {
            let coordinate = reader.read_byte()? as f32;
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].0.x += (coordinate * sign)/FONT_SIZE_FACTOR;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].0.x += (reader.read_i16()? as f32)/FONT_SIZE_FACTOR;
        }
    }

    short_vector_bit = 2;
    sign_or_skip_bit = 5;

    // FOR Y
    for i in 0..coordinates.capacity() {
        coordinates[i].0.y = coordinates[i16::max(0, (i as i16) - 1) as usize].0.y;
        let flag = flags[i];
        let on_curve = bit_is_set(flag, 0);
        coordinates[i].1 = on_curve;

        if bit_is_set(flag, short_vector_bit) {
            let coordinate = reader.read_byte()? as f32;
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].0.y += (coordinate * sign)/FONT_SIZE_FACTOR;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].0.y += (reader.read_i16()? as f32)/FONT_SIZE_FACTOR;
        }
    }

    // with respect to origin
    let first_point = coordinates[0].0;
    for (point, _) in coordinates.iter_mut() {
        point.x -= first_point.x + window_size.x/2.25;
        point.y -= first_point.y - window_size.y/4.0;
    }

    Ok(coordinates)
}

#[derive(Clone)]
pub struct Glyph {
    pub coordinates: Vec<(Vec2, bool)>, // bool is for on_curve parameter
    pub contour_end_pts: Vec<u16>,
}

#[derive(Default)]
pub struct FontTableParser {
    pub reader: FontReader,
    pub font_table: HashMap<String, u64>,
    pub glyph_locations: Vec<u64>,
    pub glyphs: Vec<Glyph>,
    pub unicodes_to_index: HashMap<u32, usize>,
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
                self.reader.read_u16()? as u64 * 2 // two byte format has halved offset so we multiply by 2
            } else {
                self.reader.read_u32()? as u64
            };
            
            self.glyph_locations.push(glyf_table_loc + glyph_offset);
        }
    
        Ok(())
    }

    pub fn get_glyphs(
        &mut self,
        window_size: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for glyf_location in self.glyph_locations.iter() {
            self.reader.go_to(*glyf_location);

            let n_contours = self.reader.read_i16()? as usize;
            if n_contours == usize::MAX {
                self.glyphs.push(self.glyphs[0].clone());
                continue;  // compound glyph
            }

            let mut contour_end_pts = Vec::with_capacity(n_contours);
            self.reader.skip_bytes(8); // skip the FWord bounding boxes (each one is 2 bytes)

            for _ in 0..contour_end_pts.capacity() {
                contour_end_pts.push(self.reader.read_u16()?);
            }

            let instructions_length = self.reader.read_u16()?;
            self.reader.skip_bytes(instructions_length as u64); // skip instructions 

            let flag_capacity: usize = *contour_end_pts.last().unwrap_or(&0) as usize + 1;
            let mut flags: Vec<u8> = Vec::with_capacity(flag_capacity);

            let mut i = 0;
            while i < flag_capacity {
                i += 1;
                let flag = self.reader.read_byte()?;
                flags.push(flag);

                if bit_is_set(flag, 3) {
                    for _ in 0..self.reader.read_byte()? {
                        flags.push(flag);
                        i += 1;
                    }
                }
                
            }

            let coordinates = get_coordinates(&mut self.reader, &flags, window_size)?;
            self.glyphs.push(Glyph { coordinates, contour_end_pts });
        }

        Ok(())
    }

    // https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6cmap.html
    pub fn map_glyph_to_unicode(
        &mut self
    ) -> std::io::Result<()> {
        self.reader.go_to(self.font_table["cmap"]);

        self.reader.skip_bytes(2); // skip version
        let n_subtables = self.reader.read_u16()?;

        let mut cmap_subtable_offset = u32::MAX;
        for _ in 0..(n_subtables as usize) {
            let platform_id = self.reader.read_u16()?;
            let platform_specific_id = self.reader.read_u16()?;
            let offset = self.reader.read_u32()?;

            if platform_id == 0 { // 0 is unicode
                if platform_specific_id == 4 { // unicode 2.0 (non bmp allowed)
                    cmap_subtable_offset = offset;
                }
                if platform_specific_id == 3 && cmap_subtable_offset == u32::MAX { // unicode 2.0 (bmp only)
                    cmap_subtable_offset = offset;
                }
            }
        }
        
        if cmap_subtable_offset == u32::MAX {
            panic!("Font does not support the needed character map type");
        }

        self.reader.go_to(self.font_table["cmap"] + cmap_subtable_offset as u64);

        let mut unicode_to_index_map: HashMap<u32, usize> = HashMap::with_capacity(self.glyphs.len());
        
        let format = self.reader.read_u16()?;
        if format != 4 && format != 12 {
            panic!("Font character map format not supported");
        } else if format == 12 {
            self.reader.skip_bytes(10); // skip reserved, length, language
            let n_groups = self.reader.read_u32()?;
            for _ in 0..n_groups {
                let start_char_code = self.reader.read_u32()?;
                let end_char_code = self.reader.read_u32()?;
                let start_glyph_code = self.reader.read_u32()?;

                for char_code_offset in 0..(end_char_code - start_char_code + 1) as usize {
                    let char_code = start_char_code + char_code_offset as u32;
                    let glyph_index = start_glyph_code as usize + char_code_offset;
                    unicode_to_index_map.insert(char_code, glyph_index);
                }
            }
        } else if format == 4 {
            self.reader.skip_bytes(4); // skip length, language
            let seg_count = (self.reader.read_u16()?/2) as usize;
            self.reader.skip_bytes(6); // skip searchRange, entrySelector, rangeShift
            
            let mut end_codes: Vec<u32> = Vec::with_capacity(seg_count);
            for _ in 0..seg_count {
                end_codes.push(self.reader.read_u16()? as u32);
            }

            self.reader.skip_bytes(2); // skip reservedPad

            let mut start_codes: Vec<u32> = Vec::with_capacity(seg_count);
            for _ in 0..seg_count {
                start_codes.push(self.reader.read_u16()? as u32);
            }

            let mut id_deltas: Vec<u32> = Vec::with_capacity(seg_count);
            for _ in 0..seg_count {
                id_deltas.push(self.reader.read_u16()? as u32);
            }
            
            let mut id_range_offsets: Vec<(u64, u64)> = Vec::with_capacity(seg_count); // (current_location, offset)
            for _ in 0..seg_count {
                id_range_offsets.push((self.reader.get_location(), self.reader.read_u16()? as u64));
            }
            
            for i in 0..start_codes.len() {
                let end_code = end_codes[i];
                let mut curr_code = start_codes[i];

                while curr_code <= end_code {
                    let mut glyph_index = 0;

                    if id_range_offsets[i].1 == 0 {
                        glyph_index = (curr_code + id_deltas[i]) % 65536;
                    } else {
                        let range_offset_location = id_range_offsets[i].0 + id_range_offsets[i].1;
                        let glyph_index_address = range_offset_location + (2 * (curr_code - start_codes[i])) as u64;

                        let reader_prev_location = self.reader.get_location();
                        self.reader.go_to(glyph_index_address);

                        let glyph_index_offset = self.reader.read_u16()? as u32;
                        self.reader.go_to(reader_prev_location);

                        if glyph_index_offset != 0 {
                            glyph_index = (glyph_index_offset + id_deltas[i]) % 65536;
                        }
                    }

                    unicode_to_index_map.insert(curr_code, glyph_index as usize);
                    curr_code += 1;
                }
            }
        }       

        self.unicodes_to_index = unicode_to_index_map;
        Ok(())
    }
}
