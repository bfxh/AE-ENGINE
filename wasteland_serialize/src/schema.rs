use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub version: u32,
    pub fields: Vec<FieldDef>,
    pub alignment: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub offset: usize,
    pub size: usize,
    pub optional: bool,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Bytes,
    String,
    Array(u32),
    Struct(u32),
    Enum(u32),
}

pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
    next_id: u32,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        SchemaRegistry { schemas: HashMap::new(), next_id: 0 }
    }

    pub fn register(&mut self, schema: Schema) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.schemas.insert(schema.name.clone(), schema);
        id
    }

    pub fn get(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    pub fn validate(&self, name: &str, data: &[u8]) -> Result<(), String> {
        let schema = self.get(name).ok_or_else(|| format!("unknown schema: {}", name))?;
        let total_size: usize = schema.fields.iter().map(|f| f.size).sum();
        if data.len() < total_size {
            return Err(format!("data too short: expected >= {}, got {}", total_size, data.len()));
        }
        for field in &schema.fields {
            if !field.optional && field.offset + field.size > data.len() {
                return Err(format!("required field '{}' out of bounds", field.name));
            }
        }
        Ok(())
    }

    pub fn generate_layout(&self, name: &str) -> Result<SchemaLayout, String> {
        let schema = self.get(name).ok_or_else(|| format!("unknown schema: {}", name))?;
        let mut layout = SchemaLayout {
            name: schema.name.clone(),
            total_size: 0,
            alignment: schema.alignment,
            fields: Vec::new(),
        };
        let mut offset = 0usize;
        for field in &schema.fields {
            let aligned = offset.div_ceil(field.size) * field.size;
            layout.fields.push(FieldLayout {
                name: field.name.clone(),
                offset: aligned,
                size: field.size,
            });
            offset = aligned + field.size;
        }
        layout.total_size = offset;
        Ok(layout)
    }

    pub fn all_schemas(&self) -> Vec<&Schema> {
        self.schemas.values().collect()
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SchemaLayout {
    pub name: String,
    pub total_size: usize,
    pub alignment: usize,
    pub fields: Vec<FieldLayout>,
}

#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub name: String,
    pub offset: usize,
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_schema() -> Schema {
        Schema {
            name: "Transform".into(),
            version: 1,
            alignment: 4,
            fields: vec![
                FieldDef {
                    name: "x".into(),
                    field_type: FieldType::F32,
                    offset: 0,
                    size: 4,
                    optional: false,
                    deprecated: false,
                },
                FieldDef {
                    name: "y".into(),
                    field_type: FieldType::F32,
                    offset: 4,
                    size: 4,
                    optional: false,
                    deprecated: false,
                },
                FieldDef {
                    name: "z".into(),
                    field_type: FieldType::F32,
                    offset: 8,
                    size: 4,
                    optional: false,
                    deprecated: false,
                },
                FieldDef {
                    name: "id".into(),
                    field_type: FieldType::U32,
                    offset: 12,
                    size: 4,
                    optional: true,
                    deprecated: false,
                },
            ],
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = SchemaRegistry::new();
        reg.register(make_test_schema());
        assert!(reg.get("Transform").is_some());
        assert!(reg.get("Nonexistent").is_none());
    }

    #[test]
    fn test_validate() {
        let mut reg = SchemaRegistry::new();
        reg.register(make_test_schema());
        assert!(reg.validate("Transform", &[0u8; 16]).is_ok());
        assert!(reg.validate("Transform", &[0u8; 8]).is_err());
    }

    #[test]
    fn test_generate_layout() {
        let mut reg = SchemaRegistry::new();
        reg.register(make_test_schema());
        let layout = reg.generate_layout("Transform").unwrap();
        assert_eq!(layout.name, "Transform");
        assert_eq!(layout.fields.len(), 4);
        assert!(layout.total_size >= 16);
    }

    #[test]
    fn test_all_schemas() {
        let mut reg = SchemaRegistry::new();
        reg.register(make_test_schema());
        reg.register(Schema {
            name: "Velocity".into(),
            version: 1,
            alignment: 4,
            fields: vec![FieldDef {
                name: "vx".into(),
                field_type: FieldType::F32,
                offset: 0,
                size: 4,
                optional: false,
                deprecated: false,
            }],
        });
        assert_eq!(reg.all_schemas().len(), 2);
    }
}
