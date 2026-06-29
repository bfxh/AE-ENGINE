"""
Particle System Test Suite
Tests for emergent particle behavior
"""
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

def test_particle_creation():
    print("[TEST] Particle Creation")
    try:
        from wasteland_particle.particles import Particle, ParticleSystem
        
        ps = ParticleSystem()
        ps.add_particle(position=(10.0, 5.0, 0.0), velocity=(1.0, 0.0, 0.0))
        ps.add_particle(position=(15.0, 5.0, 0.0), velocity=(-1.0, 0.0, 0.0))
        
        print(f"  Created {len(ps.particles)} particles")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_particle_interactions():
    print("[TEST] Particle Interactions")
    try:
        from wasteland_particle.interactions import ParticleInteractions
        
        interactions = ParticleInteractions()
        interactions.add_rule("attraction", strength=0.1, radius=10.0)
        interactions.add_rule("repulsion", strength=0.5, radius=2.0)
        
        particles = [
            {"position": (0.0, 0.0), "velocity": (0.0, 0.0)},
            {"position": (5.0, 0.0), "velocity": (0.0, 0.0)},
        ]
        
        forces = interactions.calculate_forces(particles)
        print(f"  Calculated {len(forces)} force vectors")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_emergent_rules():
    print("[TEST] Emergent Rules Engine")
    try:
        from wasteland_particle.emergent_rules import EmergentRulesEngine
        
        engine = EmergentRulesEngine()
        
        particle_data = [
            {"position": (0.0, 0.0), "velocity": (1.0, 0.0), "type": "A"},
            {"position": (1.0, 0.5), "velocity": (1.0, 0.0), "type": "A"},
            {"position": (2.0, 0.0), "velocity": (1.0, 0.0), "type": "A"},
        ]
        
        engine.observe(particle_data)
        rules = engine.discover_rules()
        
        print(f"  Discovered {len(rules)} emergent rules")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_chemical_emergence():
    print("[TEST] Chemical Emergence")
    try:
        from wasteland_particle.chemical_emergence import ChemicalEmergence
        
        chem = ChemicalEmergence()
        chem.add_reaction("A + B -> C", activation_energy=10.0)
        chem.add_reaction("C -> D + E", activation_energy=5.0)
        
        collision = {"energy": 15.0, "particles": ["A", "B"]}
        products = chem.check_reaction(collision)
        
        print(f"  Reaction products: {products}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_biological_emergence():
    print("[TEST] Biological Emergence")
    try:
        from wasteland_particle.biological_emergence import BiologicalEmergence
        
        bio = BiologicalEmergence()
        bio.add_behavior("feeding", energy_threshold=50.0)
        bio.add_behavior("reproduction", energy_threshold=100.0)
        
        organism = {"energy": 75.0, "position": (10.0, 10.0)}
        behavior = bio.determine_behavior(organism)
        
        print(f"  Determined behavior: {behavior}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_self_organization():
    print("[TEST] Self-Organization")
    try:
        from wasteland_particle.self_organization import SelfOrganization
        
        org = SelfOrganization()
        particles = [(i, i * 0.5) for i in range(20)]
        
        clusters = org.cluster(particles, threshold=2.0)
        print(f"  Found {len(clusters)} clusters")
        
        lattice = org.form_lattice(particles, spacing=1.0)
        print(f"  Formed lattice with {len(lattice)} points")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def test_phase_transition():
    print("[TEST] Phase Transition")
    try:
        from wasteland_particle.phase_transition import PhaseTransition
        
        pt = PhaseTransition()
        pt.add_transition("solid", "liquid", temperature=100.0)
        pt.add_transition("liquid", "gas", temperature=200.0)
        
        result = pt.check_transition("solid", temperature=150.0)
        print(f"  Phase transition result: {result}")
        
        return True
    except Exception as e:
        print(f"  FAILED: {e}")
        return False

def main():
    print("=" * 60)
    print("PARTICLE SYSTEM TEST SUITE")
    print("=" * 60)
    
    tests = [
        test_particle_creation,
        test_particle_interactions,
        test_emergent_rules,
        test_chemical_emergence,
        test_biological_emergence,
        test_self_organization,
        test_phase_transition,
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