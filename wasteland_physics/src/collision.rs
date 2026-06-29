use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::fixed_point::{FixedPoint, FixedVec3};
use crate::material::MaterialProperties;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollisionId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionEvent {
    pub id: CollisionId,
    pub entity_a: Uuid,
    pub entity_b: Uuid,
    pub point: FixedVec3,
    pub normal: FixedVec3,
    pub impulse: FixedPoint,
    pub relative_velocity: FixedVec3,
    pub material_a: MaterialProperties,
    pub material_b: MaterialProperties,
    pub timestamp: f64,
    pub collision_type: CollisionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollisionType {
    Impact,
    Sliding,
    Rolling,
    Penetration,
    Explosion,
    Fragmentation,
    Grazing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionDamage {
    pub entity: Uuid,
    pub point: FixedVec3,
    pub damage: FixedPoint,
    pub damage_type: DamageType,
    pub radius: FixedPoint,
    pub force: FixedVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Kinetic,
    Thermal,
    Chemical,
    Radiation,
    Electrical,
    Sonic,
    Explosive,
    Corrosive,
    Piercing,
    Blunt,
}

impl CollisionEvent {
    pub fn calculate_damage(&self) -> Vec<CollisionDamage> {
        let mut damages = Vec::new();
        let kinetic_energy =
            FixedPoint::from_f32(0.5) * self.impulse * self.relative_velocity.length();

        let damage_a = self.material_a.damage_at_point(self.impulse, self.relative_velocity);
        let damage_b = self.material_b.damage_at_point(self.impulse, -self.relative_velocity);

        let threshold = FixedPoint::from_f32(0.01);
        if damage_a > threshold {
            damages.push(CollisionDamage {
                entity: self.entity_a,
                point: self.point,
                damage: damage_a * FixedPoint::from_f32(100.0),
                damage_type: self.damage_type_from_velocity(),
                radius: (kinetic_energy * FixedPoint::from_f32(0.001))
                    .min(FixedPoint::from_f32(5.0)),
                force: self.normal * self.impulse,
            });
        }
        if damage_b > threshold {
            damages.push(CollisionDamage {
                entity: self.entity_b,
                point: self.point,
                damage: damage_b * FixedPoint::from_f32(100.0),
                damage_type: self.damage_type_from_velocity(),
                radius: (kinetic_energy * FixedPoint::from_f32(0.001))
                    .min(FixedPoint::from_f32(5.0)),
                force: -self.normal * self.impulse,
            });
        }
        damages
    }

    fn damage_type_from_velocity(&self) -> DamageType {
        let speed = self.relative_velocity.length().to_f32();
        match speed {
            s if s > 100.0 => DamageType::Explosive,
            s if s > 50.0 => DamageType::Piercing,
            s if s > 20.0 => DamageType::Kinetic,
            _ => DamageType::Blunt,
        }
    }

    pub fn is_significant(&self) -> bool {
        self.impulse.to_f32() > 10.0 || self.relative_velocity.length().to_f32() > 5.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionFilter {
    pub layer_a: u32,
    pub mask_a: u32,
    pub layer_b: u32,
    pub mask_b: u32,
}

impl CollisionFilter {
    pub fn should_collide(&self) -> bool {
        (self.layer_a & self.mask_b) != 0 && (self.layer_b & self.mask_a) != 0
    }

    pub fn player_projectile() -> Self {
        Self { layer_a: 0x0001, mask_a: 0xFFFF, layer_b: 0x0002, mask_b: 0xFFFF }
    }

    pub fn environment_all() -> Self {
        Self { layer_a: 0x0004, mask_a: 0xFFFF, layer_b: 0xFFFF, mask_b: 0xFFFF }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CollisionShape {
    Box { half_extents: FixedVec3 },
    Sphere { radius: FixedPoint },
    Capsule { radius: FixedPoint, half_height: FixedPoint },
    Cylinder { radius: FixedPoint, half_height: FixedPoint },
    ConvexHull { vertices: usize },
    TriangleMesh { triangles: usize },
    HeightField { rows: usize, cols: usize },
    VoxelGrid { resolution: [u32; 3], voxel_size: FixedPoint },
}

impl CollisionShape {
    pub fn volume(&self) -> FixedPoint {
        let pi = FixedPoint::from_f32(std::f32::consts::PI);
        match *self {
            Self::Box { half_extents } => {
                FixedPoint::from_f32(8.0) * half_extents.x * half_extents.y * half_extents.z
            },
            Self::Sphere { radius } => {
                FixedPoint::from_f32(4.0) / FixedPoint::from_f32(3.0)
                    * pi
                    * radius
                    * radius
                    * radius
            },
            Self::Capsule { radius, half_height } => {
                let sphere_vol = FixedPoint::from_f32(4.0) / FixedPoint::from_f32(3.0)
                    * pi
                    * radius
                    * radius
                    * radius;
                let cylinder_vol = pi * radius * radius * FixedPoint::from_f32(2.0) * half_height;
                sphere_vol + cylinder_vol
            },
            Self::Cylinder { radius, half_height } => {
                pi * radius * radius * FixedPoint::from_f32(2.0) * half_height
            },
            Self::ConvexHull { .. } | Self::TriangleMesh { .. } => FixedPoint::ZERO,
            Self::HeightField { .. } => FixedPoint::ZERO,
            Self::VoxelGrid { resolution, voxel_size } => {
                FixedPoint::from_i32(resolution[0] as i32)
                    * FixedPoint::from_i32(resolution[1] as i32)
                    * FixedPoint::from_i32(resolution[2] as i32)
                    * voxel_size
                    * voxel_size
                    * voxel_size
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_filter_should_collide() {
        let f =
            CollisionFilter { layer_a: 0x0001, mask_a: 0xFFFF, layer_b: 0x0002, mask_b: 0xFFFF };
        assert!(f.should_collide());
    }

    #[test]
    fn test_collision_filter_no_collision() {
        let f =
            CollisionFilter { layer_a: 0x0001, mask_a: 0x0000, layer_b: 0x0002, mask_b: 0xFFFF };
        assert!(!f.should_collide());
    }

    #[test]
    fn test_collision_filter_player_projectile() {
        let f = CollisionFilter::player_projectile();
        assert!(f.should_collide());
    }

    #[test]
    fn test_collision_filter_environment() {
        let f = CollisionFilter::environment_all();
        assert!(f.should_collide());
    }

    #[test]
    fn test_sphere_volume() {
        let shape = CollisionShape::Sphere { radius: FixedPoint::from_f32(1.0) };
        let vol = shape.volume();
        let expected = std::f32::consts::PI * 4.0 / 3.0;
        assert!((vol.to_f32() - expected).abs() < 0.1);
    }

    #[test]
    fn test_box_volume() {
        let shape = CollisionShape::Box { half_extents: FixedVec3::from_f32(1.0, 2.0, 3.0) };
        let vol = shape.volume();
        assert!((vol.to_f32() - 48.0).abs() < 0.1);
    }

    #[test]
    fn test_cylinder_volume() {
        let shape = CollisionShape::Cylinder {
            radius: FixedPoint::from_f32(1.0),
            half_height: FixedPoint::from_f32(2.0),
        };
        let vol = shape.volume();
        let expected = std::f32::consts::PI * 1.0 * 1.0 * 4.0;
        assert!((vol.to_f32() - expected).abs() < 0.1);
    }

    #[test]
    fn test_capsule_volume() {
        let shape = CollisionShape::Capsule {
            radius: FixedPoint::from_f32(1.0),
            half_height: FixedPoint::from_f32(2.0),
        };
        let vol = shape.volume();
        let expected = std::f32::consts::PI * 4.0 / 3.0 + std::f32::consts::PI * 4.0;
        assert!((vol.to_f32() - expected).abs() < 0.2);
    }

    #[test]
    fn test_voxel_grid_volume() {
        let shape = CollisionShape::VoxelGrid {
            resolution: [2, 3, 4],
            voxel_size: FixedPoint::from_f32(0.5),
        };
        let vol = shape.volume();
        let expected = 2.0 * 3.0 * 4.0 * 0.5 * 0.5 * 0.5;
        assert!((vol.to_f32() - expected).abs() < 0.01);
    }

    #[test]
    fn test_convex_hull_volume_zero() {
        let shape = CollisionShape::ConvexHull { vertices: 10 };
        assert_eq!(shape.volume().to_f32(), 0.0);
    }

    #[test]
    fn test_damage_type_from_velocity() {
        let material = MaterialProperties::default();
        let event = CollisionEvent {
            id: CollisionId(Uuid::new_v4()),
            entity_a: Uuid::new_v4(),
            entity_b: Uuid::new_v4(),
            point: FixedVec3::ZERO,
            normal: FixedVec3::from_f32(0.0, 1.0, 0.0),
            impulse: FixedPoint::from_f32(100.0),
            relative_velocity: FixedVec3::from_f32(150.0, 0.0, 0.0),
            material_a: material,
            material_b: material,
            timestamp: 0.0,
            collision_type: CollisionType::Impact,
        };
        let damages = event.calculate_damage();
        assert!(!damages.is_empty());
        assert_eq!(damages[0].damage_type, DamageType::Explosive);
    }

    #[test]
    fn test_damage_type_kinetic() {
        let material = MaterialProperties::default();
        let event = CollisionEvent {
            id: CollisionId(Uuid::new_v4()),
            entity_a: Uuid::new_v4(),
            entity_b: Uuid::new_v4(),
            point: FixedVec3::ZERO,
            normal: FixedVec3::from_f32(0.0, 1.0, 0.0),
            impulse: FixedPoint::from_f32(10.0),
            relative_velocity: FixedVec3::from_f32(30.0, 0.0, 0.0),
            material_a: material,
            material_b: material,
            timestamp: 0.0,
            collision_type: CollisionType::Impact,
        };
        let damages = event.calculate_damage();
        assert!(!damages.is_empty());
        assert_eq!(damages[0].damage_type, DamageType::Kinetic);
    }

    #[test]
    fn test_is_significant() {
        let material = MaterialProperties::default();
        let big_event = CollisionEvent {
            id: CollisionId(Uuid::new_v4()),
            entity_a: Uuid::new_v4(),
            entity_b: Uuid::new_v4(),
            point: FixedVec3::ZERO,
            normal: FixedVec3::from_f32(0.0, 1.0, 0.0),
            impulse: FixedPoint::from_f32(100.0),
            relative_velocity: FixedVec3::from_f32(50.0, 0.0, 0.0),
            material_a: material,
            material_b: material,
            timestamp: 0.0,
            collision_type: CollisionType::Impact,
        };
        assert!(big_event.is_significant());

        let small_event = CollisionEvent {
            id: CollisionId(Uuid::new_v4()),
            entity_a: Uuid::new_v4(),
            entity_b: Uuid::new_v4(),
            point: FixedVec3::ZERO,
            normal: FixedVec3::from_f32(0.0, 1.0, 0.0),
            impulse: FixedPoint::from_f32(1.0),
            relative_velocity: FixedVec3::from_f32(1.0, 0.0, 0.0),
            material_a: material,
            material_b: material,
            timestamp: 0.0,
            collision_type: CollisionType::Grazing,
        };
        assert!(!small_event.is_significant());
    }
}
