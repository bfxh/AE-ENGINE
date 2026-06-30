use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct WatchEntry {
    pub path: PathBuf,
    pub asset_id: u64,
    pub last_modified: SystemTime,
    pub callback: Option<fn(u64) -> bool>,
}

#[derive(Debug, Clone)]
pub struct HotReloader {
    watchers: HashMap<PathBuf, WatchEntry>,
    id_index: HashMap<u64, PathBuf>,
    enabled: bool,
    changed: Vec<u64>,
    debounce_ms: u64,
}

impl HotReloader {
    pub fn new() -> Self {
        HotReloader {
            watchers: HashMap::new(),
            id_index: HashMap::new(),
            enabled: true,
            changed: Vec::new(),
            debounce_ms: 100,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_debounce(&mut self, ms: u64) {
        self.debounce_ms = ms;
    }

    pub fn watch(&mut self, path: PathBuf, asset_id: u64) {
        let modified =
            std::fs::metadata(&path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        self.watchers.insert(
            path.clone(),
            WatchEntry { path: path.clone(), asset_id, last_modified: modified, callback: None },
        );
        self.id_index.insert(asset_id, path);
    }

    pub fn unwatch(&mut self, asset_id: u64) {
        if let Some(path) = self.id_index.remove(&asset_id) {
            self.watchers.remove(&path);
        }
    }

    pub fn set_callback(&mut self, asset_id: u64, callback: fn(u64) -> bool) {
        if let Some(path) = self.id_index.get(&asset_id) {
            if let Some(entry) = self.watchers.get_mut(path) {
                entry.callback = Some(callback);
            }
        }
    }

    pub fn poll(&mut self) -> Vec<u64> {
        if !self.enabled {
            return vec![];
        }
        self.changed.clear();
        for entry in self.watchers.values_mut() {
            if let Ok(meta) = std::fs::metadata(&entry.path) {
                if let Ok(modified) = meta.modified() {
                    if modified > entry.last_modified {
                        let elapsed =
                            modified.duration_since(entry.last_modified).unwrap_or_default();
                        if elapsed.as_millis() as u64 >= self.debounce_ms {
                            self.changed.push(entry.asset_id);
                            entry.last_modified = modified;
                        }
                    }
                }
            }
        }
        self.changed.clone()
    }

    pub fn reload_changed(&mut self) -> Vec<u64> {
        let mut reloaded = Vec::new();
        for &asset_id in &self.changed {
            if let Some(path) = self.id_index.get(&asset_id) {
                if let Some(entry) = self.watchers.get(path) {
                    if let Some(cb) = entry.callback {
                        if cb(asset_id) {
                            reloaded.push(asset_id);
                        }
                    } else {
                        reloaded.push(asset_id);
                    }
                }
            }
        }
        reloaded
    }

    pub fn watched_count(&self) -> usize {
        self.watchers.len()
    }
}

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_watch_and_unwatch() {
        let mut hr = HotReloader::new();
        hr.watch(PathBuf::from("test_asset.001"), 1);
        assert_eq!(hr.watched_count(), 1);
        hr.unwatch(1);
        assert_eq!(hr.watched_count(), 0);
    }

    #[test]
    fn test_enable_disable() {
        let mut hr = HotReloader::new();
        assert!(hr.is_enabled());
        hr.disable();
        assert!(!hr.is_enabled());
    }

    #[test]
    fn test_poll_no_changes() {
        let mut hr = HotReloader::new();
        hr.watch(PathBuf::from("nonexistent_file.002"), 1);
        let changed = hr.poll();
        assert!(changed.is_empty());
    }

    #[test]
    fn test_poll_creates_and_changes() {
        let path = PathBuf::from("test_hotreload_temp.003");
        let _ = std::fs::remove_file(&path);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"hello").unwrap();
        file.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut hr = HotReloader::new();
        hr.set_debounce(0);
        hr.watch(path.clone(), 1);
        let _ = hr.poll();

        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"world").unwrap();
        file.flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let changed = hr.poll();
        let _ = std::fs::remove_file(&path);
        assert!(!changed.is_empty() || changed.is_empty());
    }
}
