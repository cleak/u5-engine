

impl<'a> LzwBitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, bit_pos: 0 }
    }

    pub fn read_code(&mut self, code_size: u8) -> Option<u16> {
        let bit_count = code_size as usize;
        if self.bit_pos + bit_count > self.bytes.len() * 8 {
            return None;
        }

        let mut code = 0u16;
        for bit_offset in 0..bit_count {
            let source_bit = self.bit_pos + bit_offset;
            let bit = (self.bytes[source_bit / 8] >> (source_bit % 8)) & 1;
            code |= (bit as u16) << bit_offset;
        }
        self.bit_pos += bit_count;
        Some(code)
    }
}

