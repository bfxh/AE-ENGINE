use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::event_source::{EventStore, EventType, GameEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteEventStore {
    pub inner: EventStore,
    pub db_path: Option<String>,
    pub auto_snapshot_interval: u64,
    pub last_snapshot_tick: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
}

impl SqliteEventStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            inner: EventStore::new(max_events),
            db_path: None,
            auto_snapshot_interval: 600,
            last_snapshot_tick: 0,
            total_bytes_written: 0,
            total_bytes_read: 0,
        }
    }

    pub fn with_sqlite(max_events: usize, db_path: &str) -> Self {
        let mut store = Self::new(max_events);
        store.db_path = Some(db_path.to_string());
        store.init_sqlite();
        store
    }

    fn init_sqlite(&mut self) {
        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                let _ = conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS events (
                        event_id TEXT PRIMARY KEY,
                        tick INTEGER NOT NULL,
                        timestamp REAL NOT NULL,
                        event_type INTEGER NOT NULL,
                        entity_id TEXT,
                        data BLOB
                    );
                    CREATE INDEX IF NOT EXISTS idx_events_tick ON events(tick);
                    CREATE INDEX IF NOT EXISTS idx_events_entity ON events(entity_id);
                    CREATE TABLE IF NOT EXISTS snapshots (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        tick INTEGER NOT NULL,
                        timestamp REAL NOT NULL,
                        data BLOB NOT NULL,
                        event_index INTEGER NOT NULL
                    );
                    CREATE INDEX IF NOT EXISTS idx_snapshots_tick ON snapshots(tick);
                    CREATE TABLE IF NOT EXISTS world_metadata (
                        key TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    );",
                );
            }
        }
    }

    pub fn record(
        &mut self,
        tick: u64,
        timestamp: f64,
        event_type: EventType,
        entity_id: Option<Uuid>,
        data: Vec<u8>,
    ) -> Uuid {
        let event_id = self.inner.record(tick, timestamp, event_type, entity_id, data.clone());

        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                let _ = conn.execute(
                    "INSERT INTO events (event_id, tick, timestamp, event_type, entity_id, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        event_id.to_string(),
                        tick,
                        timestamp,
                        event_type as u32,
                        entity_id.map(|id| id.to_string()),
                        data,
                    ],
                );
                self.total_bytes_written += data.len() as u64;
            }
        }

        if tick - self.last_snapshot_tick >= self.auto_snapshot_interval {
            self.create_snapshot(tick, timestamp, &[]);
        }

        event_id
    }

    pub fn create_snapshot(&mut self, tick: u64, timestamp: f64, data: &[u8]) {
        self.inner.create_snapshot(tick, timestamp, data.to_vec());

        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                let event_index = self.inner.events.len();
                let _ = conn.execute(
                    "INSERT INTO snapshots (tick, timestamp, data, event_index) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![tick, timestamp, data, event_index],
                );
                self.total_bytes_written += data.len() as u64;
            }
        }

        self.last_snapshot_tick = tick;
    }

    pub fn replay_from_snapshot(&self, target_tick: u64, mut handler: impl FnMut(&GameEvent)) {
        let snapshot = self.inner.latest_snapshot_before(target_tick);
        let from_tick = snapshot.map(|s| s.tick).unwrap_or(0);

        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                let mut stmt = conn.prepare(
                    "SELECT event_id, tick, timestamp, event_type, entity_id, data FROM events WHERE tick >= ?1 AND tick <= ?2 ORDER BY tick"
                ).ok();

                if let Some(ref mut stmt) = stmt {
                    let rows = stmt
                        .query_map(rusqlite::params![from_tick, target_tick], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, u64>(1)?,
                                row.get::<_, f64>(2)?,
                                row.get::<_, u32>(3)?,
                                row.get::<_, Option<String>>(4)?,
                                row.get::<_, Vec<u8>>(5)?,
                            ))
                        })
                        .ok();

                    if let Some(rows) = rows {
                        for row in rows.flatten() {
                            let event = GameEvent {
                                event_id: Uuid::parse_str(&row.0)
                                    .unwrap_or_else(|_| Uuid::new_v4()),
                                tick: row.1,
                                timestamp: row.2,
                                event_type: EventType::Custom(row.3),
                                entity_id: row.4.and_then(|s| Uuid::parse_str(&s).ok()),
                                data: row.5,
                            };
                            handler(&event);
                            self.total_bytes_read += event.data.len() as u64;
                        }
                    }
                }
                return;
            }
        }

        self.inner.replay_events(from_tick, target_tick, handler);
    }

    pub fn get_metadata(&self, key: &str) -> Option<String> {
        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                return conn
                    .query_row(
                        "SELECT value FROM world_metadata WHERE key = ?1",
                        rusqlite::params![key],
                        |row| row.get(0),
                    )
                    .ok();
            }
        }
        None
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) {
        if let Some(ref path) = self.db_path {
            if let Ok(conn) = rusqlite::Connection::open(path) {
                let _ = conn.execute(
                    "INSERT OR REPLACE INTO world_metadata (key, value) VALUES (?1, ?2)",
                    rusqlite::params![key, value],
                );
            }
        }
    }

    pub fn event_count(&self) -> usize {
        self.inner.event_count()
    }

    pub fn snapshot_count(&self) -> usize {
        self.inner.snapshot_count()
    }

    pub fn total_events_ever(&self) -> u64 {
        self.inner.total_events_ever
    }

    pub fn stats(&self) -> EventStoreStats {
        EventStoreStats {
            event_count: self.event_count(),
            snapshot_count: self.snapshot_count(),
            total_events_ever: self.total_events_ever(),
            total_bytes_written: self.total_bytes_written,
            total_bytes_read: self.total_bytes_read,
            has_sqlite: self.db_path.is_some(),
        }
    }
}

impl Default for SqliteEventStore {
    fn default() -> Self {
        Self::new(100000)
    }
}

#[derive(Debug, Clone)]
pub struct EventStoreStats {
    pub event_count: usize,
    pub snapshot_count: usize,
    pub total_events_ever: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub has_sqlite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSaveData {
    pub version: u32,
    pub tick: u64,
    pub timestamp: f64,
    pub world_bounds: (f32, f32, f32, f32, f32, f32),
    pub global_temperature: f32,
    pub global_radiation: f32,
    pub seed: u64,
    pub metadata: HashMap<String, String>,
}

impl WorldSaveData {
    pub fn new(
        tick: u64,
        timestamp: f64,
        bounds_min: (f32, f32, f32),
        bounds_max: (f32, f32, f32),
        temperature: f32,
        radiation: f32,
        seed: u64,
    ) -> Self {
        Self {
            version: 1,
            tick,
            timestamp,
            world_bounds: (
                bounds_min.0,
                bounds_min.1,
                bounds_min.2,
                bounds_max.0,
                bounds_max.1,
                bounds_max.2,
            ),
            global_temperature: temperature,
            global_radiation: radiation,
            seed,
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store() {
        let mut store = SqliteEventStore::new(1000);
        let id =
            store.record(0, 0.0, EventType::EntitySpawned, Some(Uuid::new_v4()), vec![1, 2, 3]);
        assert_eq!(store.event_count(), 1);
        assert!(store.stats().event_count == 1);
    }

    #[test]
    fn test_auto_snapshot() {
        let mut store = SqliteEventStore::new(1000);
        store.auto_snapshot_interval = 10;
        for i in 0..15 {
            store.record(i, i as f64, EventType::EntityMoved, None, vec![i as u8]);
        }
        assert!(store.snapshot_count() >= 1);
    }
}
