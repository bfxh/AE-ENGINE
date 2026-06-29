"""
语义标注器 - LLM辅助3D模型部件语义标注
输入: 模型路径 + 多视角截图
输出: 语义标注JSON (Schema)
"""

import json
import os
import subprocess
import sys
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Optional

CONFIG_PATH = Path(__file__).parent / "config.json"


def load_config():
    with open(CONFIG_PATH, "r", encoding="utf-8") as f:
        return json.load(f)


@dataclass
class PartAnnotation:
    name: str
    semantic_type: str = "body"
    joint_type: str = "fixed"
    joint_axis: list = field(default_factory=lambda: [0, 1, 0])
    joint_limits: dict = field(default_factory=dict)
    physics_role: str = "static_collider"
    mass: float = 1.0
    material_label: str = "metal_steel"
    interaction_tags: list = field(default_factory=list)
    mesh_path: str = ""


@dataclass
class SchemaManifest:
    schema_version: str = "1.0"
    global_prompt: str = ""
    parts: list = field(default_factory=list)
    constraints: dict = field(default_factory=lambda: {
        "total_face_count": 50000,
        "texture_resolution": 2048,
        "lod_levels": 3,
        "format": "glb",
        "pivot_convention": "center_of_mass"
    })


SEMANTIC_TYPES = [
    "body", "wheel", "door", "window", "engine", "weapon",
    "container", "wing", "leg", "arm", "head", "torso",
    "power_source", "sensor", "antenna", "turret", "barrel",
    "seat", "handle", "lever", "pedal", "exhaust",
    "headlight", "taillight", "bumper", "roof", "trunk",
    "branch", "trunk_tree", "leaf", "root", "rock",
    "water_surface", "terrain"
]

JOINT_TYPES = ["fixed", "revolute", "prismatic", "ball", "hinge"]

MATERIAL_LABELS = [
    "metal_steel", "metal_aluminum", "metal_copper", "metal_iron",
    "wood_oak", "wood_pine", "wood_bamboo",
    "plastic", "rubber", "glass", "ceramic",
    "flesh", "bone", "leather", "fabric_cotton", "fabric_denim",
    "concrete", "brick", "stone_granite", "stone_sandstone",
    "soil", "sand", "clay", "asphalt",
    "water", "ice", "snow",
    "vegetation_leaf", "vegetation_bark", "vegetation_grass"
]

PHYSICS_ROLES = ["static_collider", "rigid_body", "kinematic_body"]

INTERACTION_TAGS = [
    "interactable", "openable", "breakable", "powered",
    "heat_source", "heat_sink", "conductor", "insulator",
    "battery", "generator", "crafting_component", "damageable",
    "container", "wearable", "mountable", "drivable",
    "flammable", "explosive", "toxic", "radioactive"
]


def query_llm(prompt: str, config: dict = None) -> str:
    if config is None:
        config = load_config()
    endpoint = config.get("llm_endpoint", "http://localhost:11434/api/generate")
    model = config.get("llm_model", "qwen3.5-4b")
    try:
        import requests
        resp = requests.post(endpoint, json={
            "model": model,
            "prompt": prompt,
            "stream": False,
            "options": {"temperature": 0.3, "num_predict": 2048}
        }, timeout=120)
        if resp.status_code == 200:
            return resp.json().get("response", "")
    except Exception as e:
        print(f"[WARN] LLM unavailable: {e}, using heuristic fallback")
    return ""


def annotate_parts_heuristic(model_path: str) -> SchemaManifest:
    """启发式语义标注 - 无LLM时的降级方案"""
    manifest = SchemaManifest()
    manifest.global_prompt = Path(model_path).stem.replace("_", " ")

    try:
        import trimesh
        scene = trimesh.load(model_path, force="scene")
        geometry_names = list(scene.geometry.keys())
    except Exception:
        try:
            from pygltflib import GLTF2
            gltf = GLTF2().load(model_path)
            geometry_names = [f"mesh_{i}" for i in range(len(gltf.meshes))]
        except Exception:
            geometry_names = ["main_body"]

    for name in geometry_names:
        name_lower = name.lower()
        semantic_type = "body"
        joint_type = "fixed"
        material_label = "metal_steel"
        physics_role = "static_collider"
        interaction_tags = []
        mass = 1.0

        if any(kw in name_lower for kw in ["wheel", "tire", "rim"]):
            semantic_type = "wheel"
            joint_type = "revolute"
            joint_axis = [1, 0, 0]
            material_label = "rubber"
            physics_role = "rigid_body"
            mass = 15.0
        elif any(kw in name_lower for kw in ["door"]):
            semantic_type = "door"
            joint_type = "hinge"
            joint_axis = [0, 1, 0]
            interaction_tags = ["openable", "interactable"]
        elif any(kw in name_lower for kw in ["engine", "motor"]):
            semantic_type = "power_source"
            interaction_tags = ["heat_source", "powered", "damageable"]
            mass = 80.0
        elif any(kw in name_lower for kw in ["headlight", "light", "lamp"]):
            semantic_type = "headlight"
            interaction_tags = ["powered"]
        elif any(kw in name_lower for kw in ["window", "glass"]):
            semantic_type = "window"
            material_label = "glass"
            interaction_tags = ["breakable"]
        elif any(kw in name_lower for kw in ["weapon", "gun", "cannon", "barrel"]):
            semantic_type = "weapon"
            interaction_tags = ["damageable"]
        elif any(kw in name_lower for kw in ["seat", "chair"]):
            semantic_type = "seat"
            material_label = "fabric_cotton"
            interaction_tags = ["mountable"]
        elif any(kw in name_lower for kw in ["trunk_tree", "trunk", "bark"]):
            semantic_type = "trunk_tree"
            material_label = "vegetation_bark"
        elif any(kw in name_lower for kw in ["branch", "leaf"]):
            semantic_type = "branch"
            material_label = "vegetation_leaf"
        elif any(kw in name_lower for kw in ["rock", "stone"]):
            semantic_type = "rock"
            material_label = "stone_granite"
            mass = 50.0
        elif any(kw in name_lower for kw in ["leg", "foot"]):
            semantic_type = "leg"
            joint_type = "revolute"
            physics_role = "rigid_body"
        elif any(kw in name_lower for kw in ["arm", "hand"]):
            semantic_type = "arm"
            joint_type = "ball"
            physics_role = "rigid_body"

        part = PartAnnotation(
            name=name,
            semantic_type=semantic_type,
            joint_type=joint_type,
            joint_axis=[1, 0, 0] if joint_type == "revolute" else [0, 1, 0],
            physics_role=physics_role,
            mass=mass,
            material_label=material_label,
            interaction_tags=interaction_tags,
            mesh_path=f"{name}.glb"
        )
        manifest.parts.append(asdict(part))

    return manifest


def annotate_parts_llm(model_path: str, screenshots: list = None) -> SchemaManifest:
    """LLM辅助语义标注"""
    config = load_config()
    manifest = annotate_parts_heuristic(model_path)

    part_list = [p["name"] for p in manifest.parts]
    prompt = f"""你是3D建模专家。为以下模型部件分配语义标注。
部件列表: {json.dumps(part_list, ensure_ascii=False)}
模型名: {Path(model_path).stem}

对每个部件返回JSON数组，每项包含:
- name: 部件名
- semantic_type: {SEMANTIC_TYPES}
- joint_type: {JOINT_TYPES}
- material_label: {MATERIAL_LABELS}
- interaction_tags: 从 {INTERACTION_TAGS} 中选择
- physics_role: {PHYSICS_ROLES}
- mass: 估计质量(kg)

只返回JSON数组，不要其他文字。"""

    response = query_llm(prompt, config)
    if response:
        try:
            start = response.index("[")
            end = response.rindex("]") + 1
            llm_parts = json.loads(response[start:end])
            llm_map = {p["name"]: p for p in llm_parts if "name" in p}
            for i, part in enumerate(manifest.parts):
                if part["name"] in llm_map:
                    llm_p = llm_map[part["name"]]
                    for key in ["semantic_type", "joint_type", "material_label",
                                "interaction_tags", "physics_role", "mass"]:
                        if key in llm_p:
                            manifest.parts[i][key] = llm_p[key]
        except (ValueError, json.JSONDecodeError):
            print("[WARN] LLM response parse failed, using heuristic results")

    return manifest


def annotate(model_path: str, output_dir: str = None, use_llm: bool = True) -> str:
    if output_dir is None:
        output_dir = str(Path(model_path).parent)
    os.makedirs(output_dir, exist_ok=True)

    if use_llm:
        manifest = annotate_parts_llm(model_path)
    else:
        manifest = annotate_parts_heuristic(model_path)

    out_path = os.path.join(output_dir, "manifest.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(asdict(manifest), f, indent=2, ensure_ascii=False)

    print(f"[OK] Schema written to {out_path} ({len(manifest.parts)} parts)")
    return out_path


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python semantic_annotator.py <model_path> [--no-llm]")
        sys.exit(1)
    model = sys.argv[1]
    use_llm = "--no-llm" not in sys.argv
    annotate(model, use_llm=use_llm)
