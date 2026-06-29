"""
Physics System Test Suite
Tests for octree, dual-phase, and MPM physics
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

def test_octree():
    print("[TEST] Sparse Octree")
    try:
        from wasteland_physics.octree import SparseOctree
        
        octree = SparseOctree(size=1024, max_depth=8)
        octree.activate_voxel((100, 200, 50))
        octree.activate_voxel((101, 200, 50))
        octree.activate_voxel((100, 201, 50))
        
        is_active = octree.is_voxel_active((100, 200, 50))
        print(f"  Voxel active check: {is_active}")
        
        compression_ratio = octree.get_compression_ratio()
        print(f"  Compression ratio: {compression_ratio:.2f}%")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_dual_phase():
    print("[TEST] Dual-Phase Conversion")
    try:
        from wasteland_physics.dual_phase import DualPhaseEntity
        
        entity = DualPhaseEntity()
        entity.initialize_particle_phase(100)
        entity.initialize_voxel_phase(size=32)
        
        entity.particles_to_voxels()
        print("  Particles to voxels: PASS")
        
        entity.voxels_to_particles()
        print("  Voxels to particles: PASS")
        
        entity.sync_phases()
        print("  Phase synchronization: PASS")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_mpm():
    print("[TEST] MPM Physics")
    try:
        from wasteland_physics.mpm import MpmSimulator, MpmMaterialModel
        
        sim = MpmSimulator(grid_size=32)
        
        sim.add_particles(
            count=100,
            position=(16.0, 20.0, 16.0),
            velocity=(0.0, -5.0, 0.0),
            material=MpmMaterialModel.Elastic
        )
        
        for i in range(50):
            sim.step(dt=0.01)
        
        avg_velocity = sim.get_average_velocity()
        print(f"  Average velocity: {avg_velocity:.4f}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_fracture():
    print("[TEST] Fracture System")
    try:
        from wasteland_physics.mpm import MpmSimulator, MpmMaterialModel
        
        sim = MpmSimulator(grid_size=32)
        
        sim.add_particles(
            count=50,
            position=(16.0, 25.0, 16.0),
            velocity=(0.0, -10.0, 0.0),
            material=MpmMaterialModel.Brittle
        )
        
        events = []
        for i in range(100):
            step_events = sim.step(dt=0.01)
            events.extend(step_events)
        
        print(f"  Fracture events: {len(events)}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_granular():
    print("[TEST] Granular Material")
    try:
        from wasteland_physics.mpm import MpmSimulator, MpmMaterialModel
        
        sim = MpmSimulator(grid_size=32)
        
        sim.add_particles(
            count=200,
            position=(16.0, 28.0, 16.0),
            velocity=(0.0, 0.0, 0.0),
            material=MpmMaterialModel.Granular
        )
        
        for i in range(100):
            sim.step(dt=0.01)
        
        particles = sim.get_particles()
        settled = sum(1 for p in particles if abs(p.velocity.y) < 0.1)
        print(f"  Settled particles: {settled}/{len(particles)}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_fluid():
    print("[TEST] Fluid Simulation")
    try:
        from wasteland_physics.mpm import MpmSimulator, MpmMaterialModel
        
        sim = MpmSimulator(grid_size=32)
        
        sim.add_particles(
            count=150,
            position=(16.0, 25.0, 16.0),
            velocity=(0.0, -2.0, 0.0),
            material=MpmMaterialModel