pub trait BinaryWritable {
    fn write(&self, buf: &mut Vec<u8>);
    fn size_hint() -> usize;
}

pub trait BinaryReadable: Sized {
    fn read(data: &[u8], offset: &mut usize) -> Option<Self>;
}

pub struct BinaryWriter {
    buf: Vec<u8>,
    position: usize,
}

impl BinaryWriter {
    pub fn new(capacity: usize) -> Self {
        BinaryWriter { buf: Vec::with_capacity(capacity), position: 0 }
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
        self.position += 1;
    }

    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.position += 2;
    }

    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.position += 4;
    }

    pub fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.position += 8;
    }

    pub fn write_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.position += 4;
    }

    pub fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
        self.position += 8;
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        self.write_u32(data.len() as u32);
        self.buf.extend_from_slice(data);
        self.position += data.len();
    }

    pub fn write_varint(&mut self, v: u64) {
        let mut val = v;
        while val >= 0x80 {
            self.buf.push((val as u8) | 0x80);
            val >>= 7;
        }
        self.buf.push(val as u8);
        self.position = self.buf.len();
    }

    pub fn align(&mut self, alignment: usize) {
        let pad = (alignment - self.position % alignment) % alignment;
        for _ in 0..pad {
            self.buf.push(0);
        }
        self.position += pad;
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

pub struct BinaryReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BinaryReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BinaryReader { data, position: 0 }
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        if self.position < self.data.len() {
            let v = self.data[self.position];
            self.position += 1;
            Some(v)
        } else {
            None
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        if self.position + 1 < self.data.len() {
            let v = u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
            self.position += 2;
            Some(v)
        } else {
            None
        }
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        if self.position + 3 < self.data.len() {
            let v = u32::from_le_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
            ]);
            self.position += 4;
            Some(v)
        } else {
            None
        }
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        if self.position + 7 < self.data.len() {
            let v = u64::from_le_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
                self.data[self.position + 4],
                self.data[self.position + 5],
                self.data[self.position + 6],
                self.data[self.position + 7],
            ]);
            self.position += 8;
            Some(v)
        } else {
            None
        }
    }

    pub fn read_f32(&mut self) -> Option<f32> {
        self.read_u32().map(|v| f32::from_le_bytes(v.to_le_bytes()))
    }

    pub fn read_f64(&mut self) -> Option<f64> {
        self.read_u64().map(|v| f64::from_le_bytes(v.to_le_bytes()))
    }

    pub fn read_bytes(&mut self) -> Option<Vec<u8>> {
        let len = self.read_u32()? as usize;
        if self.position + len <= self.data.len() {
            let v = self.data[self.position..self.position + len].to_vec();
            self.position += len;
            Some(v)
        } else {
            None
        }
    }

    pub fn read_varint(&mut self) -> Option<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        loop {
            if self.position >= self.data.len() {
                return None;
            }
            let byte = self.data[self.position];
            self.position += 1;
            result |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
    }

    pub fn align(&mut self, alignment: usize) {
        let pad = (alignment - self.position % alignment) % alignment;
        self.position = (self.position + pad).min(self.data.len());
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read_primitives() {
        let mut w = BinaryWriter::new(64);
        w.write_u8(42);
        w.write_u16(1000);
        w.write_u32(100000);
        w.write_u64(10000000000);
        w.write_f32(std::f32::consts::PI);
        w.write_f64(std::f64::consts::E);

        let data = w.into_vec();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_u8(), Some(42));
        assert_eq!(r.read_u16(), Some(1000));
        assert_eq!(r.read_u32(), Some(100000));
        assert_eq!(r.read_u64(), Some(10000000000));
        let f = r.read_f32().unwrap();
        assert!((f - std::f32::consts::PI).abs() < 0.001);
        let d = r.read_f64().unwrap();
        assert!((d - std::f64::consts::E).abs() < 0.001);
    }

    #[test]
    fn test_varint() {
        let mut w = BinaryWriter::new(32);
        w.write_varint(0);
        w.write_varint(127);
        w.write_varint(128);
        w.write_varint(1000000);

        let data = w.into_vec();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_varint(), Some(0));
        assert_eq!(r.read_varint(), Some(127));
        assert_eq!(r.read_varint(), Some(128));
        assert_eq!(r.read_varint(), Some(1000000));
    }

    #[test]
    fn test_write_read_bytes() {
        let mut w = BinaryWriter::new(64);
        w.write_bytes(&[1, 2, 3, 4, 5]);
        w.write_bytes(&[]);

        let data = w.into_vec();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_bytes(), Some(vec![1, 2, 3, 4, 5]));
        assert_eq!(r.read_bytes(), Some(vec![]));
    }

    #[test]
    fn test_alignment() {
        let mut w = BinaryWriter::new(64);
        w.write_u8(1);
        w.align(4);
        w.write_u32(42);

        let data = w.into_vec();
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_u8(), Some(1));
        r.align(4);
        assert_eq!(r.read_u32(), Some(42));
    }

    #[test]
    fn test_reader_boundary() {
        let data = vec![1, 2];
        let mut r = BinaryReader::new(&data);
        assert_eq!(r.read_u8(), Some(1));
        assert_eq!(r.read_u8(), Some(2));
        assert_eq!(r.read_u8(), None);
        assert_eq!(r.read_u32(), None);
    }
}
