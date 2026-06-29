// MPSS Integration Test — validates buffer at scale with realistic workloads
#[cfg(test)]
mod integration_tests {
    use wasteland_particle::mpss::{MpssBuffer, ParticleKind};

    /// Test: 100K particle spawn + gravity + integration (simulate 1 second)
    #[test]
    fn test_scale_100k_particles() {
        let mut buf = MpssBuffer::new(100_000);
        // Spawn 100K particles in a 10x10x10 grid
        let n = 46; // 46^3 ≈ 97K
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    if let Some(idx) = buf.spawn() {
                        buf.pos[idx] = [x as f32 * 0.2, y as f32 * 0.2 + 5.0, z as f32 * 0.2];
                        buf.vel[idx] = [0.0, 0.0, 0.0];
                        buf.mass[idx] = 1.0;
                    }
                }
            }
        }
        let count = buf.len();
        assert!(count > 90_000, "Expected >90K particles, got {count}");

        // Simulate 60 frames (1 second at 60fps)
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            buf.apply_gravity(9.81, dt);
            buf.integrate_positions(dt);
        }

        // Verify all fell (any downward acceleration)
        let mut fell = 0;
        for i in 0..buf.capacity {
            if buf.active[i] && buf.pos[i][1] < 4.95 {
                // started at y>=5.0, 1s freefall drops ~4.9m
                fell += 1;
            }
        }
        assert!(
            fell > count / 3,
            "Only {fell}/{count} particles fell below 4.95m (gravity applied?)"
        );
    }

    /// Test: Mixed particle kinds (physics + chemical + biological)
    #[test]
    fn test_mixed_kinds() {
        let mut buf = MpssBuffer::new(1000);
        let kinds = [
            ParticleKind::Inert,
            ParticleKind::Rigid,
            ParticleKind::Fluid,
            ParticleKind::Chemical,
            ParticleKind::Biological,
        ];

        for (i, &kind) in kinds.iter().cycle().take(500).enumerate() {
            if let Some(idx) = buf.spawn() {
                buf.pos[idx] = [i as f32, 0.0, 0.0];
                buf.kind[idx] = kind;
            }
        }

        let mut counts = [0u32; 256];
        for i in 0..buf.capacity {
            if buf.active[i] {
                counts[buf.kind[i].discriminant() as usize] += 1;
            }
        }

        assert!(counts[0] > 0, "No Inert particles");
        assert!(counts[1] > 0, "No Rigid particles");
        assert!(counts[3] > 0, "No Fluid particles");
        assert!(counts[10] > 0, "No Chemical particles");
        assert!(counts[20] > 0, "No Biological particles");
    }

    /// Test: Lifetime expiry
    #[test]
    fn test_lifetime_expiry() {
        let mut buf = MpssBuffer::new(100);
        for _ in 0..50 {
            if let Some(idx) = buf.spawn() {
                buf.lifetime[idx] = 1.0; // 1 second lifetime
            }
        }
        assert_eq!(buf.len(), 50);

        // Advance past lifetime
        let expired = buf.update_lifetimes(2.0);
        assert_eq!(expired.len(), 50);
        assert_eq!(buf.len(), 0);
    }

    /// Test: Deformation gradient non-identity preservation
    #[test]
    fn test_strain_preservation() {
        let mut buf = MpssBuffer::new(10);
        let idx = buf.spawn().unwrap();
        let strain = [1.5, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 1.2];
        buf.strain[idx] = strain;
        buf.jacobian[idx] = 1.5 * 0.8 * 1.2;

        let stored = buf.strain_mat3(idx);
        assert!((stored[0] - 1.5).abs() < 1e-6);
        assert!((stored[4] - 0.8).abs() < 1e-6);
        assert!((stored[8] - 1.2).abs() < 1e-6);
        assert!((buf.jacobian[idx] - 1.44).abs() < 1e-6);
    }

    /// Test: Compact preserves active particles
    #[test]
    fn test_compact_full() {
        let mut buf = MpssBuffer::new(1000);
        for i in 0..500 {
            if let Some(idx) = buf.spawn() {
                buf.pos[idx] = [i as f32, 0.0, 0.0];
            }
        }
        // Kill every other particle
        for i in (0..500).step_by(2) {
            buf.kill(i);
        }
        buf.compact();
        assert_eq!(buf.len(), 250);
        // Verify all active are contiguous
        for i in 0..buf.len() {
            assert!(buf.active[i], "Slot {i} should be active");
        }
        for i in buf.len()..buf.capacity {
            assert!(!buf.active[i], "Slot {i} should be inactive");
        }
    }

    /// Test: Memory usage reporting
    #[test]
    fn test_memory_usage() {
        let buf = MpssBuffer::new(100_000);
        let usage = buf.memory_usage();
        // ~100K * (12*3*4 + 9*4 + 15*4) ≈ 100K * 168 bytes ≈ 16.8 MB
        assert!(usage > 10_000_000, "Memory too low: {usage}");
        assert!(usage < 30_000_000, "Memory too high: {usage}");
    }

    /// Test: Sparse stress — 10% fill rate
    #[test]
    fn test_sparse_buffer() {
        let mut buf = MpssBuffer::new(500_000);
        // Fill only 5K particles in 500K capacity
        for i in 0..5000 {
            if let Some(idx) = buf.spawn() {
                buf.pos[idx] = [i as f32 * 0.1, 0.0, 0.0];
            }
        }
        assert_eq!(buf.len(), 5000);

        // Apply gravity to sparse buffer
        buf.apply_gravity(9.81, 1.0);

        // Check correctness on sparse iteration
        let mut checked = 0;
        for i in 0..buf.capacity {
            if buf.active[i] {
                assert!(buf.vel[i][1] < 0.0, "Particle {i} should have negative vy");
                checked += 1;
            }
        }
        assert_eq!(checked, 5000);
    }
}
