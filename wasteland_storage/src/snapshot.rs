use crate::compression;
use crate::delta::WorldDelta;
use crate::migration::SchemaVersion;
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 4] = b"WSTL";
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub schema: SchemaVersion,
    pub frame: u64,
    pub timestamp: u64,
    pub entity_count: u32,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub crc32: u32,
}

impl SnapshotHeader {
    pub fn validate(&self) -> Result<(), String> {
        if &self.magic != MAGIC {
            return Err(format!("invalid magic: expected {:?}, got {:?}", MAGIC, self.magic));
        }
        if self.version != VERSION {
            return Err(format!("unsupported version: {} (expected {})", self.version, VERSION));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub header: SnapshotHeader,
    pub data: Vec<u8>,
}

impl Snapshot {
    pub fn new(schema: SchemaVersion, frame: u64, timestamp: u64, raw_data: &[u8]) -> Self {
        let compressed = compression::compress_snapshot(raw_data);
        let crc32 = crc32_fast(raw_data);
        let header = SnapshotHeader {
            magic: *MAGIC,
            version: VERSION,
            schema,
            frame,
            timestamp,
            entity_count: 0,
            compressed_size: compressed.len() as u32,
            uncompressed_size: raw_data.len() as u32,
            crc32,
        };
        Snapshot { header, data: compressed }
    }

    pub fn decompress(&self) -> Vec<u8> {
        compression::decompress_snapshot(&self.data)
    }

    pub fn verify_integrity(&self) -> bool {
        let decompressed = self.decompress();
        let crc = crc32_fast(&decompressed);
        crc == self.header.crc32
    }

    pub fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("serialize failed: {}", e))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        let snapshot: Snapshot =
            bincode::deserialize(data).map_err(|e| format!("deserialize failed: {}", e))?;
        snapshot.header.validate()?;
        Ok(snapshot)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub snapshots: Vec<Snapshot>,
    pub deltas: Vec<WorldDelta>,
    pub current_frame: u64,
}

impl SaveFile {
    pub fn new() -> Self {
        SaveFile { snapshots: Vec::new(), deltas: Vec::new(), current_frame: 0 }
    }

    pub fn add_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshots.push(snapshot);
    }

    pub fn add_delta(&mut self, delta: WorldDelta) {
        self.deltas.push(delta);
    }

    pub fn latest_snapshot(&self) -> Option<&Snapshot> {
        self.snapshots.last()
    }

    pub fn size_bytes(&self) -> usize {
        self.snapshots.iter().map(|s| s.data.len()).sum::<usize>()
            + self
                .deltas
                .iter()
                .map(|d| d.entities.iter().map(|e| e.component_data.len()).sum::<usize>())
                .sum::<usize>()
    }
}

impl Default for SaveFile {
    fn default() -> Self {
        Self::new()
    }
}

fn crc32_fast(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffffffff;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let schema = SchemaVersion::new(1, 0, 0);
        let raw = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        let snap = Snapshot::new(schema, 42, 1000, &raw);
        let serialized = snap.serialize().unwrap();
        let deserialized = Snapshot::deserialize(&serialized).unwrap();
        let decompressed = deserialized.decompress();
        assert_eq!(decompressed, raw);
    }

    #[test]
    fn test_snapshot_integrity() {
        let schema = SchemaVersion::new(1, 0, 0);
        let raw = vec![0u8; 1024];
        let snap = Snapshot::new(schema, 0, 0, &raw);
        assert!(snap.verify_integrity());
    }

    #[test]
    fn test_header_validation() {
        let header = SnapshotHeader {
            magic: *MAGIC,
            version: VERSION,
            schema: SchemaVersion::new(1, 0, 0),
            frame: 0,
            timestamp: 0,
            entity_count: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            crc32: 0,
        };
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_header_validation_bad_magic() {
        let header = SnapshotHeader {
            magic: [0, 0, 0, 0],
            version: VERSION,
            schema: SchemaVersion::new(1, 0, 0),
            frame: 0,
            timestamp: 0,
            entity_count: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            crc32: 0,
        };
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_save_file() {
        let mut save = SaveFile::new();
        let schema = SchemaVersion::new(1, 0, 0);
        let snap = Snapshot::new(schema.clone(), 0, 0, &[1, 2, 3]);
        save.add_snapshot(snap);
        assert!(save.latest_snapshot().is_some());
        assert!(save.size_bytes() > 0);
    }

    #[test]
    fn test_snapshot_medium_data() {
        let schema = SchemaVersion::new(1, 0, 0);
        let raw = vec![42u8; 10000];
        let snap = Snapshot::new(schema, 100, 5000, &raw);
        let serialized = snap.serialize().unwrap();
        let deserialized = Snapshot::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.decompress(), raw);
        assert!(deserialized.verify_integrity());
    }
}
