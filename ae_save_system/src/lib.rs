use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use ae_metaentity::meta_entity::MetaEntity;
use ae_unified_interface::UnifiedWorld;

const MAGIC: [u8; 3] = [0x57, 0x4C, 0x44];
const CURRENT_SAVE_VERSION: u32 = 1;
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveFormat {
    Binary = 0,
    Json = 1,
    Compressed = 2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveHeader {
    pub version: u32,
    pub timestamp: u64,
    pub world_name: String,
    pub entity_count: usize,
    pub checksum: u64,
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub play_time: f64,
    pub tick_count: u64,
    pub player_position: Vec3,
    pub seed: u64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub header: SaveHeader,
    pub metadata: SaveMetadata,
    pub entities: Vec<MetaEntity>,
}

#[derive(Debug)]
pub enum SaveError {
    IoError(std::io::Error),
    SerializationError(String),
    VersionMismatch { expected: u32, found: u32 },
    ChecksumMismatch { expected: u64, computed: u64 },
    CorruptedData(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::IoError(e) => write!(f, "IO error: {}", e),
            SaveError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            SaveError::VersionMismatch { expected, found } => {
                write!(f, "Version mismatch: expected {}, found {}", expected, found)
            },
            SaveError::ChecksumMismatch { expected, computed } => {
                write!(f, "Checksum mismatch: expected {}, computed {}", expected, computed)
            },
            SaveError::CorruptedData(e) => write!(f, "Corrupted data: {}", e),
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SaveError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::IoError(e)
    }
}

fn compute_checksum(entities: &[MetaEntity]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entity in entities {
        entity.id.hash(&mut hasher);
        entity.version.hash(&mut hasher);
        entity.position.x.to_bits().hash(&mut hasher);
        entity.position.y.to_bits().hash(&mut hasher);
        entity.position.z.to_bits().hash(&mut hasher);
        entity.attribute_hash().hash(&mut hasher);
    }
    hasher.finish()
}

fn serialize_save_data(data: &SaveData, format: SaveFormat) -> Result<Vec<u8>, SaveError> {
    let payload =
        match format {
            SaveFormat::Binary => bincode::serialize(data)
                .map_err(|e| SaveError::SerializationError(e.to_string()))?,
            SaveFormat::Json => serde_json::to_vec(data)
                .map_err(|e| SaveError::SerializationError(e.to_string()))?,
            SaveFormat::Compressed => {
                let bin = bincode::serialize(data)
                    .map_err(|e| SaveError::SerializationError(e.to_string()))?;
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&bin).map_err(SaveError::IoError)?;
                encoder.finish().map_err(SaveError::IoError)?
            },
        };

    let mut output = Vec::with_capacity(8 + payload.len());
    output.extend_from_slice(&MAGIC);
    output.push(format as u8);
    output.extend_from_slice(&CURRENT_SAVE_VERSION.to_le_bytes());
    output.extend_from_slice(&payload);
    Ok(output)
}

fn deserialize_save_data(bytes: &[u8]) -> Result<(SaveFormat, SaveData), SaveError> {
    if bytes.len() < 8 {
        return Err(SaveError::CorruptedData("file too short".into()));
    }
    if bytes[0..3] != MAGIC {
        return Err(SaveError::CorruptedData("invalid magic bytes".into()));
    }
    let format_byte = bytes[3];
    let format = match format_byte {
        0 => SaveFormat::Binary,
        1 => SaveFormat::Json,
        2 => SaveFormat::Compressed,
        _ => return Err(SaveError::CorruptedData(format!("unknown format byte: {}", format_byte))),
    };
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version > CURRENT_SAVE_VERSION {
        return Err(SaveError::VersionMismatch { expected: CURRENT_SAVE_VERSION, found: version });
    }

    let payload = &bytes[8..];
    let data: SaveData = match format {
        SaveFormat::Binary => bincode::deserialize(payload)
            .map_err(|e| SaveError::SerializationError(e.to_string()))?,
        SaveFormat::Json => serde_json::from_slice(payload)
            .map_err(|e| SaveError::SerializationError(e.to_string()))?,
        SaveFormat::Compressed => {
            let mut decoder = GzDecoder::new(payload);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).map_err(SaveError::IoError)?;
            bincode::deserialize(&decompressed)
                .map_err(|e| SaveError::SerializationError(e.to_string()))?
        },
    };

    Ok((format, data))
}

pub struct QuickSaveManager {
    max_slots: usize,
    current_slot: usize,
}

impl QuickSaveManager {
    pub fn new(max_slots: usize) -> Self {
        Self { max_slots: max_slots.min(3), current_slot: 0 }
    }

    pub fn next_slot(&mut self) -> usize {
        let slot = self.current_slot;
        self.current_slot = (self.current_slot + 1) % self.max_slots;
        slot
    }

    pub fn slot_path(&self, save_dir: &Path, slot: usize, format: SaveFormat) -> PathBuf {
        let ext = match format {
            SaveFormat::Binary => "wsave",
            SaveFormat::Json => "wsave.json",
            SaveFormat::Compressed => "wsave.gz",
        };
        save_dir.join(format!("quicksave_{}.{}", slot, ext))
    }

    pub fn all_slot_paths(&self, save_dir: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for slot in 0..self.max_slots {
            for ext in &["wsave", "wsave.json", "wsave.gz"] {
                let p = save_dir.join(format!("quicksave_{}.{}", slot, ext));
                if p.exists() {
                    paths.push(p);
                    break;
                }
            }
        }
        paths
    }
}

impl Default for QuickSaveManager {
    fn default() -> Self {
        Self::new(3)
    }
}

pub struct AutoSaveManager {
    interval_seconds: f64,
    max_saves: usize,
    last_auto_save: Option<SystemTime>,
    save_counter: usize,
}

impl AutoSaveManager {
    pub fn new(interval_seconds: f64, max_saves: usize) -> Self {
        Self { interval_seconds, max_saves, last_auto_save: None, save_counter: 0 }
    }

    pub fn should_save(&self) -> bool {
        match self.last_auto_save {
            None => true,
            Some(last) => {
                let elapsed = last.elapsed().map(|d| d.as_secs_f64()).unwrap_or(f64::MAX);
                elapsed >= self.interval_seconds
            },
        }
    }

    pub fn mark_saved(&mut self) {
        self.last_auto_save = Some(SystemTime::now());
        self.save_counter += 1;
    }

    pub fn auto_save_path(&self, save_dir: &Path, format: SaveFormat) -> PathBuf {
        let ext = match format {
            SaveFormat::Binary => "wsave",
            SaveFormat::Json => "wsave.json",
            SaveFormat::Compressed => "wsave.gz",
        };
        save_dir.join(format!("autosave_{}.{}", self.save_counter % self.max_saves, ext))
    }

    pub fn cleanup_old_saves(&self, save_dir: &Path) {
        for i in 0..self.max_saves {
            for ext in &["wsave", "wsave.json", "wsave.gz"] {
                let p = save_dir.join(format!("autosave_{}.{}", i, ext));
                if p.exists() {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }
}

impl Default for AutoSaveManager {
    fn default() -> Self {
        Self::new(300.0, 5)
    }
}

pub struct SaveSystem {
    save_dir: PathBuf,
    schema_version: u32,
    last_entity_versions: HashMap<Uuid, u64>,
    quick_save_manager: QuickSaveManager,
    auto_save_manager: AutoSaveManager,
}

impl SaveSystem {
    pub fn new(save_dir: impl Into<PathBuf>) -> Self {
        let dir = save_dir.into();
        let _ = fs::create_dir_all(&dir);
        Self {
            save_dir: dir,
            schema_version: CURRENT_SCHEMA_VERSION,
            last_entity_versions: HashMap::new(),
            quick_save_manager: QuickSaveManager::default(),
            auto_save_manager: AutoSaveManager::default(),
        }
    }

    pub fn with_auto_save_config(
        save_dir: impl Into<PathBuf>,
        interval_seconds: f64,
        max_saves: usize,
    ) -> Self {
        let dir = save_dir.into();
        let _ = fs::create_dir_all(&dir);
        Self {
            save_dir: dir,
            schema_version: CURRENT_SCHEMA_VERSION,
            last_entity_versions: HashMap::new(),
            quick_save_manager: QuickSaveManager::default(),
            auto_save_manager: AutoSaveManager::new(interval_seconds, max_saves),
        }
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn reset_incremental_state(&mut self) {
        self.last_entity_versions.clear();
    }

    pub fn mark_entity_saved(&mut self, id: Uuid, version: u64) {
        self.last_entity_versions.insert(id, version);
    }

    pub fn save<W: UnifiedWorld>(
        &mut self,
        world: &W,
        metadata: SaveMetadata,
        path: &Path,
        format: SaveFormat,
        incremental: bool,
    ) -> Result<SaveHeader, SaveError> {
        let entities: Vec<MetaEntity> = if incremental {
            let all = world.query_entities(&|_| true);
            let mut changed = Vec::new();
            for e in all {
                match self.last_entity_versions.get(&e.id).copied() {
                    None => changed.push(e.clone()),
                    Some(v) if e.version > v => changed.push(e.clone()),
                    _ => {},
                }
            }
            changed
        } else {
            world.query_entities(&|_| true).into_iter().cloned().collect()
        };

        let checksum = compute_checksum(&entities);
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let header = SaveHeader {
            version: CURRENT_SAVE_VERSION,
            timestamp,
            world_name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string(),
            entity_count: entities.len(),
            checksum,
            schema_version: self.schema_version,
        };

        let data = SaveData { header: header.clone(), metadata, entities };

        let serialized = serialize_save_data(&data, format)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &serialized)?;

        for entity in &data.entities {
            self.last_entity_versions.insert(entity.id, entity.version);
        }

        Ok(header)
    }

    pub fn save_full<W: UnifiedWorld>(
        &mut self,
        world: &W,
        metadata: SaveMetadata,
        path: &Path,
        format: SaveFormat,
    ) -> Result<SaveHeader, SaveError> {
        self.save(world, metadata, path, format, false)
    }

    pub fn save_incremental<W: UnifiedWorld>(
        &mut self,
        world: &W,
        metadata: SaveMetadata,
        path: &Path,
        format: SaveFormat,
    ) -> Result<SaveHeader, SaveError> {
        self.save(world, metadata, path, format, true)
    }

    pub fn load(
        &self,
        path: &Path,
    ) -> Result<(SaveHeader, SaveMetadata, Vec<MetaEntity>), SaveError> {
        let bytes = fs::read(path)?;
        let (_format, data) = deserialize_save_data(&bytes)?;

        if data.header.schema_version > self.schema_version {
            return Err(SaveError::VersionMismatch {
                expected: self.schema_version,
                found: data.header.schema_version,
            });
        }

        let computed = compute_checksum(&data.entities);
        if computed != data.header.checksum {
            return Err(SaveError::ChecksumMismatch { expected: data.header.checksum, computed });
        }

        Ok((data.header, data.metadata, data.entities))
    }

    pub fn quick_save<W: UnifiedWorld>(
        &mut self,
        world: &W,
        metadata: SaveMetadata,
        format: SaveFormat,
    ) -> Result<SaveHeader, SaveError> {
        let slot = self.quick_save_manager.next_slot();
        let path = self.quick_save_manager.slot_path(&self.save_dir, slot, format);
        self.save_full(world, metadata, &path, format)
    }

    pub fn quick_load(&self) -> Result<(SaveHeader, SaveMetadata, Vec<MetaEntity>), SaveError> {
        let paths = self.quick_save_manager.all_slot_paths(&self.save_dir);
        if paths.is_empty() {
            return Err(SaveError::CorruptedData("no quick saves found".into()));
        }

        let mut newest: Option<(SystemTime, PathBuf)> = None;
        for p in &paths {
            if let Ok(meta) = fs::metadata(p) {
                if let Ok(mod_time) = meta.modified() {
                    match newest {
                        None => newest = Some((mod_time, p.clone())),
                        Some((t, _)) if mod_time > t => newest = Some((mod_time, p.clone())),
                        _ => {},
                    }
                }
            }
        }

        match newest {
            Some((_, path)) => self.load(&path),
            None => Err(SaveError::CorruptedData("no valid quick saves found".into())),
        }
    }

    pub fn auto_save<W: UnifiedWorld>(
        &mut self,
        world: &W,
        metadata: SaveMetadata,
        format: SaveFormat,
    ) -> Result<Option<SaveHeader>, SaveError> {
        if !self.auto_save_manager.should_save() {
            return Ok(None);
        }

        let path = self.auto_save_manager.auto_save_path(&self.save_dir, format);
        let header = self.save_full(world, metadata, &path, format)?;
        self.auto_save_manager.mark_saved();
        Ok(Some(header))
    }

    pub fn list_saves(&self) -> Result<Vec<(String, SaveMetadata)>, SaveError> {
        let mut saves = Vec::new();
        let entries = fs::read_dir(&self.save_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
            match self.load(&path) {
                Ok((_header, meta, _entities)) => {
                    saves.push((name, meta));
                },
                Err(_) => continue,
            }
        }

        saves.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(saves)
    }

    pub fn delete_save(&self, name: &str) -> Result<bool, SaveError> {
        let mut deleted = false;
        for ext in &["wsave", "wsave.json", "wsave.gz"] {
            let path = self.save_dir.join(format!("{}.{}", name, ext));
            if path.exists() {
                fs::remove_file(&path)?;
                deleted = true;
            }
        }
        Ok(deleted)
    }

    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    pub fn change_save_dir(&mut self, dir: impl Into<PathBuf>) {
        self.save_dir = dir.into();
        let _ = fs::create_dir_all(&self.save_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ae_metaentity::meta_entity::MetaEntity;
    use ae_unified_interface::WorldStorage;

    fn create_test_world() -> WorldStorage {
        let mut world = WorldStorage::new();
        world.spawn_entity(MetaEntity::iron(Vec3::new(0.0, 0.0, 0.0), 0));
        world.spawn_entity(MetaEntity::water(Vec3::new(1.0, 0.0, 0.0), 0));
        world.spawn_entity(MetaEntity::concrete(Vec3::new(2.0, 0.0, 0.0), 0));
        world
    }

    fn test_metadata() -> SaveMetadata {
        SaveMetadata {
            play_time: 3600.0,
            tick_count: 1000,
            player_position: Vec3::new(10.0, 5.0, 0.0),
            seed: 42,
            tags: vec!["test".into(), "debug".into()],
        }
    }

    fn temp_save_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ae_save_test_{}", Uuid::new_v4()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_save_load_json() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("test_json.wsave.json");
        let meta = test_metadata();

        system.save_full(&world, meta.clone(), &path, SaveFormat::Json).unwrap();
        assert!(path.exists());

        let (header, loaded_meta, entities) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 3);
        assert_eq!(loaded_meta.play_time, 3600.0);
        assert_eq!(loaded_meta.tick_count, 1000);
        assert_eq!(loaded_meta.seed, 42);
        assert_eq!(entities.len(), 3);
        assert_eq!(loaded_meta.tags.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_load_binary() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("test_bin.wsave");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();
        assert!(path.exists());

        let (header, _loaded_meta, entities) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 3);
        assert_eq!(entities.len(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_load_compressed() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("test_gz.wsave.gz");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Compressed).unwrap();
        assert!(path.exists());

        let (header, _loaded_meta, entities) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 3);
        assert_eq!(entities.len(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_checksum_validation() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("test_checksum.wsave");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();

        let (header, _, _) = system.load(&path).unwrap();
        assert_ne!(header.checksum, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_corrupted_data_detection() {
        let dir = temp_save_dir();
        let path = dir.join("corrupt.wsave");
        fs::write(&path, b"this is not a valid save file").unwrap();

        let system = SaveSystem::new(&dir);
        let result = system.load(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_incremental_save() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let mut world = create_test_world();
        let path = dir.join("test_incr.wsave");
        let meta = test_metadata();

        system.save_incremental(&world, meta.clone(), &path, SaveFormat::Binary).unwrap();
        let (header, _, entities) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 3);
        assert_eq!(entities.len(), 3);

        let mut new_entity = MetaEntity::iron(Vec3::new(100.0, 0.0, 0.0), 0);
        new_entity.apply_force(Vec3::X);
        world.spawn_entity(new_entity);

        let path2 = dir.join("test_incr2.wsave");
        system.save_incremental(&world, meta, &path2, SaveFormat::Binary).unwrap();
        let (header2, _, entities2) = system.load(&path2).unwrap();
        assert_eq!(header2.entity_count, 1);
        assert_eq!(entities2.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_list_saves() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let meta = test_metadata();

        system
            .save_full(&world, meta.clone(), &dir.join("save_a.wsave"), SaveFormat::Binary)
            .unwrap();
        system
            .save_full(&world, meta.clone(), &dir.join("save_b.wsave"), SaveFormat::Binary)
            .unwrap();

        let saves = system.list_saves().unwrap();
        assert!(saves.len() >= 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete_save() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let meta = test_metadata();
        let path = dir.join("delete_me.wsave");

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();
        assert!(path.exists());

        let deleted = system.delete_save("delete_me").unwrap();
        assert!(deleted);
        assert!(!path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_quick_save_and_load() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let meta = test_metadata();

        system.quick_save(&world, meta.clone(), SaveFormat::Binary).unwrap();

        let (header, _loaded_meta, entities) = system.quick_load().unwrap();
        assert_eq!(header.entity_count, 3);
        assert_eq!(entities.len(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_auto_save_triggers() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::with_auto_save_config(&dir, 3600.0, 3);
        let _world = create_test_world();
        let _meta = test_metadata();

        assert!(system.auto_save_manager.should_save());

        system.auto_save_manager.mark_saved();
        assert!(!system.auto_save_manager.should_save());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_auto_save_slot_rotation() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::with_auto_save_config(&dir, 0.0, 2);
        let world = create_test_world();
        let meta = test_metadata();

        system.auto_save(&world, meta.clone(), SaveFormat::Binary).unwrap();
        system.auto_save_manager.mark_saved();
        system.auto_save(&world, meta.clone(), SaveFormat::Binary).unwrap();
        system.auto_save_manager.mark_saved();
        system.auto_save(&world, meta, SaveFormat::Binary).unwrap();

        let saves = system.list_saves().unwrap();
        let auto_saves: Vec<_> = saves.iter().filter(|(n, _)| n.starts_with("auto")).collect();
        assert!(!auto_saves.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_quick_save_slot_rotation() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let meta = test_metadata();

        for _ in 0..5 {
            system.quick_save(&world, meta.clone(), SaveFormat::Binary).unwrap();
        }

        let paths = system.quick_save_manager.all_slot_paths(&dir);
        assert_eq!(paths.len(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_version_mismatch_detection() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("version_test.wsave");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        bytes[4..8].copy_from_slice(&999u32.to_le_bytes());
        fs::write(&path, &bytes).unwrap();

        let result = system.load(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_checksum_mismatch_detection() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("checksum_test.wsave");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        let (format, mut data) = deserialize_save_data(&bytes).unwrap();
        data.header.checksum = 0xDEADBEEF;
        bytes = serialize_save_data(&data, format).unwrap();
        fs::write(&path, &bytes).unwrap();

        let result = system.load(&path);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_entity_data_preservation() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let path = dir.join("entity_test.wsave");
        let meta = test_metadata();

        system.save_full(&world, meta, &path, SaveFormat::Json).unwrap();

        let (_, _, entities) = system.load(&path).unwrap();
        let iron = entities.iter().find(|e| e.physics.density == 7874.0).unwrap();
        assert_eq!(
            iron.chemistry.elemental_composition[0].element,
            ae_metaentity::meta_entity::Element::Fe
        );
        assert_eq!(iron.physics.hardness, 4.0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_empty_world_save() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = WorldStorage::new();
        let path = dir.join("empty.wsave");
        let meta = SaveMetadata {
            play_time: 0.0,
            tick_count: 0,
            player_position: Vec3::ZERO,
            seed: 0,
            tags: vec![],
        };

        system.save_full(&world, meta, &path, SaveFormat::Binary).unwrap();
        let (header, _, entities) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 0);
        assert_eq!(entities.len(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_reset_incremental_state() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();

        let path = dir.join("incr_test.wsave");
        system.save_incremental(&world, test_metadata(), &path, SaveFormat::Binary).unwrap();
        let (header, _, _) = system.load(&path).unwrap();
        assert_eq!(header.entity_count, 3);

        system.reset_incremental_state();

        let path2 = dir.join("incr_test2.wsave");
        system.save_incremental(&world, test_metadata(), &path2, SaveFormat::Binary).unwrap();
        let (header2, _, _) = system.load(&path2).unwrap();
        assert_eq!(header2.entity_count, 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_format_roundtrip() {
        let dir = temp_save_dir();
        let mut system = SaveSystem::new(&dir);
        let world = create_test_world();
        let meta = test_metadata();

        for format in &[SaveFormat::Binary, SaveFormat::Json, SaveFormat::Compressed] {
            let ext = match format {
                SaveFormat::Binary => "wsave",
                SaveFormat::Json => "wsave.json",
                SaveFormat::Compressed => "wsave.gz",
            };
            let path = dir.join(format!("roundtrip.{}", ext));
            system.save_full(&world, meta.clone(), &path, *format).unwrap();
            let (header, loaded_meta, entities) = system.load(&path).unwrap();
            assert_eq!(header.entity_count, 3);
            assert_eq!(loaded_meta.seed, 42);
            assert_eq!(entities.len(), 3);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_change_save_dir() {
        let dir1 = temp_save_dir();
        let dir2 = temp_save_dir();
        let mut system = SaveSystem::new(&dir1);
        let world = create_test_world();
        let path = dir1.join("test.wsave");

        system.save_full(&world, test_metadata(), &path, SaveFormat::Binary).unwrap();
        assert!(path.exists());

        system.change_save_dir(&dir2);
        let saves = system.list_saves().unwrap();
        assert!(saves.is_empty());

        let _ = fs::remove_dir_all(&dir1);
        let _ = fs::remove_dir_all(&dir2);
    }
}
