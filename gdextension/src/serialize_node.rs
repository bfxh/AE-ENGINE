use godot::prelude::*;

use wasteland_serialize::binary::{BinaryReader, BinaryWriter};
use wasteland_serialize::bitpack::BitPacker;
use wasteland_serialize::schema::SchemaRegistry;
use wasteland_serialize::zerocopy::ZeroCopyBuf;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandSerialize {
    #[var]
    use_compression: bool,
    #[var]
    alignment: i64,
    #[var]
    schema_version: i64,

    registry: SchemaRegistry,
    write_count: i64,
    read_count: i64,
    total_bytes_written: i64,
    total_bytes_read: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandSerialize {
    fn init(base: Base<Node>) -> Self {
        Self {
            use_compression: true,
            alignment: 8,
            schema_version: 1,
            registry: SchemaRegistry::new(),
            write_count: 0,
            read_count: 0,
            total_bytes_written: 0,
            total_bytes_read: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandSerialize {
    #[func]
    fn write_u8(&mut self, value: i64) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_u8(value as u8);
        self.write_count += 1;
        self.total_bytes_written += 1;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_u32(&mut self, value: i64) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_u32(value as u32);
        self.write_count += 1;
        self.total_bytes_written += 4;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_u64(&mut self, value: i64) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_u64(value as u64);
        self.write_count += 1;
        self.total_bytes_written += 8;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_f32(&mut self, value: f32) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_f32(value);
        self.write_count += 1;
        self.total_bytes_written += 4;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_f64(&mut self, value: f64) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_f64(value);
        self.write_count += 1;
        self.total_bytes_written += 8;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_bytes(&mut self, data: PackedByteArray) -> PackedByteArray {
        let mut writer = BinaryWriter::new(data.len() + 16);
        writer.write_bytes(data.as_slice());
        self.write_count += 1;
        self.total_bytes_written += data.len() as i64 + 4;
        to_packed_byte_array(&writer.into_vec())
    }

    #[func]
    fn write_varint(&mut self, value: i64) -> PackedByteArray {
        let mut writer = BinaryWriter::new(16);
        writer.write_varint(value as u64);
        self.write_count += 1;
        let bytes = writer.into_vec();
        self.total_bytes_written += bytes.len() as i64;
        to_packed_byte_array(&bytes)
    }

    #[func]
    fn read_u8(&mut self, data: PackedByteArray) -> i64 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_u8().unwrap_or(0);
        self.read_count += 1;
        self.total_bytes_read += 1;
        val as i64
    }

    #[func]
    fn read_u32(&mut self, data: PackedByteArray) -> i64 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_u32().unwrap_or(0);
        self.read_count += 1;
        self.total_bytes_read += 4;
        val as i64
    }

    #[func]
    fn read_u64(&mut self, data: PackedByteArray) -> i64 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_u64().unwrap_or(0);
        self.read_count += 1;
        self.total_bytes_read += 8;
        val as i64
    }

    #[func]
    fn read_f32(&mut self, data: PackedByteArray) -> f32 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_f32().unwrap_or(0.0);
        self.read_count += 1;
        self.total_bytes_read += 4;
        val
    }

    #[func]
    fn read_f64(&mut self, data: PackedByteArray) -> f64 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_f64().unwrap_or(0.0);
        self.read_count += 1;
        self.total_bytes_read += 8;
        val
    }

    #[func]
    fn read_varint(&mut self, data: PackedByteArray) -> i64 {
        let mut reader = BinaryReader::new(data.as_slice());
        let val = reader.read_varint().unwrap_or(0);
        self.read_count += 1;
        self.total_bytes_read += data.len() as i64;
        val as i64
    }

    #[func]
    fn pack_bits(&self, values: PackedFloat32Array, bits_per_value: i64) -> PackedByteArray {
        let mut packer = BitPacker::new();
        let data: Vec<f32> = values.as_slice().to_vec();
        for &v in &data {
            let bits = v.to_bits();
            packer.write_bits(bits as u64, bits_per_value as usize);
        }
        let packed = packer.to_bytes();
        let mut arr = PackedByteArray::new();
        for &b in packed.iter() {
            arr.push(b);
        }
        arr
    }

    #[func]
    fn unpack_bits(
        &self,
        packed: PackedByteArray,
        bits_per_value: i64,
        count: i64,
    ) -> PackedFloat32Array {
        let mut arr = PackedFloat32Array::new();
        let bytes = packed.as_slice();
        let words: Vec<u64> = bytes
            .chunks(8)
            .map(|c| {
                let mut buf = [0u8; 8];
                buf[..c.len()].copy_from_slice(c);
                u64::from_le_bytes(buf)
            })
            .collect();
        let mut unpacker = wasteland_serialize::bitpack::BitUnpacker::new(&words);
        for _i in 0..count as usize {
            if let Some(bits) = unpacker.read_bits(bits_per_value as usize) {
                arr.push(f32::from_bits(bits as u32));
            }
        }
        arr
    }

    #[func]
    fn create_zero_copy_buffer(&self, data: PackedByteArray) -> i64 {
        let buf = ZeroCopyBuf::from_vec(data.as_slice().to_vec());
        let len = buf.len();
        drop(buf);
        len as i64
    }

    #[func]
    fn register_schema(&mut self, name: GString, _fields_json: GString) -> bool {
        let schema = wasteland_serialize::schema::Schema {
            name: name.to_string(),
            version: self.schema_version as u32,
            fields: vec![],
            alignment: self.alignment as usize,
        };
        self.registry.register(schema);
        true
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "write_count" => self.write_count,
            "read_count" => self.read_count,
            "total_bytes_written" => self.total_bytes_written,
            "total_bytes_read" => self.total_bytes_read,
            "schema_version" => self.schema_version,
            "use_compression" => self.use_compression,
        }
    }
}

fn to_packed_byte_array(data: &[u8]) -> PackedByteArray {
    let mut arr = PackedByteArray::new();
    for &b in data.iter() {
        arr.push(b);
    }
    arr
}
