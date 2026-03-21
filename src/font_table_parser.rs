use bevy::math::Vec2;
use std::collections::HashMap;

use crate::font_reader::FontReader;

const FONT_SIZE_CONSTANT: f32 = 85.0;
// https://developer.apple.com/fonts/TrueType-Reference-Manual/
// https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6glyf.html
fn bit_is_set(flag: u8, bit: u8) -> bool {
    return (flag >> bit) & 1 == 1;
}

fn get_coordinates(
    reader: &mut FontReader,
    flags: &Vec<u8>,
    font_scale: f32,
) -> Result<Vec<(Vec2, bool)>, Box<dyn std::error::Error>> {
    let mut coordinates: Vec<(Vec2, bool)> = vec![(Vec2::ZERO, false); flags.len()];

    // FOR X
    let mut short_vector_bit = 1;
    let mut sign_or_skip_bit = 4;

    for i in 0..coordinates.capacity() {
        coordinates[i].0.x = coordinates[i16::max(0, (i as i16) - 1) as usize].0.x; // all points are with respect to previous one and we need with respect to origin (0,0)
        let flag = flags[i];
        let on_curve = bit_is_set(flag, 0);
        coordinates[i].1 = on_curve;

        if bit_is_set(flag, short_vector_bit) {
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].0.x += (reader.read_byte()? as f32 * sign)*font_scale;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].0.x += (reader.read_i16()? as f32)*font_scale;
        }
    }

    // FOR Y
    short_vector_bit = 2;
    sign_or_skip_bit = 5;

    for i in 0..coordinates.capacity() {
        coordinates[i].0.y = coordinates[i16::max(0, (i as i16) - 1) as usize].0.y;
        let flag = flags[i];
        let on_curve = bit_is_set(flag, 0);
        coordinates[i].1 = on_curve;

        if bit_is_set(flag, short_vector_bit) {
            let sign: f32 = if bit_is_set(flag, sign_or_skip_bit) {
                1.0
            } else {
                -1.0
            };
            coordinates[i].0.y += (reader.read_byte()? as f32 * sign)*font_scale;
        } else if !bit_is_set(flag, sign_or_skip_bit) {
            coordinates[i].0.y += (reader.read_i16()? as f32)*font_scale;
        }
    }

    // move towards (0,0)
    

    Ok(coordinates)
}

#[derive(Clone, Default)]
pub struct Glyph {
    pub coordinates: Vec<(Vec2, bool)>, // bool is for on_curve parameter
    pub contour_end_pts: Vec<u16>,
    pub contour_coordinates: Vec<Vec<(Vec2, u8)>>, // these are setup in main.rs (setup_implied_points)
    pub bounding_box: [f32; 4], // x_min, y_min, x_max, y_max
}

#[derive(Default)]
pub struct FontData {
    pub reader: FontReader,
    pub font_table: HashMap<String, u64>,
    pub glyph_locations: Vec<u64>,
    pub glyphs: Vec<Glyph>,
    pub unicodes_to_index: HashMap<u32, usize>,
    pub glyph_spaces: Vec<f32>,
    pub font_scale: f32,
    pub line_height: f32,
}

impl FontData {
    //https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6.html
    pub fn get_lookup_table(&mut self) -> std::io::Result<()> {
        self.reader.skip_bytes(4); // skip scaler type
        let n_tables = self.reader.read_u16()?;
        self.reader.skip_bytes(6); // skip searchRange, entrySelector and rangeShift

        let mut table_data: HashMap<String, u64> = HashMap::with_capacity(n_tables as usize);
        for _ in 0..n_tables {
            let tag = self.reader.read_tag()?;
            self.reader.skip_bytes(4); // let _checksum = self.reader.read_u32()?; 
            let offset = self.reader.read_u32()?;
            self.reader.skip_bytes(4); // let _length = self.reader.read_u32()?;
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        // GET FONT SCALE BEFORE THAT
        let prev_location = self.reader.get_location();
        self.reader.go_to(self.font_table["head"] + 18);
        self.font_scale = FONT_SIZE_CONSTANT/self.reader.read_u16()? as f32;
        self.reader.go_to(prev_location);

        enum CompoundScaleForm {
            NotMatrix([f32; 2]),
            Matrix([f32; 4])
        }         
        let mut compound_glyph_hashes: Vec<HashMap<[usize; 2], [Vec2; 3]>> = Vec::with_capacity(20); // ((glyf_index, loop_index), (offset, bound_min, bound_max))
        let mut compound_glyph_scales: Vec<CompoundScaleForm> = Vec::with_capacity(20);
        for (loop_index, glyf_location) in self.glyph_locations.iter().enumerate() {
            self.reader.go_to(*glyf_location);

            let n_contours = self.reader.read_i16()? as usize;
            if n_contours == usize::MAX { // COMPOUND GLYPH
                self.glyphs.push(Glyph::default());
                
                /*
                since there is arbitrary ordering of compound and simple glyphs ill just store this data 
                somewhere and stich them up after all simple glyphs have been loaded.
                */
                
                let (x_min, y_min, x_max, y_max) = (self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale);
                
                // ((glyf_index, loop_index), (offset, bound_min, bound_max))
                let mut glyf_data: HashMap<[usize; 2], [Vec2; 3]> = HashMap::with_capacity(2);  // its messy i know but its aight
                loop { 
                    let flags = self.reader.read_u16()? as u8; //  since i do need the bits above 8 just make it one byte
                    let glyph_index = self.reader.read_u16()? as usize;

                    let arg_are_2_bytes = bit_is_set(flags, 0);
                    let args_are_xy_values = bit_is_set(flags, 1);
                    let x_y_scale_same = bit_is_set(flags, 3);
                    let x_y_scale_not_same = bit_is_set(flags, 6);
                    let is_2by2 = bit_is_set(flags, 7);

                    if !args_are_xy_values {todo!("arguments are points not xyvalues")}; // have not really encountered fonts that use fonts yet so ill implement if i find one :3

                    // as i8 because range of u8 is [0,255] where as i8 is [-128, +127] and negative offsets can exist (this shit took too long to figure out 😭).
                    let x_offset = if arg_are_2_bytes {self.reader.read_i16()? as f32} else {self.reader.read_byte()? as i8 as f32};
                    let y_offset = if arg_are_2_bytes {self.reader.read_i16()? as f32} else {self.reader.read_byte()? as i8 as f32};

                    if !is_2by2 {
                        let mut x_scale = 1.0;
                        let mut y_scale = 1.0;
                        if x_y_scale_same {
                            x_scale = self.reader.read_i16()? as f32 / 16384.0;
                            y_scale = x_scale;
                        } else if  x_y_scale_not_same {
                            x_scale = self.reader.read_i16()? as f32 / 16384.0;
                            y_scale = self.reader.read_i16()? as f32 / 16384.0;
                        }
                        compound_glyph_scales.push(CompoundScaleForm::NotMatrix([x_scale,y_scale]));
                    } else {
                        let x_scale = self.reader.read_i16()? as f32 / 16384.0;
                        let scale01 = self.reader.read_i16()? as f32 / 16384.0;
                        let scale10 = self.reader.read_i16()? as f32 / 16384.0;
                        let y_scale = self.reader.read_i16()? as f32 / 16384.0;
                        compound_glyph_scales.push(CompoundScaleForm::Matrix([x_scale,scale01,scale10,y_scale]));
                    }

                    glyf_data.insert([glyph_index, loop_index], [Vec2::new(x_offset, y_offset), Vec2::new(x_min, y_min), Vec2::new(x_max, y_max)]);

                    if !bit_is_set(flags, 5) {
                        break;
                    }
                }
                compound_glyph_hashes.push(glyf_data);
            } else { // SIMPLE GLYPH
                let mut contour_end_pts = Vec::with_capacity(n_contours);
                let (x_min, y_min, x_max, y_max) = (self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale,self.reader.read_i16()? as f32*self.font_scale);
                
                for _ in 0..n_contours {
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

                let coordinates = get_coordinates(&mut self.reader, &flags, self.font_scale)?;
                self.glyphs.push(Glyph { coordinates, contour_end_pts, contour_coordinates: Vec::with_capacity(n_contours), bounding_box: [x_min,y_min,x_max,y_max] });
            }
        }

        // ALL SIMPLE GLYPHS LOADED SO WE STITCH UP COMPOUND GLYPHS
        for compound_glyf in compound_glyph_hashes.into_iter() {
            let mut new_coordinates: Vec<(Vec2, bool)> = Vec::with_capacity(105);
            let mut new_contour_end_pts: Vec<u16> = Vec::with_capacity(5); 
            let mut insert_at: usize = 0; // loop_index
            let mut last_end_point: u16 = 0;
            let mut bounding_box: Option<[f32;4]> = None;
            for (index,glyf) in compound_glyf.iter().enumerate() {
                if let None = bounding_box {
                    insert_at = glyf.0[1]; // loop_index
                    bounding_box = Some([glyf.1[1].x, glyf.1[1].y, glyf.1[2].x ,glyf.1[2].y]);
                }
                
                let glyph = &self.glyphs[glyf.0[0]]; // glyph_index
                let offset_x = glyf.1[0].x*self.font_scale;
                let offset_y = glyf.1[0].y*self.font_scale;
               
               let scaleform = &compound_glyph_scales[index];
                if let CompoundScaleForm::NotMatrix(scale) = scaleform {
                    let s_x = scale[0];
                    let s_y = scale[1];  
                    glyph.coordinates.iter().for_each(|f| new_coordinates.push((Vec2::new(f.0.x*s_x + offset_x, f.0.y*s_y + offset_y), f.1)));
                } else if let CompoundScaleForm::Matrix(scale) = scaleform {
                    let a11 = scale[0];
                    let a12 = scale[1];
                    let a21 = scale[2];
                    let a22 = scale[3];
                    glyph.coordinates.iter().for_each(|f| new_coordinates.push((Vec2::new(f.0.x*a11 + f.0.y*a12 + offset_x, f.0.x*a21 + f.0.y*a22 + offset_y), f.1)));
                }
                
                glyph.contour_end_pts.iter().for_each(|f| new_contour_end_pts.push(f + last_end_point));
                
                last_end_point += *glyph.contour_end_pts.last().unwrap_or(&0) + 1;
            }
            
            self.glyphs[insert_at] = Glyph { coordinates: new_coordinates, contour_end_pts: new_contour_end_pts, contour_coordinates: Vec::with_capacity(5), bounding_box: bounding_box.unwrap() };
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

    pub fn get_glyph_spacings(&mut self) -> std::io::Result<()> {
        self.reader.go_to(self.font_table["hhea"] + 2);
        
        let ascent = self.reader.read_i16()? as f32;
        let descent = self.reader.read_i16()? as f32;
        
        self.reader.skip_bytes(28);
        let num_long_hor_metrics = self.reader.read_u16()?;

        self.reader.go_to(self.font_table["hmtx"]);
        let mut advance_widths: Vec<f32> = Vec::with_capacity(self.glyphs.len());

        for _ in 0..num_long_hor_metrics {
            advance_widths.push(self.reader.read_u16()? as f32);
            self.reader.skip_bytes(2);
        }

        // some fonts include a run of mono-spaced glyphs at the end
        // they all share the same advanced width value as whatever we read last
        let num_monospaced = self.glyphs.len() - num_long_hor_metrics as usize;
        let monospace_aw = advance_widths[num_monospaced];
        
        for _ in 0..num_monospaced {
            advance_widths.push(monospace_aw);
        }
        
        self.glyph_spaces = advance_widths;
        self.line_height = (ascent - descent) * self.font_scale * 1.2;
        Ok(())
    }
}
