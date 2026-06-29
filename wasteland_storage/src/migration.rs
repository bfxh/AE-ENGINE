use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        SchemaVersion { major, minor, patch }
    }

    pub fn is_compatible(&self, other: &SchemaVersion) -> bool {
        self.major == other.major
    }

    pub fn needs_migration(&self, target: &SchemaVersion) -> bool {
        self < target
    }
}

impl PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.major
                .cmp(&other.major)
                .then(self.minor.cmp(&other.minor))
                .then(self.patch.cmp(&other.patch)),
        )
    }
}

pub type MigrationFn = fn(&[u8]) -> Result<Vec<u8>, String>;

pub struct Migration {
    pub from: SchemaVersion,
    pub to: SchemaVersion,
    pub description: String,
    pub apply: MigrationFn,
}

pub struct MigrationRegistry {
    migrations: HashMap<(SchemaVersion, SchemaVersion), Migration>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        MigrationRegistry { migrations: HashMap::new() }
    }

    pub fn register(&mut self, migration: Migration) {
        let key = (migration.from.clone(), migration.to.clone());
        self.migrations.insert(key, migration);
    }

    pub fn find_path(
        &self,
        from: &SchemaVersion,
        to: &SchemaVersion,
    ) -> Option<Vec<SchemaVersion>> {
        if from == to {
            return Some(vec![]);
        }
        let mut visited = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(from.clone());
        visited.insert(from.clone(), None);
        while let Some(current) = queue.pop_front() {
            if &current == to {
                let mut path = Vec::new();
                let mut node = to;
                loop {
                    path.push(node.clone());
                    match visited.get(node) {
                        Some(Some(prev)) => node = prev,
                        _ => break,
                    }
                }
                path.reverse();
                return Some(path);
            }
            for (s, t) in self.migrations.keys() {
                if s == &current && !visited.contains_key(t) {
                    visited.insert(t.clone(), Some(current.clone()));
                    queue.push_back(t.clone());
                }
            }
        }
        None
    }

    pub fn migrate(
        &self,
        data: &[u8],
        from: &SchemaVersion,
        to: &SchemaVersion,
    ) -> Result<Vec<u8>, String> {
        let path = self
            .find_path(from, to)
            .ok_or_else(|| format!("no migration path from {:?} to {:?}", from, to))?;
        let mut current = data.to_vec();
        for i in 0..path.len().saturating_sub(1) {
            let from_v = &path[i];
            let to_v = &path[i + 1];
            let key = (from_v.clone(), to_v.clone());
            let migration = self
                .migrations
                .get(&key)
                .ok_or_else(|| format!("missing migration {:?} -> {:?}", from_v, to_v))?;
            current = (migration.apply)(&current)?;
        }
        Ok(current)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 5, 0);
        let v3 = SchemaVersion::new(2, 0, 0);
        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 5, 0);
        let v3 = SchemaVersion::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_path_finding_direct() {
        let mut registry = MigrationRegistry::new();
        let from = SchemaVersion::new(1, 0, 0);
        let to = SchemaVersion::new(1, 1, 0);
        registry.register(Migration {
            from: from.clone(),
            to: to.clone(),
            description: "test".into(),
            apply: |data| Ok(data.to_vec()),
        });
        let path = registry.find_path(&from, &to).unwrap();
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_path_finding_same_version() {
        let registry = MigrationRegistry::new();
        let v = SchemaVersion::new(1, 0, 0);
        let path = registry.find_path(&v, &v).unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn test_path_finding_no_path() {
        let registry = MigrationRegistry::new();
        let from = SchemaVersion::new(1, 0, 0);
        let to = SchemaVersion::new(2, 0, 0);
        assert!(registry.find_path(&from, &to).is_none());
    }

    #[test]
    fn test_migrate_chain() {
        let mut registry = MigrationRegistry::new();
        let v1 = SchemaVersion::new(1, 0, 0);
        let v2 = SchemaVersion::new(1, 1, 0);
        let v3 = SchemaVersion::new(1, 2, 0);
        registry.register(Migration {
            from: v1.clone(),
            to: v2.clone(),
            description: "step1".into(),
            apply: |data| {
                let mut d = data.to_vec();
                d.push(1);
                Ok(d)
            },
        });
        registry.register(Migration {
            from: v2.clone(),
            to: v3.clone(),
            description: "step2".into(),
            apply: |data| {
                let mut d = data.to_vec();
                d.push(2);
                Ok(d)
            },
        });
        let result = registry.migrate(&[0], &v1, &v3).unwrap();
        assert_eq!(result, vec![0, 1, 2]);
    }
}
