use godot::prelude::*;

use ae_storage::compression::{compress_snapshot, decompress_snapshot};
use ae_storage::delta::WorldDelta;
use ae_storage::migration::SchemaVersion;
use ae_storage::snapshot::Snapshot;

#[derive(GodotClass)]
#[class(base=Node)]
pub(crate) struct WastelandStorage {
    #[var]
    save_directory: GString,
    #[var]
    compression_level: i64,
    #[var]
    auto_save_interval_secs: f32,

    snapshot_count: i64,
    delta_count: i64,
    total_saved_bytes: i64,
    total_loaded_bytes: i64,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandStorage {
    fn init(base: Base<Node>) -> Self {
        Self {
            save_directory: GString::from("saves/"),
            compression_level: 6,
            auto_save_interval_secs: 300.0,
            snapshot_count: 0,
            delta_count: 0,
            total_saved_bytes: 0,
            total_loaded_bytes: 0,
            base,
        }
    }
}

#[godot_api]
impl WastelandStorage {
    #[func]
    fn create_snapshot(
        &mut self,
        raw_data: PackedByteArray,
        frame: i64,
        timestamp: i64,
    ) -> Dictionary<Variant, Variant> {
        let data = raw_data.as_slice();
        let schema = SchemaVersion::new(1, 0, 0);
        let snapshot = Snapshot::new(schema, frame as u64, timestamp as u64, data);
        let compressed_size = snapshot.header.compressed_size;
        let original_size = snapshot.header.uncompressed_size;
        self.snapshot_count += 1;
        self.total_saved_bytes += compressed_size as i64;
        dict! {
            "frame" => snapshot.header.frame as i64,
            "compressed_size" => compressed_size,
            "original_size" => original_size as i64,
            "crc32" => snapshot.header.crc32,
            "ratio" => if original_size > 0 { compressed_size as f32 / original_size as f32 } else { 0.0 },
        }
    }

    #[func]
    fn verify_snapshot_integrity(&self, raw_data: PackedByteArray) -> bool {
        let data = raw_data.as_slice();
        let schema = SchemaVersion::new(1, 0, 0);
        let snapshot = Snapshot::new(schema, 0, 0, data);
        snapshot.header.validate().is_ok()
    }

    #[func]
    fn decompress_snapshot_data(&self, compressed: PackedByteArray) -> PackedByteArray {
        let data = decompress_snapshot(compressed.as_slice());
        let mut arr = PackedByteArray::new();
        for &b in data.iter() {
            arr.push(b);
        }
        arr
    }

    #[func]
    fn compress_data(&self, raw: PackedByteArray) -> PackedByteArray {
        let data = compress_snapshot(raw.as_slice());
        let mut arr = PackedByteArray::new();
        for &b in data.iter() {
            arr.push(b);
        }
        arr
    }

    #[func]
    fn create_delta(
        &mut self,
        from_frame: i64,
        to_frame: i64,
        _changed_entities: PackedInt64Array,
    ) -> Dictionary<Variant, Variant> {
        let delta = WorldDelta::new(from_frame as u64, to_frame as u64);
        self.delta_count += 1;
        dict! {
            "from_frame" => delta.frame_from as i64,
            "to_frame" => delta.frame_to as i64,
            "entity_count" => delta.entities.len() as i64,
            "delta_size" => delta.global_state.len() as i64,
        }
    }

    #[func]
    fn check_schema_compatibility(&self, major: i64, minor: i64, patch: i64) -> bool {
        let version = SchemaVersion::new(major as u32, minor as u32, patch as u32);
        let current = SchemaVersion::new(1, 0, 0);
        version.is_compatible(&current)
    }

    #[func]
    fn get_stats(&self) -> Dictionary<Variant, Variant> {
        dict! {
            "snapshot_count" => self.snapshot_count,
            "delta_count" => self.delta_count,
            "total_saved_bytes" => self.total_saved_bytes,
            "total_loaded_bytes" => self.total_loaded_bytes,
            "save_directory" => &self.save_directory,
        }
    }
}
