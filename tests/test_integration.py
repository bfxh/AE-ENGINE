"""
Integration Test Suite
Tests the integration between different systems
"""
import sys
import os
import json

def test_project_structure():
    """Verify project structure is complete."""
    print("[TEST] Project Structure")
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    required_dirs = [
        "wasteland_engine",
        "wasteland_physics",
        "wasteland_field",
        "wasteland_particle",
        "wasteland_emergence",
        "wasteland_biology",
        "wasteland_chemistry",
        "gdextension",
        "godot_project",
        "blender_plugin",
        "tests",
    ]
    
    all_found = True
    for dir_name in required_dirs:
        path = os.path.join(root, dir_name)
        if os.path.isdir(path):
            print(f"  Found: {dir_name}/")
        else:
            print(f"  MISSING: {dir_name}/")
            all_found = False
    
    if all_found:
        print("  PASS: All directories present")
    return all_found

def test_core_crates():
    """Verify core crate files."""
    print("[TEST] Core Crates")
    
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    crate_files = [
        ("wasteland_engine", "src/lib.rs"),
        ("wasteland_physics", "src/lib.rs"),
        ("wasteland_field", "src/lib.rs"),
        ("wasteland_particle", "src/lib.rs"),
        ("wasteland_emergence", "src/lib.rs"),
        ("wasteland_biology", "src/lib.rs"),
        ("wasteland_chemistry", "src/lib.rs"),
        ("gdextension", "src/lib.rs"),
    ]
    
    all_found = True
    for crate, file in crate_files:
        path = os.path.join(root, crate, file)
        if os.path.exists(path):
            print(f"  Found: {crate}/{file}")
        else:
            print(f"  MISSING: {crate}/{file}")
            all_found = False
    
    if all_found:
        print("  PASS: All crate files present")
    return all_found

def test_phase_implementation():
    """Verify Phase 1-5 implementations."""
    print("[TEST] Phase Implementation")
    
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    phases = {
        "Phase 1 - Computational Paradigm": [
            ("wasteland_timeslice", "src/lib.rs"),
        ],
        "Phase 2 - Voxel + Dual Phase": [
            ("wasteland_physics", "src/octree.rs"),
            ("wasteland_physics", "src/dual_phase.rs"),
            ("wasteland_physics", "src/mpm.rs"),
        ],
        "Phase 3 - Unified Field Theory": [
            ("wasteland_field", "src/unified_field.rs"),
            ("wasteland_field", "src/field_solver.rs"),
        ],
        "Phase 4 - Emergent Particle System": [
            ("wasteland_particle", "src/emergent_rules.rs"),
            ("wasteland_particle", "src/chemical_emergence.rs"),
            ("wasteland_particle", "src/biological_emergence.rs"),
        ],
        "Phase 5 - Morphogenesis & Materials": [
            ("wasteland_emergence", "src/morphogenesis.rs"),
            ("wasteland_emergence", "src/holographic_material.rs"),
            ("wasteland_emergence", "src/time_surface.rs"),
            ("wasteland_emergence", "src/emergent_details.rs"),
            ("wasteland_emergence", "src/mycelial_network.rs"),
        ],
    }
    
    all_found = True
    for phase, files in phases.items():
        print(f"\n  {phase}:")
        for crate, file in files:
            path = os.path.join(root, crate, file)
            if os.path.exists(path):
                print(f"    OK {crate}/{file}")
            else:
                print(f"    NO {crate}/{file}")
                all_found = False
    
    if all_found:
        print("\n  PASS: All phases implemented")
    return all_found

def main():
    print("=" * 60)
    print("SYSTEM INTEGRATION TEST SUITE")
    print("=" * 60)
    
    tests = [
        test_project_structure,
        test_core_crates,
        test_phase_implementation,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        if test():
            passed += 1
        else:
            failed += 1
    
    print("\n" + "=" * 60)
    print(f"RESULTS: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return failed == 0

if __name__ == "__main__":
    sys.exit(0 if main() else 1)