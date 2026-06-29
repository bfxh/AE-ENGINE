"""
材质→物理属性映射器
输入: manifest.json (含 material_label)
输出: 更新 manifest.json (添加 material_properties)
"""

import json
import sys
from pathlib import Path

MATERIAL_DATABASE = {
    "metal_steel": {
        "density": 7800.0, "hardness": 8.0, "toughness": 5.0,
        "thermal_conductivity": 50.0, "electrical_conductivity": 1e7,
        "specific_heat": 500.0, "melting_point": 1800.0,
        "youngs_modulus": 200e9, "poisson_ratio": 0.3,
        "friction_coefficient": 0.6, "restitution": 0.3,
        "pbr_metallic": 0.95, "pbr_roughness": 0.3,
        "flammable": False, "corrosion_resistance": 0.3
    },
    "metal_aluminum": {
        "density": 2700.0, "hardness": 3.0, "toughness": 7.0,
        "thermal_conductivity": 205.0, "electrical_conductivity": 3.5e7,
        "specific_heat": 900.0, "melting_point": 660.0,
        "youngs_modulus": 70e9, "poisson_ratio": 0.33,
        "friction_coefficient": 0.4, "restitution": 0.2,
        "pbr_metallic": 0.95, "pbr_roughness": 0.2,
        "flammable": False, "corrosion_resistance": 0.7
    },
    "metal_copper": {
        "density": 8900.0, "hardness": 3.0, "toughness": 8.0,
        "thermal_conductivity": 385.0, "electrical_conductivity": 5.8e7,
        "specific_heat": 385.0, "melting_point": 1085.0,
        "youngs_modulus": 120e9, "poisson_ratio": 0.34,
        "friction_coefficient": 0.4, "restitution": 0.2,
        "pbr_metallic": 0.95, "pbr_roughness": 0.3,
        "flammable": False, "corrosion_resistance": 0.5
    },
    "metal_iron": {
        "density": 7874.0, "hardness": 4.0, "toughness": 4.0,
        "thermal_conductivity": 80.0, "electrical_conductivity": 1e7,
        "specific_heat": 450.0, "melting_point": 1538.0,
        "youngs_modulus": 200e9, "poisson_ratio": 0.29,
        "friction_coefficient": 0.5, "restitution": 0.3,
        "pbr_metallic": 0.9, "pbr_roughness": 0.5,
        "flammable": False, "corrosion_resistance": 0.1
    },
    "wood_oak": {
        "density": 700.0, "hardness": 3.0, "toughness": 8.0,
        "thermal_conductivity": 0.15, "electrical_conductivity": 0.0,
        "specific_heat": 2400.0, "melting_point": 0.0,
        "youngs_modulus": 12e9, "poisson_ratio": 0.35,
        "friction_coefficient": 0.4, "restitution": 0.1,
        "pbr_metallic": 0.0, "pbr_roughness": 0.7,
        "flammable": True, "corrosion_resistance": 0.2,
        "ignition_temp": 250.0, "burn_rate": 0.5
    },
    "wood_pine": {
        "density": 500.0, "hardness": 2.0, "toughness": 6.0,
        "thermal_conductivity": 0.12, "electrical_conductivity": 0.0,
        "specific_heat": 2300.0, "melting_point": 0.0,
        "youngs_modulus": 9e9, "poisson_ratio": 0.35,
        "friction_coefficient": 0.4, "restitution": 0.1,
        "pbr_metallic": 0.0, "pbr_roughness": 0.8,
        "flammable": True, "corrosion_resistance": 0.15,
        "ignition_temp": 220.0, "burn_rate": 0.7
    },
    "plastic": {
        "density": 1200.0, "hardness": 2.0, "toughness": 5.0,
        "thermal_conductivity": 0.2, "electrical_conductivity": 0.0,
        "specific_heat": 1500.0, "melting_point": 160.0,
        "youngs_modulus": 3e9, "poisson_ratio": 0.4,
        "friction_coefficient": 0.3, "restitution": 0.2,
        "pbr_metallic": 0.0, "pbr_roughness": 0.4,
        "flammable": True, "corrosion_resistance": 0.8,
        "ignition_temp": 350.0, "burn_rate": 0.3
    },
    "rubber": {
        "density": 1100.0, "hardness": 1.0, "toughness": 10.0,
        "thermal_conductivity": 0.16, "electrical_conductivity": 0.0,
        "specific_heat": 2000.0, "melting_point": 180.0,
        "youngs_modulus": 0.05e9, "poisson_ratio": 0.49,
        "friction_coefficient": 0.8, "restitution": 0.8,
        "pbr_metallic": 0.0, "pbr_roughness": 0.9,
        "flammable": True, "corrosion_resistance": 0.6,
        "ignition_temp": 300.0, "burn_rate": 0.2
    },
    "glass": {
        "density": 2500.0, "hardness": 6.0, "toughness": 0.5,
        "thermal_conductivity": 1.0, "electrical_conductivity": 0.0,
        "specific_heat": 840.0, "melting_point": 1500.0,
        "youngs_modulus": 70e9, "poisson_ratio": 0.22,
        "friction_coefficient": 0.3, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.1,
        "flammable": False, "corrosion_resistance": 0.9,
        "transparency": 0.9, "refraction_index": 1.52
    },
    "flesh": {
        "density": 1050.0, "hardness": 0.5, "toughness": 3.0,
        "thermal_conductivity": 0.5, "electrical_conductivity": 0.5,
        "specific_heat": 3500.0, "melting_point": 0.0,
        "youngs_modulus": 0.02e9, "poisson_ratio": 0.45,
        "friction_coefficient": 0.5, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.6,
        "flammable": False, "corrosion_resistance": 0.0,
        "is_organic": True
    },
    "concrete": {
        "density": 2400.0, "hardness": 7.0, "toughness": 1.0,
        "thermal_conductivity": 1.4, "electrical_conductivity": 0.0,
        "specific_heat": 880.0, "melting_point": 0.0,
        "youngs_modulus": 30e9, "poisson_ratio": 0.2,
        "friction_coefficient": 0.6, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.9,
        "flammable": False, "corrosion_resistance": 0.5
    },
    "stone_granite": {
        "density": 2700.0, "hardness": 7.0, "toughness": 2.0,
        "thermal_conductivity": 2.8, "electrical_conductivity": 0.0,
        "specific_heat": 790.0, "melting_point": 0.0,
        "youngs_modulus": 60e9, "poisson_ratio": 0.25,
        "friction_coefficient": 0.5, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.8,
        "flammable": False, "corrosion_resistance": 0.7
    },
    "soil": {
        "density": 1500.0, "hardness": 1.0, "toughness": 0.5,
        "thermal_conductivity": 0.8, "electrical_conductivity": 0.01,
        "specific_heat": 1800.0, "melting_point": 0.0,
        "youngs_modulus": 0.05e9, "poisson_ratio": 0.4,
        "friction_coefficient": 0.7, "restitution": 0.0,
        "pbr_metallic": 0.0, "pbr_roughness": 1.0,
        "flammable": False, "corrosion_resistance": 0.0
    },
    "water": {
        "density": 1000.0, "hardness": 0.0, "toughness": 0.0,
        "thermal_conductivity": 0.6, "electrical_conductivity": 0.5,
        "specific_heat": 4186.0, "melting_point": 0.0,
        "youngs_modulus": 0.0, "poisson_ratio": 0.5,
        "friction_coefficient": 0.0, "restitution": 0.0,
        "pbr_metallic": 0.0, "pbr_roughness": 0.0,
        "flammable": False, "corrosion_resistance": 0.0,
        "is_fluid": True, "viscosity": 0.001, "boiling_point": 100.0
    },
    "vegetation_leaf": {
        "density": 600.0, "hardness": 0.5, "toughness": 2.0,
        "thermal_conductivity": 0.25, "electrical_conductivity": 0.0,
        "specific_heat": 3000.0, "melting_point": 0.0,
        "youngs_modulus": 0.01e9, "poisson_ratio": 0.4,
        "friction_coefficient": 0.3, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.6,
        "flammable": True, "corrosion_resistance": 0.0,
        "ignition_temp": 200.0, "burn_rate": 0.8, "is_organic": True
    },
    "vegetation_bark": {
        "density": 800.0, "hardness": 2.5, "toughness": 5.0,
        "thermal_conductivity": 0.15, "electrical_conductivity": 0.0,
        "specific_heat": 2400.0, "melting_point": 0.0,
        "youngs_modulus": 8e9, "poisson_ratio": 0.35,
        "friction_coefficient": 0.5, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.9,
        "flammable": True, "corrosion_resistance": 0.1,
        "ignition_temp": 280.0, "burn_rate": 0.4, "is_organic": True
    },
    "fabric_cotton": {
        "density": 300.0, "hardness": 0.1, "toughness": 3.0,
        "thermal_conductivity": 0.04, "electrical_conductivity": 0.0,
        "specific_heat": 1300.0, "melting_point": 0.0,
        "youngs_modulus": 0.001e9, "poisson_ratio": 0.4,
        "friction_coefficient": 0.3, "restitution": 0.0,
        "pbr_metallic": 0.0, "pbr_roughness": 0.8,
        "flammable": True, "corrosion_resistance": 0.0,
        "ignition_temp": 210.0, "burn_rate": 0.6, "is_cloth": True
    },
    "leather": {
        "density": 900.0, "hardness": 1.0, "toughness": 6.0,
        "thermal_conductivity": 0.15, "electrical_conductivity": 0.0,
        "specific_heat": 1500.0, "melting_point": 0.0,
        "youngs_modulus": 0.1e9, "poisson_ratio": 0.4,
        "friction_coefficient": 0.4, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.7,
        "flammable": True, "corrosion_resistance": 0.1,
        "ignition_temp": 300.0, "burn_rate": 0.3
    },
    "sand": {
        "density": 1600.0, "hardness": 0.5, "toughness": 0.1,
        "thermal_conductivity": 0.3, "electrical_conductivity": 0.0,
        "specific_heat": 830.0, "melting_point": 0.0,
        "youngs_modulus": 0.0, "poisson_ratio": 0.35,
        "friction_coefficient": 0.5, "restitution": 0.0,
        "pbr_metallic": 0.0, "pbr_roughness": 1.0,
        "flammable": False, "corrosion_resistance": 0.0,
        "is_granular": True
    },
    "ceramic": {
        "density": 2500.0, "hardness": 8.0, "toughness": 1.0,
        "thermal_conductivity": 1.5, "electrical_conductivity": 0.0,
        "specific_heat": 900.0, "melting_point": 2000.0,
        "youngs_modulus": 300e9, "poisson_ratio": 0.2,
        "friction_coefficient": 0.3, "restitution": 0.05,
        "pbr_metallic": 0.0, "pbr_roughness": 0.3,
        "flammable": False, "corrosion_resistance": 0.9
    },
}


def map_materials(manifest_path: str) -> dict:
    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    for part in manifest.get("parts", []):
        label = part.get("material_label", "metal_steel")
        props = MATERIAL_DATABASE.get(label, MATERIAL_DATABASE["metal_steel"])
        part["material_properties"] = props

    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)

    print(f"[OK] Material properties mapped for {len(manifest.get('parts', []))} parts")
    return manifest


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python material_mapper.py <manifest.json>")
        sys.exit(1)
    map_materials(sys.argv[1])
