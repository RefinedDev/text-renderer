use std::fs;
use std::io::{Cursor, Read, Result};

#[derive(Default)]
pub struct FontReader {
    cursor: Cursor<Vec<u8>>,
}

impl FontReader {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Ok(FontReader {
            cursor: Cursor::new(fs::read(path)?),
        })
    }

    pub fn read_byte(&mut self) -> Result<u8> {
        let mut byte = [0; 1];
        self.cursor.read_exact(&mut byte)?;
        Ok(byte[0])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let mut bytes = [0; 2];
        self.cursor.read_exact(&mut bytes)?;
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let mut bytes = [0; 4];
        self.cursor.read_exact(&mut bytes)?;
        Ok(u32::from_be_bytes(bytes))
    }
    
    pub fn read_i16(&mut self) -> Result<i16> {
        let mut bytes = [0; 2];
        self.cursor.read_exact(&mut bytes)?;
        Ok(i16::from_be_bytes(bytes))
    }

    pub fn read_tag(&mut self) -> Result<String> {
        let mut tag = String::with_capacity(4);
        for _ in 0..tag.capacity() {
            tag.push(char::from_u32(self.read_byte()? as u32).expect("Could not convert to char"));
        }
        Ok(tag)
    }

    pub fn skip_bytes(&mut self, pos: u64) {
        self.cursor.set_position(self.cursor.position() + pos);
    }

    pub fn go_to(&mut self, pos: u64) {
        self.cursor.set_position(pos);
    }

    pub fn get_location(&self) -> u64 {
        self.cursor.position()
    }
}