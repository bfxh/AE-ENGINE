use glam::{Quat, Vec3};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wasteland_metaentity::prelude::*;

// ============================================================
// AssetFormat
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetFormat {
    OBJ,
    GLTF,
    FBX,
    BLUEPRINT,
    VOXEL,
    HEIGHTMAP,
}

impl AssetFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "obj" => Some(Self::OBJ),
            "gltf" | "glb" => Some(Self::GLTF),
            "fbx" => Some(Self::FBX),
            "blueprint" | "json" => Some(Self::BLUEPRINT),
            "vox" | "voxel" => Some(Self::VOXEL),
            "png" | "raw" | "heightmap" | "hgt" => Some(Self::HEIGHTMAP),
            _ => None,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::OBJ => "obj",
            Self::GLTF => "gltf",
            Self::FBX => "fbx",
            Self::BLUEPRINT => "blueprint",
            Self::VOXEL => "vox",
            Self::HEIGHTMAP => "heightmap",
        }
    }
}

// ============================================================
// ImportError / ExportError
// ============================================================

#[derive(Debug, Clone)]
pub enum ImportError {
    UnsupportedFormat(AssetFormat),
    ParseError(String),
    InvalidData(String),
    VersionMismatch { expected: u32, actual: u32 },
    MissingDependency(String),
    IoError(String),
    CacheError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFormat(fmt) => write!(f, "unsupported format: {:?}", fmt),
            Self::ParseError(msg) => write!(f, "parse error: {}", msg),
            Self::InvalidData(msg) => write!(f, "invalid data: {}", msg),
            Self::VersionMismatch { expected, actual } => {
                write!(f, "version mismatch: expected {}, got {}", expected, actual)
            },
            Self::MissingDependency(dep) => write!(f, "missing dependency: {}", dep),
            Self::IoError(msg) => write!(f, "io error: {}", msg),
            Self::CacheError(msg) => write!(f, "cache error: {}", msg),
        }
    }
}

impl std::error::Error for ImportError {}

#[derive(Debug, Clone)]
pub enum ExportError {
    SerializationError(String),
    UnsupportedFormat(AssetFormat),
    EmptyEntitySet,
    VersionConflict(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "serialization error: {}", msg),
            Self::UnsupportedFormat(fmt) => write!(f, "unsupported format: {:?}", fmt),
            Self::EmptyEntitySet => write!(f, "empty entity set"),
            Self::VersionConflict(msg) => write!(f, "version conflict: {}", msg),
        }
    }
}

impl std::error::Error for ExportError {}

// ============================================================
// AssetManifest
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    pub name: String,
    pub format: AssetFormat,
    pub version: u32,
    pub entities_count: u32,
    pub dependencies: Vec<String>,
    pub author: String,
    pub created_at: u64,
    pub source_path: String,
    pub checksum: u64,
    pub metadata: HashMap<String, String>,
}

impl AssetManifest {
    pub fn new(name: &str, format: AssetFormat, version: u32) -> Self {
        Self {
            name: name.to_string(),
            format,
            version,
            entities_count: 0,
            dependencies: Vec::new(),
            author: String::new(),
            created_at: 0,
            source_path: String::new(),
            checksum: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.format == other.format && self.version == other.version
    }

    pub fn add_dependency(&mut self, dep: &str) {
        if !self.dependencies.contains(&dep.to_string()) {
            self.dependencies.push(dep.to_string());
        }
    }
}

// ============================================================
// AssetImporter trait
// ============================================================

pub trait AssetImporter: Send + Sync {
    fn supported_formats(&self) -> Vec<AssetFormat>;
    fn import(&self, data: &[u8], tick: u64) -> Result<Vec<MetaEntity>, ImportError>;
    fn can_import(&self, format: AssetFormat) -> bool {
        self.supported_formats().contains(&format)
    }
}

// ============================================================
// ObjImporter
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct ObjImporter {
    pub scale: f32,
    pub default_material: String,
}

impl ObjImporter {
    pub fn new() -> Self {
        Self { scale: 1.0, default_material: "default".into() }
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    fn parse_vertex(line: &str) -> Option<Vec3> {
        let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
        if parts.len() < 3 {
            return None;
        }
        let x: f32 = parts[0].parse().ok()?;
        let y: f32 = parts[1].parse().ok()?;
        let z: f32 = parts[2].parse().ok()?;
        Some(Vec3::new(x, y, z))
    }

    fn parse_normal(line: &str) -> Option<Vec3> {
        let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
        if parts.len() < 3 {
            return None;
        }
        let x: f32 = parts[0].parse().ok()?;
        let y: f32 = parts[1].parse().ok()?;
        let z: f32 = parts[2].parse().ok()?;
        Some(Vec3::new(x, y, z))
    }

    fn parse_face_vertex(token: &str) -> Option<usize> {
        let v_idx = token.split('/').next()?;
        v_idx
            .parse::<isize>()
            .ok()
            .map(|i| if i > 0 { (i - 1) as usize } else { (i.abs() - 1) as usize })
    }

    fn parse_face(line: &str, vertices: &[Vec3], normals: &[Vec3]) -> Option<(Vec3, Vec3)> {
        let parts: Vec<&str> = line.split_whitespace().skip(1).collect();
        if parts.len() < 3 {
            return None;
        }

        let indices: Vec<usize> = parts.iter().filter_map(|t| Self::parse_face_vertex(t)).collect();
        let vert_indices: Vec<usize> =
            parts.iter().filter_map(|t| Self::parse_face_vertex(t)).collect();

        if vert_indices.len() < 3 {
            return None;
        }

        let v0 = vertices.get(vert_indices[0])?;
        let v1 = vertices.get(vert_indices[1])?;
        let v2 = vertices.get(vert_indices[2])?;

        let center = (*v0 + *v1 + *v2) / 3.0;

        let computed_normal = (*v1 - *v0).cross(*v2 - *v0).normalize_or_zero();
        let normal = if indices.len() >= 3 {
            let norm_idx = parts[0]
                .split('/')
                .nth(2)
                .and_then(|n| n.parse::<isize>().ok())
                .map(|i| if i > 0 { (i - 1) as usize } else { (i.abs() - 1) as usize });
            match norm_idx.and_then(|i| normals.get(i)) {
                Some(n) => *n,
                None => computed_normal,
            }
        } else {
            computed_normal
        };

        Some((center, normal))
    }
}

impl AssetImporter for ObjImporter {
    fn supported_formats(&self) -> Vec<AssetFormat> {
        vec![AssetFormat::OBJ]
    }

    fn import(&self, data: &[u8], tick: u64) -> Result<Vec<MetaEntity>, ImportError> {
        let content = std::str::from_utf8(data)
            .map_err(|e| ImportError::ParseError(format!("invalid UTF-8: {}", e)))?;

        let mut vertices: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut faces: Vec<(Vec3, Vec3)> = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with("vn ") {
                if let Some(n) = Self::parse_normal(trimmed) {
                    normals.push(n);
                }
            } else if trimmed.starts_with("v ") {
                if let Some(v) = Self::parse_vertex(trimmed) {
                    vertices.push(v * self.scale);
                }
            } else if trimmed.starts_with("f ") {
                if let Some(face) = Self::parse_face(trimmed, &vertices, &normals) {
                    faces.push(face);
                }
            }
        }

        if vertices.is_empty() {
            return Err(ImportError::ParseError("no vertices found".into()));
        }

        let mut entities = Vec::with_capacity(faces.len().max(1));

        if faces.is_empty() {
            let centroid = vertices.iter().sum::<Vec3>() / vertices.len() as f32;
            let entity = MetaEntity::new(centroid, PhysicsAttributes::default(), tick);
            entities.push(entity);
        } else {
            for (center, _normal) in &faces {
                let rotation = Quat::IDENTITY;
                let mut entity = MetaEntity::new(*center, PhysicsAttributes::default(), tick);
                entity.rotation = rotation;
                entity.set_extension("source_format", ExtensionValue::String("obj".into()));
                entity.set_extension(
                    "material",
                    ExtensionValue::String(self.default_material.clone()),
                );
                entities.push(entity);
            }
        }

        Ok(entities)
    }
}

// ============================================================
// BlueprintImporter
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintAsset {
    pub name: String,
    pub version: u32,
    pub entities: Vec<BlueprintEntityDef>,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintEntityDef {
    pub id: Option<Uuid>,
    pub position: [f32; 3],
    pub rotation: Option<[f32; 4]>,
    pub velocity: Option<[f32; 3]>,
    pub physics: Option<BlueprintPhysicsDef>,
    pub chemistry: Option<BlueprintChemistryDef>,
    pub biology: Option<BlueprintBiologyDef>,
    pub function_tags: Vec<String>,
    pub parent_id: Option<Uuid>,
    pub extensions: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintPhysicsDef {
    pub mass: Option<f32>,
    pub density: Option<f32>,
    pub hardness: Option<f32>,
    pub toughness: Option<f32>,
    pub elastic_modulus: Option<f32>,
    pub yield_strength: Option<f32>,
    pub ultimate_strength: Option<f32>,
    pub poisson_ratio: Option<f32>,
    pub friction_coefficient: Option<f32>,
    pub restitution: Option<f32>,
    pub temperature: Option<f32>,
    pub thermal_conductivity: Option<f32>,
    pub specific_heat_capacity: Option<f32>,
    pub electrical_conductivity: Option<f32>,
    pub magnetic_permeability: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintChemistryDef {
    pub elements: Vec<BlueprintElementDef>,
    pub bond_types: Vec<String>,
    pub reactivity: Option<f32>,
    pub ph: Option<f32>,
    pub redox_potential: Option<f32>,
    pub oxidation_state: Option<f32>,
    pub corrosion_depth: Option<f32>,
    pub chemical_stain: Option<u8>,
    pub solubility: Option<f32>,
    pub flammability: Option<f32>,
    pub toxicity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintElementDef {
    pub element: String,
    pub fraction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintBiologyDef {
    pub gene_tokens: Vec<BlueprintGeneTokenDef>,
    pub metabolic_rate: Option<f32>,
    pub growth_rate: Option<f32>,
    pub repair_rate: Option<f32>,
    pub neural_signal_strength: Option<f32>,
    pub health: Option<f32>,
    pub max_health: Option<f32>,
    pub cell_type: Option<String>,
    pub tissue_density: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintGeneTokenDef {
    pub name: String,
    pub expression_level: f32,
    pub dominant: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BlueprintImporter;

impl BlueprintImporter {
    pub fn new() -> Self {
        Self
    }

    fn parse_element(s: &str) -> Option<Element> {
        match s.to_lowercase().as_str() {
            "h" => Some(Element::H),
            "he" => Some(Element::He),
            "li" => Some(Element::Li),
            "be" => Some(Element::Be),
            "b" => Some(Element::B),
            "c" => Some(Element::C),
            "n" => Some(Element::N),
            "o" => Some(Element::O),
            "f" => Some(Element::F),
            "ne" => Some(Element::Ne),
            "na" => Some(Element::Na),
            "mg" => Some(Element::Mg),
            "al" => Some(Element::Al),
            "si" => Some(Element::Si),
            "p" => Some(Element::P),
            "s" => Some(Element::S),
            "cl" => Some(Element::Cl),
            "ar" => Some(Element::Ar),
            "k" => Some(Element::K),
            "ca" => Some(Element::Ca),
            "sc" => Some(Element::Sc),
            "ti" => Some(Element::Ti),
            "v" => Some(Element::V),
            "cr" => Some(Element::Cr),
            "mn" => Some(Element::Mn),
            "fe" => Some(Element::Fe),
            "co" => Some(Element::Co),
            "ni" => Some(Element::Ni),
            "cu" => Some(Element::Cu),
            "zn" => Some(Element::Zn),
            "ga" => Some(Element::Ga),
            "ge" => Some(Element::Ge),
            "as" => Some(Element::As),
            "se" => Some(Element::Se),
            "br" => Some(Element::Br),
            "kr" => Some(Element::Kr),
            "rb" => Some(Element::Rb),
            "sr" => Some(Element::Sr),
            "y" => Some(Element::Y),
            "zr" => Some(Element::Zr),
            "nb" => Some(Element::Nb),
            "mo" => Some(Element::Mo),
            "tc" => Some(Element::Tc),
            "ru" => Some(Element::Ru),
            "rh" => Some(Element::Rh),
            "pd" => Some(Element::Pd),
            "ag" => Some(Element::Ag),
            "cd" => Some(Element::Cd),
            "in" => Some(Element::In),
            "sn" => Some(Element::Sn),
            "sb" => Some(Element::Sb),
            "te" => Some(Element::Te),
            "i" => Some(Element::I),
            "xe" => Some(Element::Xe),
            "cs" => Some(Element::Cs),
            "ba" => Some(Element::Ba),
            "la" => Some(Element::La),
            "ce" => Some(Element::Ce),
            "pr" => Some(Element::Pr),
            "nd" => Some(Element::Nd),
            "pm" => Some(Element::Pm),
            "sm" => Some(Element::Sm),
            "eu" => Some(Element::Eu),
            "gd" => Some(Element::Gd),
            "tb" => Some(Element::Tb),
            "dy" => Some(Element::Dy),
            "ho" => Some(Element::Ho),
            "er" => Some(Element::Er),
            "tm" => Some(Element::Tm),
            "yb" => Some(Element::Yb),
            "lu" => Some(Element::Lu),
            "hf" => Some(Element::Hf),
            "ta" => Some(Element::Ta),
            "w" => Some(Element::W),
            "re" => Some(Element::Re),
            "os" => Some(Element::Os),
            "ir" => Some(Element::Ir),
            "pt" => Some(Element::Pt),
            "au" => Some(Element::Au),
            "hg" => Some(Element::Hg),
            "tl" => Some(Element::Tl),
            "pb" => Some(Element::Pb),
            "bi" => Some(Element::Bi),
            "po" => Some(Element::Po),
            "at" => Some(Element::At),
            "rn" => Some(Element::Rn),
            "fr" => Some(Element::Fr),
            "ra" => Some(Element::Ra),
            "ac" => Some(Element::Ac),
            "th" => Some(Element::Th),
            "pa" => Some(Element::Pa),
            "u" => Some(Element::U),
            _ => None,
        }
    }

    fn parse_bond(s: &str) -> Option<ChemicalBond> {
        match s.to_lowercase().as_str() {
            "ionic" => Some(ChemicalBond::Ionic),
            "covalent" => Some(ChemicalBond::Covalent),
            "metallic" => Some(ChemicalBond::Metallic),
            "hydrogen" => Some(ChemicalBond::Hydrogen),
            "van_der_waals" | "vanderwaals" => Some(ChemicalBond::VanDerWaals),
            "pi" | "pi_bond" => Some(ChemicalBond::PiBond),
            "sigma" | "sigma_bond" => Some(ChemicalBond::SigmaBond),
            _ => None,
        }
    }

    fn parse_cell_type(s: &str) -> CellType {
        match s.to_lowercase().as_str() {
            "prokaryotic" => CellType::Prokaryotic,
            "eukaryoticanimal" | "eukaryotic_animal" | "animal" => CellType::EukaryoticAnimal,
            "eukaryoticplant" | "eukaryotic_plant" | "plant" => CellType::EukaryoticPlant,
            "eukaryoticfungal" | "eukaryotic_fungal" | "fungal" => CellType::EukaryoticFungal,
            "synthetic" => CellType::Synthetic,
            "mycelial" => CellType::Mycelial,
            _ => CellType::Undefined,
        }
    }

    #[allow(clippy::field_reassign_with_default)]
    fn build_entity(def: &BlueprintEntityDef, tick: u64) -> MetaEntity {
        let position = Vec3::new(def.position[0], def.position[1], def.position[2]);

        let mut physics = PhysicsAttributes::default();
        if let Some(ref p) = def.physics {
            if let Some(v) = p.mass {
                physics.mass = v;
            }
            if let Some(v) = p.density {
                physics.density = v;
            }
            if let Some(v) = p.hardness {
                physics.hardness = v;
            }
            if let Some(v) = p.toughness {
                physics.toughness = v;
            }
            if let Some(v) = p.elastic_modulus {
                physics.elastic_modulus = v;
            }
            if let Some(v) = p.yield_strength {
                physics.yield_strength = v;
            }
            if let Some(v) = p.ultimate_strength {
                physics.ultimate_strength = v;
            }
            if let Some(v) = p.poisson_ratio {
                physics.poisson_ratio = v;
            }
            if let Some(v) = p.friction_coefficient {
                physics.friction_coefficient = v;
            }
            if let Some(v) = p.restitution {
                physics.restitution = v;
            }
            if let Some(v) = p.temperature {
                physics.temperature = v;
            }
            if let Some(v) = p.thermal_conductivity {
                physics.thermal_conductivity = v;
            }
            if let Some(v) = p.specific_heat_capacity {
                physics.specific_heat_capacity = v;
            }
            if let Some(v) = p.electrical_conductivity {
                physics.electrical_conductivity = v;
            }
            if let Some(v) = p.magnetic_permeability {
                physics.magnetic_permeability = v;
            }
        }

        let mut entity = MetaEntity::new(position, physics, tick);

        if let Some(rot) = def.rotation {
            entity.rotation = Quat::from_xyzw(rot[0], rot[1], rot[2], rot[3]);
        }

        if let Some(vel) = def.velocity {
            entity.velocity = Vec3::new(vel[0], vel[1], vel[2]);
        }

        if let Some(ref c) = def.chemistry {
            let elemental_composition: Vec<ElementFraction> = c
                .elements
                .iter()
                .filter_map(|e| {
                    Self::parse_element(&e.element)
                        .map(|el| ElementFraction { element: el, fraction: e.fraction })
                })
                .collect();

            let bond_types: Vec<ChemicalBond> =
                c.bond_types.iter().filter_map(|b| Self::parse_bond(b)).collect();

            let mut chem = ChemistryAttributes::default();
            chem.elemental_composition = elemental_composition;
            chem.bond_types = bond_types;
            if let Some(v) = c.reactivity {
                chem.reactivity = v;
            }
            if let Some(v) = c.ph {
                chem.ph = v;
            }
            if let Some(v) = c.redox_potential {
                chem.redox_potential = v;
            }
            if let Some(v) = c.oxidation_state {
                chem.oxidation_state = v;
            }
            if let Some(v) = c.corrosion_depth {
                chem.corrosion_depth = v;
            }
            if let Some(v) = c.chemical_stain {
                chem.chemical_stain = v;
            }
            if let Some(v) = c.solubility {
                chem.solubility = v;
            }
            if let Some(v) = c.flammability {
                chem.flammability = v;
            }
            if let Some(v) = c.toxicity {
                chem.toxicity = v;
            }
            entity.chemistry = chem;
        }

        if let Some(ref b) = def.biology {
            let gene_tokens: Vec<GeneToken> = b
                .gene_tokens
                .iter()
                .map(|g| GeneToken {
                    name: g.name.clone(),
                    expression_level: g.expression_level,
                    mutation_state: MutationState::Normal,
                    epigenetic_markers: Vec::new(),
                    dominant: g.dominant,
                })
                .collect();

            let mut bio = BiologyAttributes::default();
            bio.gene_tokens = gene_tokens;
            if let Some(v) = b.metabolic_rate {
                bio.metabolic_rate = v;
            }
            if let Some(v) = b.growth_rate {
                bio.growth_rate = v;
            }
            if let Some(v) = b.repair_rate {
                bio.repair_rate = v;
            }
            if let Some(v) = b.neural_signal_strength {
                bio.neural_signal_strength = v;
            }
            if let Some(v) = b.health {
                bio.health = v;
            }
            if let Some(v) = b.max_health {
                bio.max_health = v;
            }
            if let Some(v) = b.tissue_density {
                bio.tissue_density = v;
            }
            if let Some(ref ct) = b.cell_type {
                bio.cell_type = Self::parse_cell_type(ct);
            }
            entity.biology = bio;
        }

        if let Some(id) = def.id {
            entity.id = id;
        }

        if let Some(pid) = def.parent_id {
            entity.parent_id = Some(pid);
        }

        if !def.function_tags.is_empty() {
            let tags = def.function_tags.join(",");
            entity.set_extension("function_tags", ExtensionValue::String(tags));
        }

        for (key, val) in &def.extensions {
            match val {
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        entity.set_extension(key, ExtensionValue::Float(f as f32));
                    } else if let Some(i) = n.as_i64() {
                        entity.set_extension(key, ExtensionValue::Int(i));
                    }
                },
                serde_json::Value::String(s) => {
                    entity.set_extension(key, ExtensionValue::String(s.clone()));
                },
                serde_json::Value::Bool(b) => {
                    entity.set_extension(key, ExtensionValue::Bool(*b));
                },
                _ => {},
            }
        }

        entity
    }
}

impl AssetImporter for BlueprintImporter {
    fn supported_formats(&self) -> Vec<AssetFormat> {
        vec![AssetFormat::BLUEPRINT]
    }

    fn import(&self, data: &[u8], tick: u64) -> Result<Vec<MetaEntity>, ImportError> {
        let blueprint: BlueprintAsset = serde_json::from_slice(data)
            .map_err(|e| ImportError::ParseError(format!("JSON parse error: {}", e)))?;

        if blueprint.entities.is_empty() {
            return Err(ImportError::InvalidData("blueprint has no entities".into()));
        }

        let entities: Vec<MetaEntity> =
            blueprint.entities.iter().map(|def| Self::build_entity(def, tick)).collect();

        Ok(entities)
    }
}

// ============================================================
// AssetExporter trait
// ============================================================

pub trait AssetExporter: Send + Sync {
    fn supported_formats(&self) -> Vec<AssetFormat>;
    fn export(&self, entities: &[MetaEntity]) -> Result<Vec<u8>, ExportError>;
    fn can_export(&self, format: AssetFormat) -> bool {
        self.supported_formats().contains(&format)
    }
}

// ============================================================
// BlueprintExporter
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct BlueprintExporter {
    pub name: String,
    pub version: u32,
    pub pretty_print: bool,
}

impl BlueprintExporter {
    pub fn new(name: &str, version: u32) -> Self {
        Self { name: name.to_string(), version, pretty_print: true }
    }

    fn entity_to_def(entity: &MetaEntity) -> BlueprintEntityDef {
        let function_tags: Vec<String> = entity
            .get_extension("function_tags")
            .and_then(|v| {
                if let ExtensionValue::String(s) = v {
                    Some(s.split(',').map(|s| s.trim().to_string()).collect())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let extensions: HashMap<String, serde_json::Value> = entity
            .extensions
            .iter()
            .filter_map(|(k, v)| {
                let json_val = match v {
                    ExtensionValue::Float(f) => serde_json::Value::Number(
                        serde_json::Number::from_f64(*f as f64)
                            .unwrap_or(serde_json::Number::from(0)),
                    ),
                    ExtensionValue::Int(i) => {
                        serde_json::Value::Number(serde_json::Number::from(*i))
                    },
                    ExtensionValue::String(s) => serde_json::Value::String(s.clone()),
                    ExtensionValue::Bool(b) => serde_json::Value::Bool(*b),
                    _ => return None,
                };
                Some((k.clone(), json_val))
            })
            .collect();

        let physics = BlueprintPhysicsDef {
            mass: Some(entity.physics.mass),
            density: Some(entity.physics.density),
            hardness: Some(entity.physics.hardness),
            toughness: Some(entity.physics.toughness),
            elastic_modulus: Some(entity.physics.elastic_modulus),
            yield_strength: Some(entity.physics.yield_strength),
            ultimate_strength: Some(entity.physics.ultimate_strength),
            poisson_ratio: Some(entity.physics.poisson_ratio),
            friction_coefficient: Some(entity.physics.friction_coefficient),
            restitution: Some(entity.physics.restitution),
            temperature: Some(entity.physics.temperature),
            thermal_conductivity: Some(entity.physics.thermal_conductivity),
            specific_heat_capacity: Some(entity.physics.specific_heat_capacity),
            electrical_conductivity: Some(entity.physics.electrical_conductivity),
            magnetic_permeability: Some(entity.physics.magnetic_permeability),
        };

        let chemistry = BlueprintChemistryDef {
            elements: entity
                .chemistry
                .elemental_composition
                .iter()
                .map(|e| BlueprintElementDef {
                    element: format!("{:?}", e.element),
                    fraction: e.fraction,
                })
                .collect(),
            bond_types: entity.chemistry.bond_types.iter().map(|b| format!("{:?}", b)).collect(),
            reactivity: Some(entity.chemistry.reactivity),
            ph: Some(entity.chemistry.ph),
            redox_potential: Some(entity.chemistry.redox_potential),
            oxidation_state: Some(entity.chemistry.oxidation_state),
            corrosion_depth: Some(entity.chemistry.corrosion_depth),
            chemical_stain: Some(entity.chemistry.chemical_stain),
            solubility: Some(entity.chemistry.solubility),
            flammability: Some(entity.chemistry.flammability),
            toxicity: Some(entity.chemistry.toxicity),
        };

        let biology = BlueprintBiologyDef {
            gene_tokens: entity
                .biology
                .gene_tokens
                .iter()
                .map(|g| BlueprintGeneTokenDef {
                    name: g.name.clone(),
                    expression_level: g.expression_level,
                    dominant: g.dominant,
                })
                .collect(),
            metabolic_rate: Some(entity.biology.metabolic_rate),
            growth_rate: Some(entity.biology.growth_rate),
            repair_rate: Some(entity.biology.repair_rate),
            neural_signal_strength: Some(entity.biology.neural_signal_strength),
            health: Some(entity.biology.health),
            max_health: Some(entity.biology.max_health),
            cell_type: Some(format!("{:?}", entity.biology.cell_type)),
            tissue_density: Some(entity.biology.tissue_density),
        };

        BlueprintEntityDef {
            id: Some(entity.id),
            position: [entity.position.x, entity.position.y, entity.position.z],
            rotation: Some([
                entity.rotation.x,
                entity.rotation.y,
                entity.rotation.z,
                entity.rotation.w,
            ]),
            velocity: Some([entity.velocity.x, entity.velocity.y, entity.velocity.z]),
            physics: Some(physics),
            chemistry: Some(chemistry),
            biology: Some(biology),
            function_tags,
            parent_id: entity.parent_id,
            extensions,
        }
    }
}

impl AssetExporter for BlueprintExporter {
    fn supported_formats(&self) -> Vec<AssetFormat> {
        vec![AssetFormat::BLUEPRINT]
    }

    fn export(&self, entities: &[MetaEntity]) -> Result<Vec<u8>, ExportError> {
        if entities.is_empty() {
            return Err(ExportError::EmptyEntitySet);
        }

        let entity_defs: Vec<BlueprintEntityDef> =
            entities.iter().map(Self::entity_to_def).collect();

        let blueprint = BlueprintAsset {
            name: self.name.clone(),
            version: self.version,
            entities: entity_defs,
            dependencies: Vec::new(),
            metadata: HashMap::new(),
        };

        let json = if self.pretty_print {
            serde_json::to_vec_pretty(&blueprint)
        } else {
            serde_json::to_vec(&blueprint)
        }
        .map_err(|e| ExportError::SerializationError(format!("JSON serialization: {}", e)))?;

        Ok(json)
    }
}

// ============================================================
// AssetCache
// ============================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CacheEntry {
    data: Vec<MetaEntity>,
    format: AssetFormat,
    version: u32,
    last_access: u64,
    access_count: u64,
    key_hash: u64,
}

#[derive(Debug)]
pub struct AssetCache {
    entries: HashMap<u64, CacheEntry>,
    access_order: Vec<u64>,
    max_entries: usize,
    current_tick: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl AssetCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            access_order: Vec::with_capacity(max_entries),
            max_entries: max_entries.max(1),
            current_tick: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    pub fn get(&mut self, key: &str, format: AssetFormat, version: u32) -> Option<Vec<MetaEntity>> {
        let hash = Self::compute_key_hash(key, format, version);
        self.current_tick += 1;

        if let Some(entry) = self.entries.get_mut(&hash) {
            self.hits += 1;
            entry.last_access = self.current_tick;
            entry.access_count += 1;
            self.access_order.retain(|h| *h != hash);
            self.access_order.push(hash);
            Some(entry.data.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, key: &str, format: AssetFormat, version: u32, data: Vec<MetaEntity>) {
        let hash = Self::compute_key_hash(key, format, version);
        self.current_tick += 1;

        if self.entries.contains_key(&hash) {
            if let Some(entry) = self.entries.get_mut(&hash) {
                entry.data = data;
                entry.last_access = self.current_tick;
                entry.access_count += 1;
            }
            self.access_order.retain(|h| *h != hash);
            self.access_order.push(hash);
            return;
        }

        while self.entries.len() >= self.max_entries {
            self.evict_lru();
        }

        self.entries.insert(
            hash,
            CacheEntry {
                data,
                format,
                version,
                last_access: self.current_tick,
                access_count: 1,
                key_hash: hash,
            },
        );
        self.access_order.push(hash);
    }

    pub fn contains(&self, key: &str, format: AssetFormat, version: u32) -> bool {
        let hash = Self::compute_key_hash(key, format, version);
        self.entries.contains_key(&hash)
    }

    pub fn remove(&mut self, key: &str, format: AssetFormat, version: u32) {
        let hash = Self::compute_key_hash(key, format, version);
        self.entries.remove(&hash);
        self.access_order.retain(|h| *h != hash);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }

    fn evict_lru(&mut self) {
        if let Some(oldest) = self.access_order.first().copied() {
            self.entries.remove(&oldest);
            self.access_order.remove(0);
            self.evictions += 1;
        }
    }

    fn compute_key_hash(key: &str, format: AssetFormat, version: u32) -> u64 {
        let mut hash: u64 = 0x9E3779B97F4A7C15;
        for byte in key.bytes() {
            hash = hash.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(byte as u64);
        }
        hash ^= (format as u64).wrapping_mul(0xC6A4A7935BD1E995);
        hash ^= (version as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        hash
    }

    pub fn stats(&self) -> AssetCacheStats {
        AssetCacheStats {
            entries: self.entries.len(),
            max_entries: self.max_entries,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssetCacheStats {
    pub entries: usize,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}

// ============================================================
// AssetPipeline
// ============================================================

pub struct AssetPipeline {
    pub importers: Vec<Box<dyn AssetImporter>>,
    pub exporters: Vec<Box<dyn AssetExporter>>,
    pub cache: AssetCache,
    pub version_registry: HashMap<String, u32>,
    pub supported_versions: HashMap<AssetFormat, Vec<u32>>,
    pub stats: PipelineStats,
}

impl std::fmt::Debug for AssetPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetPipeline")
            .field("importers", &self.importers.len())
            .field("exporters", &self.exporters.len())
            .field("cache", &self.cache)
            .field("version_registry", &self.version_registry)
            .field("supported_versions", &self.supported_versions)
            .field("stats", &self.stats)
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub total_imports: u64,
    pub total_exports: u64,
    pub failed_imports: u64,
    pub failed_exports: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl AssetPipeline {
    pub fn new(cache_size: usize) -> Self {
        Self {
            importers: Vec::new(),
            exporters: Vec::new(),
            cache: AssetCache::new(cache_size),
            version_registry: HashMap::new(),
            supported_versions: HashMap::new(),
            stats: PipelineStats::default(),
        }
    }

    pub fn register_importer(&mut self, importer: Box<dyn AssetImporter>) {
        for format in importer.supported_formats() {
            let entry = self.supported_versions.entry(format).or_default();
            if !entry.contains(&1) {
                entry.push(1);
            }
        }
        self.importers.push(importer);
    }

    pub fn register_exporter(&mut self, exporter: Box<dyn AssetExporter>) {
        self.exporters.push(exporter);
    }

    pub fn register_version(&mut self, format: AssetFormat, version: u32) {
        let entry = self.supported_versions.entry(format).or_default();
        if !entry.contains(&version) {
            entry.push(version);
        }
        entry.sort();
    }

    pub fn is_version_supported(&self, format: AssetFormat, version: u32) -> bool {
        self.supported_versions
            .get(&format)
            .map(|versions| versions.contains(&version))
            .unwrap_or(false)
    }

    pub fn latest_version(&self, format: AssetFormat) -> Option<u32> {
        self.supported_versions.get(&format).and_then(|versions| versions.last().copied())
    }

    pub fn find_importer(&self, format: AssetFormat) -> Option<&dyn AssetImporter> {
        self.importers.iter().find(|i| i.can_import(format)).map(|i| i.as_ref())
    }

    pub fn find_exporter(&self, format: AssetFormat) -> Option<&dyn AssetExporter> {
        self.exporters.iter().find(|e| e.can_export(format)).map(|e| e.as_ref())
    }

    pub fn import(
        &mut self,
        key: &str,
        format: AssetFormat,
        data: &[u8],
        tick: u64,
    ) -> Result<Vec<MetaEntity>, ImportError> {
        let version = self.latest_version(format).unwrap_or(1);

        if let Some(cached) = self.cache.get(key, format, version) {
            self.stats.cache_hits += 1;
            return Ok(cached);
        }
        self.stats.cache_misses += 1;

        let importer = self.find_importer(format).ok_or(ImportError::UnsupportedFormat(format))?;

        match importer.import(data, tick) {
            Ok(entities) => {
                self.stats.total_imports += 1;
                self.cache.insert(key, format, version, entities.clone());
                Ok(entities)
            },
            Err(e) => {
                self.stats.failed_imports += 1;
                Err(e)
            },
        }
    }

    pub fn import_with_version(
        &mut self,
        key: &str,
        format: AssetFormat,
        version: u32,
        data: &[u8],
        tick: u64,
    ) -> Result<Vec<MetaEntity>, ImportError> {
        if !self.is_version_supported(format, version) {
            return Err(ImportError::VersionMismatch {
                expected: self.latest_version(format).unwrap_or(1),
                actual: version,
            });
        }

        if let Some(cached) = self.cache.get(key, format, version) {
            self.stats.cache_hits += 1;
            return Ok(cached);
        }
        self.stats.cache_misses += 1;

        let importer = self.find_importer(format).ok_or(ImportError::UnsupportedFormat(format))?;

        match importer.import(data, tick) {
            Ok(entities) => {
                self.stats.total_imports += 1;
                self.cache.insert(key, format, version, entities.clone());
                Ok(entities)
            },
            Err(e) => {
                self.stats.failed_imports += 1;
                Err(e)
            },
        }
    }

    pub fn export(
        &mut self,
        format: AssetFormat,
        entities: &[MetaEntity],
    ) -> Result<Vec<u8>, ExportError> {
        let exporter = self.find_exporter(format).ok_or(ExportError::UnsupportedFormat(format))?;

        match exporter.export(entities) {
            Ok(data) => {
                self.stats.total_exports += 1;
                Ok(data)
            },
            Err(e) => {
                self.stats.failed_exports += 1;
                Err(e)
            },
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_stats(&self) -> AssetCacheStats {
        self.cache.stats()
    }
}

impl Default for AssetPipeline {
    fn default() -> Self {
        Self::new(256)
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- AssetFormat tests ---

    #[test]
    fn test_asset_format_from_extension() {
        assert_eq!(AssetFormat::from_extension("obj"), Some(AssetFormat::OBJ));
        assert_eq!(AssetFormat::from_extension("gltf"), Some(AssetFormat::GLTF));
        assert_eq!(AssetFormat::from_extension("fbx"), Some(AssetFormat::FBX));
        assert_eq!(AssetFormat::from_extension("blueprint"), Some(AssetFormat::BLUEPRINT));
        assert_eq!(AssetFormat::from_extension("vox"), Some(AssetFormat::VOXEL));
        assert_eq!(AssetFormat::from_extension("heightmap"), Some(AssetFormat::HEIGHTMAP));
        assert_eq!(AssetFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_asset_format_extension() {
        assert_eq!(AssetFormat::OBJ.extension(), "obj");
        assert_eq!(AssetFormat::GLTF.extension(), "gltf");
        assert_eq!(AssetFormat::BLUEPRINT.extension(), "blueprint");
    }

    #[test]
    fn test_asset_format_case_insensitive() {
        assert_eq!(AssetFormat::from_extension("OBJ"), Some(AssetFormat::OBJ));
        assert_eq!(AssetFormat::from_extension("BluePrint"), Some(AssetFormat::BLUEPRINT));
        assert_eq!(AssetFormat::from_extension("GLB"), Some(AssetFormat::GLTF));
    }

    // --- AssetManifest tests ---

    #[test]
    fn test_manifest_creation() {
        let manifest = AssetManifest::new("test_asset", AssetFormat::OBJ, 1);
        assert_eq!(manifest.name, "test_asset");
        assert_eq!(manifest.format, AssetFormat::OBJ);
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.entities_count, 0);
    }

    #[test]
    fn test_manifest_compatibility() {
        let a = AssetManifest::new("a", AssetFormat::BLUEPRINT, 1);
        let b = AssetManifest::new("b", AssetFormat::BLUEPRINT, 1);
        let c = AssetManifest::new("c", AssetFormat::BLUEPRINT, 2);
        assert!(a.is_compatible_with(&b));
        assert!(!a.is_compatible_with(&c));
    }

    #[test]
    fn test_manifest_dependencies() {
        let mut manifest = AssetManifest::new("test", AssetFormat::BLUEPRINT, 1);
        manifest.add_dependency("base_materials");
        manifest.add_dependency("base_materials");
        assert_eq!(manifest.dependencies.len(), 1);
        manifest.add_dependency("physics_lib");
        assert_eq!(manifest.dependencies.len(), 2);
    }

    // --- ObjImporter tests ---

    #[test]
    fn test_obj_import_simple_triangle() {
        let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"#;
        let importer = ObjImporter::new();
        let result = importer.import(obj_data.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
        assert!(entities[0].is_active());
    }

    #[test]
    fn test_obj_import_with_normals() {
        let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
f 1//1 2//1 3//1
"#;
        let importer = ObjImporter::new();
        let result = importer.import(obj_data.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn test_obj_import_with_scale() {
        let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
f 1 2 3
"#;
        let importer = ObjImporter::new().with_scale(2.0);
        let result = importer.import(obj_data.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn test_obj_import_empty_data() {
        let obj_data = "# just a comment\n";
        let importer = ObjImporter::new();
        let result = importer.import(obj_data.as_bytes(), 0);
        assert!(result.is_err());
    }

    // --- Blueprint import/export tests ---

    #[test]
    fn test_blueprint_import_single_entity() {
        let json = r#"{
            "name": "test_blueprint",
            "version": 1,
            "entities": [
                {
                    "position": [10.0, 0.0, 5.0],
                    "physics": {
                        "mass": 50.0,
                        "density": 7874.0,
                        "hardness": 4.0
                    },
                    "chemistry": {
                        "elements": [{"element": "Fe", "fraction": 1.0}],
                        "bond_types": ["Metallic"],
                        "ph": 7.0
                    },
                    "function_tags": ["structural", "load_bearing"],
                    "extensions": {}
                }
            ],
            "dependencies": [],
            "metadata": {}
        }"#;

        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].position.x, 10.0);
        assert_eq!(entities[0].physics.mass, 50.0);
        assert_eq!(entities[0].physics.density, 7874.0);

        let tags = entities[0].get_extension("function_tags");
        assert!(tags.is_some());
    }

    #[test]
    fn test_blueprint_import_with_rotation() {
        let json = r#"{
            "name": "rotated_entity",
            "version": 1,
            "entities": [
                {
                    "position": [0.0, 0.0, 0.0],
                    "rotation": [0.0, 0.707, 0.0, 0.707],
                    "function_tags": [],
                    "extensions": {}
                }
            ],
            "dependencies": [],
            "metadata": {}
        }"#;

        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
        assert!((entities[0].rotation.w - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_blueprint_import_multiple_entities() {
        let json = r#"{
            "name": "multi_entity",
            "version": 1,
            "entities": [
                {
                    "position": [0.0, 0.0, 0.0],
                    "function_tags": ["core"],
                    "extensions": {}
                },
                {
                    "position": [1.0, 0.0, 0.0],
                    "function_tags": ["armor"],
                    "extensions": {}
                },
                {
                    "position": [2.0, 0.0, 0.0],
                    "function_tags": ["weapon"],
                    "extensions": {}
                }
            ],
            "dependencies": [],
            "metadata": {}
        }"#;

        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 3);
    }

    #[test]
    fn test_blueprint_export_roundtrip() {
        let original_json = r#"{"name":"roundtrip_test","version":1,"entities":[{"id":null,"position":[1.0,2.0,3.0],"rotation":null,"velocity":null,"physics":null,"chemistry":null,"biology":null,"function_tags":["test_tag"],"parent_id":null,"extensions":{}}],"dependencies":[],"metadata":{}}"#;
        let importer = BlueprintImporter::new();
        let entities = importer.import(original_json.as_bytes(), 0).unwrap();

        let exporter = BlueprintExporter::new("roundtrip_test", 1);
        let exported = exporter.export(&entities).unwrap();

        let reimported = importer.import(&exported, 0).unwrap();
        assert_eq!(reimported.len(), 1);
        assert_eq!(reimported[0].position.x, 1.0);
        assert_eq!(reimported[0].position.y, 2.0);
        assert_eq!(reimported[0].position.z, 3.0);
    }

    #[test]
    fn test_blueprint_export_empty_entities() {
        let exporter = BlueprintExporter::new("empty", 1);
        let result = exporter.export(&[]);
        assert!(result.is_err());
    }

    // --- AssetCache tests ---

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = AssetCache::new(10);
        let entity = MetaEntity::iron(Vec3::ZERO, 0);

        cache.insert("test_key", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);

        let result = cache.get("test_key", AssetFormat::BLUEPRINT, 1);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = AssetCache::new(10);
        let result = cache.get("non_existent", AssetFormat::OBJ, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = AssetCache::new(3);
        let entity = MetaEntity::iron(Vec3::ZERO, 0);

        cache.insert("a", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);
        cache.insert("b", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);
        cache.insert("c", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);

        cache.get("a", AssetFormat::BLUEPRINT, 1);

        cache.insert("d", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);

        assert!(cache.get("b", AssetFormat::BLUEPRINT, 1).is_none());
        assert!(cache.get("a", AssetFormat::BLUEPRINT, 1).is_some());
        assert!(cache.get("c", AssetFormat::BLUEPRINT, 1).is_some());
        assert!(cache.get("d", AssetFormat::BLUEPRINT, 1).is_some());
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = AssetCache::new(10);
        let entity = MetaEntity::iron(Vec3::ZERO, 0);

        cache.insert("k1", AssetFormat::BLUEPRINT, 1, vec![entity.clone()]);
        cache.get("k1", AssetFormat::BLUEPRINT, 1);
        cache.get("missing", AssetFormat::BLUEPRINT, 1);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    // --- Version compatibility tests ---

    #[test]
    fn test_version_registry() {
        let mut pipeline = AssetPipeline::new(10);

        pipeline.register_version(AssetFormat::BLUEPRINT, 1);
        pipeline.register_version(AssetFormat::BLUEPRINT, 2);
        pipeline.register_version(AssetFormat::BLUEPRINT, 3);

        assert!(pipeline.is_version_supported(AssetFormat::BLUEPRINT, 1));
        assert!(pipeline.is_version_supported(AssetFormat::BLUEPRINT, 2));
        assert!(pipeline.is_version_supported(AssetFormat::BLUEPRINT, 3));
        assert!(!pipeline.is_version_supported(AssetFormat::BLUEPRINT, 4));
        assert_eq!(pipeline.latest_version(AssetFormat::BLUEPRINT), Some(3));
    }

    #[test]
    fn test_import_with_version_check() {
        let mut pipeline = AssetPipeline::new(10);
        pipeline.register_importer(Box::new(BlueprintImporter::new()));
        pipeline.register_version(AssetFormat::BLUEPRINT, 1);

        let json = r#"{"name":"test","version":1,"entities":[{"position":[0.0,0.0,0.0],"function_tags":[],"extensions":{}}],"dependencies":[],"metadata":{}}"#;

        let result =
            pipeline.import_with_version("test", AssetFormat::BLUEPRINT, 1, json.as_bytes(), 0);
        assert!(result.is_ok());

        let result =
            pipeline.import_with_version("test2", AssetFormat::BLUEPRINT, 99, json.as_bytes(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_import_export_roundtrip() {
        let mut pipeline = AssetPipeline::new(10);
        pipeline.register_importer(Box::new(BlueprintImporter::new()));
        pipeline.register_exporter(Box::new(BlueprintExporter::new("pipeline_test", 1)));
        pipeline.register_version(AssetFormat::BLUEPRINT, 1);

        let json = r#"{"name":"pipeline_test","version":1,"entities":[{"position":[1.0,2.0,3.0],"function_tags":["test"],"extensions":{}}],"dependencies":[],"metadata":{}}"#;

        let entities =
            pipeline.import("pipe_test", AssetFormat::BLUEPRINT, json.as_bytes(), 0).unwrap();
        assert_eq!(entities.len(), 1);

        let exported = pipeline.export(AssetFormat::BLUEPRINT, &entities).unwrap();
        assert!(!exported.is_empty());

        let reimported =
            pipeline.import("pipe_test2", AssetFormat::BLUEPRINT, &exported, 0).unwrap();
        assert_eq!(reimported.len(), 1);
    }

    #[test]
    fn test_unsupported_format_error() {
        let mut pipeline = AssetPipeline::new(10);
        let result = pipeline.import("test", AssetFormat::FBX, b"fake data", 0);
        assert!(result.is_err());
        match result {
            Err(ImportError::UnsupportedFormat(_)) => {},
            _ => panic!("expected UnsupportedFormat"),
        }
    }

    #[test]
    fn test_blueprint_import_invalid_json() {
        let importer = BlueprintImporter::new();
        let result = importer.import(b"not valid json", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_blueprint_import_empty_entities() {
        let json = r#"{"name":"empty","version":1,"entities":[],"dependencies":[],"metadata":{}}"#;
        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_blueprint_import_with_biology() {
        let json = r#"{
            "name": "bio_entity",
            "version": 1,
            "entities": [
                {
                    "position": [0.0, 0.0, 0.0],
                    "biology": {
                        "gene_tokens": [
                            {"name": "ACTN3", "expression_level": 1.0, "dominant": true}
                        ],
                        "cell_type": "EukaryoticAnimal",
                        "health": 100.0,
                        "max_health": 100.0
                    },
                    "function_tags": [],
                    "extensions": {}
                }
            ],
            "dependencies": [],
            "metadata": {}
        }"#;

        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].biology.health, 100.0);
        assert_eq!(entities[0].biology.cell_type, CellType::EukaryoticAnimal);
        assert_eq!(entities[0].biology.gene_tokens.len(), 1);
    }

    #[test]
    fn test_obj_import_multiple_faces() {
        let obj_data = r#"
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
v 1.0 1.0 0.0
f 1 2 3
f 2 4 3
"#;
        let importer = ObjImporter::new();
        let result = importer.import(obj_data.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = AssetCache::new(10);
        let entity = MetaEntity::iron(Vec3::ZERO, 0);

        cache.insert("k", AssetFormat::BLUEPRINT, 1, vec![entity]);
        assert!(cache.contains("k", AssetFormat::BLUEPRINT, 1));

        cache.clear();
        assert!(!cache.contains("k", AssetFormat::BLUEPRINT, 1));
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = AssetCache::new(10);
        let entity = MetaEntity::iron(Vec3::ZERO, 0);

        cache.insert("k", AssetFormat::BLUEPRINT, 1, vec![entity]);
        cache.remove("k", AssetFormat::BLUEPRINT, 1);
        assert!(!cache.contains("k", AssetFormat::BLUEPRINT, 1));
    }

    #[test]
    fn test_blueprint_import_with_parent_id() {
        let parent_id = Uuid::new_v4();
        let json = format!(
            r#"{{
            "name": "child_entity",
            "version": 1,
            "entities": [
                {{
                    "position": [0.0, 0.0, 0.0],
                    "parent_id": "{}",
                    "function_tags": [],
                    "extensions": {{}}
                }}
            ],
            "dependencies": [],
            "metadata": {{}}
        }}"#,
            parent_id
        );

        let importer = BlueprintImporter::new();
        let result = importer.import(json.as_bytes(), 0);
        assert!(result.is_ok());
        let entities = result.unwrap();
        assert_eq!(entities[0].parent_id, Some(parent_id));
    }
}
