"""
网格质量检查器 - 验证GLB文件是否符合管线标准
"""

import json
import sys
from pathlib import Path

MAX_FACE_COUNT = 50000
MAX_PART_FACE_COUNT = 5000
TARGET_TEXTURE_RES = 2048
MAX_UV_OVERLAP_RATIO = 0.05


def check_glb(path: str) -> dict:
    results = {"file": path, "checks": [], "pass": True}
    errors = []

    try:
        import trimesh
        scene = trimesh.load(path, force="scene")
        total_faces = 0
        for name, geom in scene.geometry.items():
            if hasattr(geom, 'faces'):
                fc = len(geom.faces)
                total_faces += fc
                if fc > MAX_PART_FACE_COUNT:
                    errors.append(f"Part '{name}': {fc} faces > {MAX_PART_FACE_COUNT}")
        results["total_faces"] = total_faces
        if total_faces > MAX_FACE_COUNT:
            errors.append(f"Total {total_faces} faces > {MAX_FACE_COUNT}")
    except Exception as e:
        errors.append(f"trimesh load failed: {e}")

    try:
        from pygltflib import GLTF2
        gltf = GLTF2().load(path)
        has_materials = len(gltf.materials) > 0
        has_textures = any(m.pbrMetallicRoughness and
                          (m.pbrMetallicRoughness.baseColorTexture is not None or
                           m.pbrMetallicRoughness.metallicRoughnessTexture is not None)
                          for m in gltf.materials) if has_materials else False

        results["mesh_count"] = len(gltf.meshes)
        results["material_count"] = len(gltf.materials)
        results["texture_count"] = len(gltf.textures)
        results["has_pbr"] = has_textures

        if not has_materials:
            errors.append("No materials found")
    except Exception as e:
        errors.append(f"pygltflib load failed: {e}")

    file_size_mb = Path(path).stat().st_size / (1024 * 1024)
    results["file_size_mb"] = round(file_size_mb, 2)
    if file_size_mb > 50:
        errors.append(f"File too large: {file_size_mb:.1f}MB > 50MB")

    if errors:
        results["pass"] = False
        results["errors"] = errors

    return results


def check_manifest(manifest_path: str) -> dict:
    results = {"file": manifest_path, "checks": [], "pass": True}
    errors = []

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    parts = manifest.get("parts", [])
    if not parts:
        errors.append("No parts defined in manifest")
    else:
        for part in parts:
            name = part.get("name", "")
            if not part.get("semantic_type"):
                errors.append(f"Part '{name}': missing semantic_type")
            if not part.get("material_label"):
                errors.append(f"Part '{name}': missing material_label")
            if part.get("joint_type") not in ["fixed", "revolute", "prismatic", "ball", "hinge"]:
                errors.append(f"Part '{name}': invalid joint_type '{part.get('joint_type')}'")
            if not part.get("physics_role"):
                errors.append(f"Part '{name}': missing physics_role")

    constraints = manifest.get("constraints", {})
    if constraints.get("total_face_count", 0) > MAX_FACE_COUNT:
        errors.append(f"Constraint face_count exceeds limit")

    if errors:
        results["pass"] = False
        results["errors"] = errors

    results["part_count"] = len(parts)
    return results


def check_directory(asset_dir: str) -> list:
    results = []
    asset_path = Path(asset_dir)

    manifest_path = asset_path / "manifest.json"
    if manifest_path.exists():
        results.append(check_manifest(str(manifest_path)))
    else:
        results.append({"file": str(manifest_path), "pass": False, "errors": ["manifest.json not found"]})

    for glb_file in asset_path.glob("**/*.glb"):
        if glb_file.name != "collision.glb":
            results.append(check_glb(str(glb_file)))

    return results


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python quality_checker.py <asset_dir>")
        sys.exit(1)
    results = check_directory(sys.argv[1])
    for r in results:
        status = "PASS" if r.get("pass") else "FAIL"
        print(f"[{status}] {r.get('file', '?')}")
        for err in r.get("errors", []):
            print(f"  - {err}")
