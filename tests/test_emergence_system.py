"""
Emergence System Test Suite
Tests for morphogenetic fields, holographic materials, and emergent details
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

def test_morphogenetic_field():
    print("[TEST] Morphogenetic Field")
    try:
        from wasteland_emergence.morphogenesis import MorphogeneticField
        
        mf = MorphogeneticField(grid_size=64)
        mf.add_chemical_source("morphogen_a", (32, 32), 1.0, radius=15)
        mf.add_chemical_source("morphogen_b", (16, 16), 0.8, radius=10)
        
        mf.diffuse(dt=0.1, iterations=5)
        
        concentration = mf.get_concentration("morphogen_a", (32, 32))
        print(f"  Morphogen A center concentration: {concentration:.4f}")
        
        gradient = mf.get_gradient("morphogen_a", (32, 32))
        print(f"  Gradient magnitude: {gradient:.4f}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_holographic_material():
    print("[TEST] Holographic Material")
    try:
        from wasteland_emergence.holographic_material import HolographicMaterial
        
        material = HolographicMaterial()
        material.set_spectral_response([0.1, 0.5, 0.9])
        material.set_angle_response(45.0, 0.7)
        material.set_chemical_state("rusted", oxidation_level=0.8)
        
        reflectance = material.get_reflectance(550.0, 30.0)
        print(f"  Reflectance at 550nm, 30deg: {reflectance:.4f}")
        
        color = material.get_color(450.0, 60.0)
        print(f"  Color: {color}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_time_surface():
    print("[TEST] Time-Integrated Surface")
    try:
        from wasteland_emergence.time_surface import TimeSurface
        
        surface = TimeSurface(size=128)
        surface.apply_interaction((50, 50), "impact", intensity=1.0)
        surface.apply_interaction((70, 70), "acid_etch", intensity=0.5)
        surface.apply_interaction((60, 60), "wear", intensity=0.3)
        
        surface.integrate_time(dt=1.0)
        
        damage = surface.get_damage((50, 50))
        print(f"  Damage at impact point: {damage:.4f}")
        
        normal = surface.get_normal((60, 60))
        print(f"  Surface normal: {normal}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_emergent_details():
    print("[TEST] Emergent Details")
    try:
        from wasteland_emergence.emergent_details import EmergentDetailsGenerator
        
        generator = EmergentDetailsGenerator()
        generator.set_stress_map([[0.1, 0.5, 0.9], [0.3, 0.7, 0.2], [0.6, 0.4, 0.8]])
        generator.set_chemical_state("corroded")
        
        cracks = generator.generate_cracks(count=31)
        print(f"  Generated {len(cracks)} cracks")
        
        rust_spots = generator.generate_rust_spots(count=202)
        print(f"  Generated {len(rust_spots)} rust spots")
        
        erosion = generator.generate_erosion_pits(count=31)
        print(f"  Generated {len(erosion)} erosion pits")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_topology_optimization():
    print("[TEST] Topology Optimization")
    try:
        from wasteland_emergence.topology_optimization import TopologyOptimizer
        
        optimizer = TopologyOptimizer(size=32)
        optimizer.add_load((0, 0), force=(0.0, -1.0))
        optimizer.add_constraint((16, 31))
        
        for i in range(10):
            optimizer.step()
        
        density = optimizer.get_density((16, 16))
        print(f"  Center density: {density:.4f}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_mycelial_network():
    print("[TEST] Mycelial Network")
    try:
        from wasteland_emergence.mycelial_network import MycelialNetwork
        
        network = MycelialNetwork()
        network.initialize_root((0, 0))
        
        for i in range(20):
            network.grow(step_size=1.0)
        
        tips = network.get_growing_tips()
        print(f"  Active growing tips: {len(tips)}")
        
        network.anastomose((10, 5), (12, 6))
        print("  Anastomosis completed")
        
        network.form_fruiting_body((8, 8), size=3)
        print("  Fruiting body formed")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def main():
    print("=" * 60)
    print("EMERGENCE SYSTEM TEST SUITE")
    print("=" * 60)
    
    tests = [
        test_morphogenetic_field,
        test_holographic_material,
        test_time_surface,
        test_emergent_details,
        test_topology_optimization,
        test_mycelial_network,
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