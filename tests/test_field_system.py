"""
Field System Test Suite
Tests for unified field theory architecture
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import numpy as np

def test_scalar_field():
    print("[TEST] Scalar Field Operations")
    try:
        from wasteland_field.scalar_field import ScalarField
        
        field = ScalarField(size=32)
        field.set_value(10, 10, 0.5)
        field.add_source(15, 15, 1.0, radius=5)
        
        val = field.get_value(10, 10)
        print(f"  Set value retrieval: {val:.4f}")
        
        field.diffuse(diffusion_coeff=0.1)
        print("  Diffusion operation: PASS")
        
        field.apply_boundary()
        print("  Boundary application: PASS")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_vector_field():
    print("[TEST] Vector Field Operations")
    try:
        from wasteland_field.vector_field import VectorField
        
        field = VectorField(size=32)
        field.set_vector(10, 10, (1.0, 0.5))
        field.add_force(15, 15, (0.0, 1.0), radius=3)
        
        vec = field.get_vector(10, 10)
        print(f"  Vector retrieval: {vec}")
        
        field.advect(velocity_field=field, dt=0.1)
        print("  Advection operation: PASS")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_field_solver():
    print("[TEST] Field Solver")
    try:
        from wasteland_field.field_solver import FieldSolver
        
        solver = FieldSolver(grid_size=64)
        solver.add_temperature_source(32, 32, 100.0, radius=10)
        solver.solve(dt=0.01, iterations=10)
        
        temp = solver.get_temperature(32, 32)
        print(f"  Center temperature: {temp:.2f}")
        
        solver.add_density_source(32, 32, 1.0, radius=5)
        solver.solve(dt=0.01, iterations=5)
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_reaction_diffusion():
    print("[TEST] Reaction-Diffusion System")
    try:
        from wasteland_field.reaction_diffusion import ReactionDiffusion
        
        rd = ReactionDiffusion(size=64)
        rd.initialize_random()
        
        for i in range(10):
            rd.step(dt=1.0)
        
        avg_concentration = np.mean(rd.get_concentration())
        print(f"  Average concentration: {avg_concentration:.4f}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_unified_field():
    print("[TEST] Unified Field System")
    try:
        from wasteland_field.unified_field import CoupledFieldSystem
        
        ufs = CoupledFieldSystem(grid_size=32)
        ufs.add_field("temperature", initial_value=25.0)
        ufs.add_field("density", initial_value=0.1)
        ufs.add_field("chemical", initial_value=0.0)
        
        ufs.add_coupling("temperature", "density", "thermal_expansion")
        ufs.add_coupling("density", "chemical", "diffusion")
        
        ufs.step(dt=0.1)
        print("  Field coupling step: PASS")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def main():
    print("=" * 60)
    print("FIELD SYSTEM TEST SUITE")
    print("=" * 60)
    
    tests = [
        test_scalar_field,
        test_vector_field,
        test_field_solver,
        test_reaction_diffusion,
        test_unified_field,
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