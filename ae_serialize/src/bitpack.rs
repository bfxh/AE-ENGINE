pub struct BitPacker {
    buf: Vec<u64>,
    bit_pos: usize,
    total_bits: usize,
}

impl BitPacker {
    pub fn new() -> Self {
        BitPacker { buf: vec![0], bit_pos: 0, total_bits: 0 }
    }

    pub fn with_capacity(capacity_bits: usize) -> Self {
        let words = capacity_bits.div_ceil(64);
        BitPacker { buf: vec![0; words], bit_pos: 0, total_bits: 0 }
    }

    pub fn write_bits(&mut self, value: u64, num_bits: usize) {
        if num_bits == 0 {
            return;
        }
        let num_bits = num_bits.min(64);
        let mask = if num_bits == 64 { u64::MAX } else { (1u64 << num_bits) - 1 };
        let v = value & mask;
        let word_idx = self.bit_pos / 64;
        let bit_offset = self.bit_pos % 64;
        if word_idx >= self.buf.len() {
            self.buf.push(0);
        }
        self.buf[word_idx] |= v << bit_offset;
        if bit_offset + num_bits > 64 {
            let overflow = bit_offset + num_bits - 64;
            if word_idx + 1 >= self.buf.len() {
                self.buf.push(0);
            }
            self.buf[word_idx + 1] |= v >> (num_bits - overflow);
        }
        self.bit_pos += num_bits;
        self.total_bits += num_bits;
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_bits(if value { 1 } else { 0 }, 1);
    }

    pub fn write_u32_compact(&mut self, value: u32) {
        if value < 0x80 {
            self.write_bits(value as u64, 8);
        } else if value < 0x8000 {
            self.write_bits(0x80 | 0x40, 8);
            self.write_bits(value as u64, 16);
        } else {
            self.write_bits(0x80 | 0x40 | 0x20, 8);
            self.write_bits(value as u64, 32);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.buf.len() * 8);
        for word in &self.buf {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    pub fn bit_count(&self) -> usize {
        self.total_bits
    }

    pub fn byte_count(&self) -> usize {
        self.total_bits.div_ceil(8)
    }

    pub fn compression_ratio(&self, original_bytes: usize) -> f32 {
        if original_bytes == 0 {
            return 1.0;
        }
        self.byte_count() as f32 / original_bytes as f32
    }
}

impl Default for BitPacker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BitUnpacker<'a> {
    data: &'a [u64],
    bit_pos: usize,
}

impl<'a> BitUnpacker<'a> {
    pub fn new(data: &'a [u64]) -> Self {
        BitUnpacker { data, bit_pos: 0 }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        let words = bytes.len().div_ceil(8);
        let aligned = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const u64, words) };
        BitUnpacker { data: aligned, bit_pos: 0 }
    }

    pub fn read_bits(&mut self, num_bits: usize) -> Option<u64> {
        if num_bits == 0 {
            return Some(0);
        }
        let num_bits = num_bits.min(64);
        let word_idx = self.bit_pos / 64;
        let bit_offset = self.bit_pos % 64;
        if word_idx >= self.data.len() {
            return None;
        }
        let mut value = (self.data[word_idx] >> bit_offset)
            & if num_bits == 64 { u64::MAX } else { (1u64 << num_bits) - 1 };
        if bit_offset + num_bits > 64 {
            let overflow = bit_offset + num_bits - 64;
            if word_idx + 1 >= self.data.len() {
                return None;
            }
            let upper = self.data[word_idx + 1] & ((1u64 << overflow) - 1);
            value |= upper << (num_bits - overflow);
        }
        self.bit_pos += num_bits;
        Some(value)
    }

    pub fn read_bool(&mut self) -> Option<bool> {
        self.read_bits(1).map(|v| v != 0)
    }

    pub fn read_u32_compact(&mut self) -> Option<u32> {
        let header = self.read_bits(8)? as u32;
        if header & 0x80 == 0 {
            return Some(header & 0x7f);
        }
        if header & 0x40 != 0 {
            if header & 0x20 != 0 {
                Some(self.read_bits(32)? as u32)
            } else {
                Some(self.read_bits(16)? as u32)
            }
        } else {
            Some(self.read_bits(16)? as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read_bits() {
        let mut packer = BitPacker::new();
        packer.write_bits(0b101, 3);
        packer.write_bits(0b1100, 4);
        packer.write_bits(0b1, 1);

        let bytes = packer.to_bytes();
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| {
                let mut arr = [0u8; 8];
                arr[..c.len()].copy_from_slice(c);
                u64::from_le_bytes(arr)
            })
            .collect();
        let mut unpacker = BitUnpacker::new(&words);
        assert_eq!(unpacker.read_bits(3), Some(0b101));
        assert_eq!(unpacker.read_bits(4), Some(0b1100));
        assert_eq!(unpacker.read_bits(1), Some(0b1));
    }

    #[test]
    fn test_write_bool() {
        let mut packer = BitPacker::new();
        packer.write_bool(true);
        packer.write_bool(false);
        packer.write_bool(true);
        packer.write_bool(true);

        let bytes = packer.to_bytes();
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| {
                let mut arr = [0u8; 8];
                arr[..c.len()].copy_from_slice(c);
                u64::from_le_bytes(arr)
            })
            .collect();
        let mut unpacker = BitUnpacker::new(&words);
        assert_eq!(unpacker.read_bool(), Some(true));
        assert_eq!(unpacker.read_bool(), Some(false));
        assert_eq!(unpacker.read_bool(), Some(true));
        assert_eq!(unpacker.read_bool(), Some(true));
    }

    #[test]
    fn test_compact_u32() {
        let mut packer = BitPacker::new();
        packer.write_u32_compact(50);
        packer.write_u32_compact(5000);
        packer.write_u32_compact(100000);

        let bytes = packer.to_bytes();
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| {
                let mut arr = [0u8; 8];
                arr[..c.len()].copy_from_slice(c);
                u64::from_le_bytes(arr)
            })
            .collect();
        let mut unpacker = BitUnpacker::new(&words);
        assert_eq!(unpacker.read_u32_compact(), Some(50));
        assert_eq!(unpacker.read_u32_compact(), Some(5000));
        assert_eq!(unpacker.read_u32_compact(), Some(100000));
    }

    #[test]
    fn test_cross_64bit_boundary() {
        let mut packer = BitPacker::new();
        packer.write_bits(u64::MAX >> 1, 63);
        packer.write_bits(0b1010, 4);

        let bytes = packer.to_bytes();
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| {
                let mut arr = [0u8; 8];
                arr[..c.len()].copy_from_slice(c);
                u64::from_le_bytes(arr)
            })
            .collect();
        let mut unpacker = BitUnpacker::new(&words);
        assert_eq!(unpacker.read_bits(63), Some(u64::MAX >> 1));
        assert_eq!(unpacker.read_bits(4), Some(0b1010));
    }

    #[test]
    fn test_empty() {
        let packer = BitPacker::new();
        assert_eq!(packer.bit_count(), 0);
        assert_eq!(packer.byte_count(), 0);
    }
}
