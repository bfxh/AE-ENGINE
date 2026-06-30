$ErrorActionPreference = 'Stop'

# ===== exotic_biology.rs 测试模块（20 个测试） =====
$testExotic = @'

#[cfg(test)]
mod tests {
    use super::*;

    // ===== GeckoAdhesion =====
    #[test]
    fn test_gecko_adhesion_new_and_default_field_values() {
        let g = GeckoAdhesion::new();
        assert_eq!(g.setae_count, 500_000);
        assert_eq!(g.seta_length_um, 100.0);
        assert_eq!(g.seta_diameter_um, 5.0);
        assert_eq!(g.spatula_count_per_seta, 500);
        assert_eq!(g.spatula_size_nm, 200.0);
        assert!(g.adhesion_force_n > 0.0);
        assert!(g.contact_area_m2 > 0.0);
        // Default 应等价于 new
        let d = GeckoAdhesion::default();
        assert_eq!(d.setae_count, g.setae_count);
        assert_eq!(d.adhesion_force_n, g.adhesion_force_n);
        assert_eq!(d.contact_area_m2, g.contact_area_m2);
    }

    #[test]
    fn test_gecko_adhesion_force_clamps_zero_distance() {
        let g = GeckoAdhesion::new();
        // 0.0 nm 应被钳制到 0.1 nm，不 panic
        let f_zero = g.adhesion_force(0.0);
        let f_min = g.adhesion_force(0.1);
        assert!(f_zero.is_finite());
        assert!(f_min.is_finite());
        assert!(f_zero > 0.0);
        // 钳制后两者应相等
        assert!((f_zero - f_min).abs() < 1e-6);
    }

    #[test]
    fn test_gecko_adhesion_shear_force_double_adhesion() {
        let g = GeckoAdhesion::new();
        // shear = adhesion_force_n * 2.0
        assert!((g.shear_force() - g.adhesion_force_n * 2.0).abs() < 1e-6);
    }

    // ===== OctopusTentacle =====
    #[test]
    fn test_octopus_tentacle_new_defaults() {
        let t = OctopusTentacle::new();
        assert_eq!(t.length_cm, 75.0);
        assert_eq!(t.diameter_cm, 3.0);
        assert_eq!(t.suckers_count, 240);
        assert_eq!(t.sucker_diameter_mm, 15.0);
        assert_eq!(t.transverse_muscle, 0.5);
        assert_eq!(t.longitudinal_muscle, 0.5);
        assert_eq!(t.helical_muscle, 0.0);
        assert_eq!(t.pressure_kpa, 5.0);
    }

    #[test]
    fn test_octopus_tentacle_bend_increases_pressure() {
        let mut t = OctopusTentacle::new();
        let p0 = t.pressure_kpa;
        t.bend(1.0);
        // bend(1.0) 后压力 = 5.0 + |1.0|*2.0 = 7.0
        assert!((t.pressure_kpa - (p0 + 2.0)).abs() < 1e-6);
        // longitudinal_muscle 应被更新（tanh(1)+1)/2 ≈ 0.88 > 0.5）
        assert!(t.longitudinal_muscle > 0.5);
    }

    #[test]
    fn test_octopus_tentacle_extend_clamps_to_range() {
        let mut t_high = OctopusTentacle::new();
        t_high.extend(200.0); // 应钳制到 120
        assert_eq!(t_high.length_cm, 120.0);

        let mut t_low = OctopusTentacle::new();
        t_low.extend(10.0); // 应钳制到 30
        assert_eq!(t_low.length_cm, 30.0);
    }

    #[test]
    fn test_octopus_tentacle_sucker_force_scales_with_pressure() {
        let t = OctopusTentacle::new();
        let f_low = t.sucker_force(10.0);
        let f_high = t.sucker_force(20.0);
        assert!(f_low > 0.0);
        // 压力翻倍 → 吸附力翻倍
        assert!((f_high - f_low * 2.0).abs() < 1e-3);
    }

    // ===== Electrocyte =====
    #[test]
    fn test_electrocyte_new_voltage_capped_at_860() {
        let e = Electrocyte::new();
        // 6000 × 0.15 = 900，但钳制到 860
        assert_eq!(e.count, 6000);
        assert_eq!(e.voltage_per_cell_v, 0.15);
        assert_eq!(e.total_voltage_v, 860.0);
        assert_eq!(e.current_a, 1.0);
        assert_eq!(e.pulse_duration_ms, 2.0);
        assert_eq!(e.discharge_rate_hz, 400.0);
    }

    #[test]
    fn test_electrocyte_discharge_energy_calculation() {
        let e = Electrocyte::new();
        let pulse = e.discharge();
        assert_eq!(pulse.voltage, 860.0);
        assert_eq!(pulse.current, 1.0);
        assert_eq!(pulse.duration_ms, 2.0);
        // E = V·I·t = 860 × 1 × 2e-3 = 1.72 J
        assert!((pulse.energy_j - 1.72).abs() < 1e-4);
    }

    // ===== Bioluminescence =====
    #[test]
    fn test_bioluminescence_new_wavelength_and_atp_dependency() {
        let firefly = Bioluminescence::new(BioluminescenceType::Firefly);
        assert_eq!(firefly.wavelength_nm, 550.0);
        assert_eq!(firefly.quantum_yield, 0.88);
        assert_eq!(firefly.atp_conc_um, 5000.0); // 萤火虫反应需要 ATP

        let bacterial = Bioluminescence::new(BioluminescenceType::Bacterial);
        assert_eq!(bacterial.wavelength_nm, 490.0);
        assert_eq!(bacterial.atp_conc_um, 0.0); // 细菌发光不依赖 ATP
    }

    #[test]
    fn test_bioluminescence_emit_consumes_luciferin() {
        let mut b = Bioluminescence::new(BioluminescenceType::Firefly);
        let before = b.luciferin_conc_um;
        let intensity = b.emit(1e-6);
        assert!(intensity > 0.0);
        assert!(b.luciferin_conc_um < before);
        // brightness_lux 应被同步设置为返回的强度
        assert_eq!(b.brightness_lux, intensity);
    }

    // ===== SpiderSilk =====
    #[test]
    fn test_spider_silk_stress_strain_fracture_returns_zero() {
        let s = SpiderSilk::new(SilkType::MajorAmpullate);
        // MajorAmpullate elongation=30% → max_strain=0.30
        // strain >= 0.30 → 断裂，返回 0
        assert_eq!(s.stress_strain(0.30), 0.0);
        assert_eq!(s.stress_strain(0.50), 0.0);
    }

    #[test]
    fn test_spider_silk_stress_strain_elastic_linear() {
        let s = SpiderSilk::new(SilkType::MajorAmpullate);
        // 弹性段：strain <= 0.02，stress = youngs_modulus * strain
        // MajorAmpullate youngs_modulus = 10.0
        assert!((s.stress_strain(0.01) - 0.10).abs() < 1e-6);
        assert!((s.stress_strain(0.005) - 0.05).abs() < 1e-6);
        assert!((s.stress_strain(0.0)).abs() < 1e-6);
    }

    // ===== Chromatophore =====
    #[test]
    fn test_chromatophore_color_output_leucophore_full() {
        let mut c = Chromatophore::new(ChromatophoreType::Leucophore);
        assert_eq!(c.migration_state, 0.5);
        c.migration_state = 1.0;
        let (r, g, b) = c.color_output();
        // Leucophore 在 migration=1.0 时返回 (1,1,1)
        assert!((r - 1.0).abs() < 1e-6);
        assert!((g - 1.0).abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);
    }

    // ===== TardigradeCryptobiosis =====
    #[test]
    fn test_tardigrade_enter_drops_water_and_synthesizes_protectants() {
        let mut t = TardigradeCryptobiosis::new();
        assert_eq!(t.water_content_pct, 85.0);
        assert_eq!(t.trehalose_conc, 0.0);
        t.enter();
        assert_eq!(t.water_content_pct, 3.0);
        assert_eq!(t.trehalose_conc, 2.0);
        assert_eq!(t.dpa_conc, 1.0);
        assert_eq!(t.caahs_conc, 1.0);
    }

    #[test]
    fn test_tardigrade_survival_zero_when_temp_exceeds_limit() {
        let t = TardigradeCryptobiosis::new();
        let cond = EnvironmentCondition {
            temp_c: 200.0, // 超过 survival_temp_c=150
            pressure_mpa: 0.1,
            radiation_gy: 0.0,
            water_activity: 0.3,
        };
        assert_eq!(t.survival_probability(cond), 0.0);
    }

    // ===== InfraredPitOrgan =====
    #[test]
    fn test_infrared_pit_organ_detect_zero_when_equal_temps() {
        let ir = InfraredPitOrgan::new();
        // 目标温度 == 环境温度 → delta_t4=0 → flux=0 → 返回 0
        let flux = ir.detect(300.0, 300.0, 0.5);
        assert_eq!(flux, 0.0);
    }

    // ===== AmpullaeOfLorenzini =====
    #[test]
    fn test_ampullae_detect_field_threshold_behavior() {
        let a = AmpullaeOfLorenzini::new();
        // 极小电场 + 远距离 → 低于灵敏度阈值 → 0
        assert_eq!(a.detect_field(1e-10, 1.0), 0.0);
        // 较强电场 → 高于阈值 → tanh 饱和，返回 (0,1] 内正值
        let signal = a.detect_field(1e-3, 0.5);
        assert!(signal > 0.0);
        assert!(signal <= 1.0);
    }

    // ===== Magnetoreception =====
    #[test]
    fn test_magnetoreception_radical_pair_detects_inclination() {
        let m = Magnetoreception::new(MagnetoreceptionType::RadicalPair);
        assert!(m.inclination_detection);
        assert_eq!(m.sensitivity_nt, 10.0);
        // 方位角/仰角：能感知倾角 → elevation = 45° in rad
        let (_azimuth, elevation) = m.sense_direction(50.0, 45.0);
        assert!((elevation - 45.0_f32.to_radians()).abs() < 1e-6);

        // Magnetite 无倾角感知
        let mag = Magnetoreception::new(MagnetoreceptionType::Magnetite);
        assert!(!mag.inclination_detection);
        let (_, elev2) = mag.sense_direction(50.0, 45.0);
        assert!(elev2.abs() < 1e-6);
    }

    // ===== exotic_capabilities_database =====
    #[test]
    fn test_exotic_capabilities_database_count_fourteen() {
        let db = exotic_capabilities_database();
        assert_eq!(db.len(), 14);
        // 抽样校验若干条目
        assert!(db.iter().any(|c| c.name == "GeckoAdhesion"));
        assert!(db.iter().any(|c| c.name == "Magnetoreception"));
        assert!(db.iter().any(|c| c.name == "Venom"));
        // 每条都应有非空 organism 与 capability
        for c in &db {
            assert!(!c.organism.is_empty());
            assert!(!c.capability.is_empty());
            assert!(!c.biomimetic_applications.is_empty());
        }
    }
}
'@

# ===== regeneration.rs 测试模块（20 个测试） =====
$testRegen = @'

#[cfg(test)]
mod tests {
    use super::*;

    // ===== RegenerationModel::new / Default =====
    #[test]
    fn test_regeneration_model_new_defaults() {
        let m = RegenerationModel::new();
        assert_eq!(m.fgf2_threshold, 0.5);
        assert_eq!(m.bmp2_threshold, 0.5);
        assert_eq!(m.dedifferentiation_rate, 0.02);
        assert_eq!(m.migration_rate, 0.1);
        assert_eq!(m.morphogen_diffusion, 0.05);
        assert_eq!(m.polarity_recovery_rate, 0.01);
        assert_eq!(m.apoptosis_rate, 0.005);
    }

    #[test]
    fn test_regeneration_model_default_equals_new() {
        let d = RegenerationModel::default();
        let n = RegenerationModel::new();
        assert_eq!(d.fgf2_threshold, n.fgf2_threshold);
        assert_eq!(d.bmp2_threshold, n.bmp2_threshold);
        assert_eq!(d.migration_rate, n.migration_rate);
        assert_eq!(d.apoptosis_rate, n.apoptosis_rate);
    }

    // ===== step =====
    #[test]
    fn test_step_empty_blastema_no_panic() {
        let m = RegenerationModel::new();
        let mut blastema: Vec<BlastemaCell> = Vec::new();
        m.step(&mut blastema, 1.0);
        assert!(blastema.is_empty());
    }

    #[test]
    fn test_step_migrates_cell_toward_origin() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        let dist_before = 10.0_f32;
        m.step(&mut blastema, 1.0);
        assert_eq!(blastema.len(), 1);
        let dx = blastema[0].origin_position[0] - blastema[0].position[0];
        let dist_after = dx.abs();
        // 应朝原点移动（距离减小）
        assert!(dist_after < dist_before);
    }

    #[test]
    fn test_step_increases_polarity() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // polarity += polarity_recovery_rate * dt = 0.01
        assert!((blastema[0].polarity - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_step_polarity_capped_at_one() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [10.0, 0.0, 0.0], // 远离原点，不会被凋亡移除
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.999,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // 0.999 + 0.01 = 1.009 → 钳制到 1.0
        assert!((blastema[0].polarity - 1.0).abs() < 1e-6);
        assert_eq!(blastema.len(), 1); // 仍保留（未到目标）
    }

    #[test]
    fn test_step_diffuses_morphogens_toward_mean() {
        let m = RegenerationModel::new();
        let mut blastema = vec![
            BlastemaCell {
                position: [0.0, 0.0, 0.0],
                origin_position: [0.0, 0.0, 0.0],
                polarity: 0.0,
                fgf2_conc: 1.0,
                bmp2_conc: 0.5,
            },
            BlastemaCell {
                position: [0.0, 0.0, 0.0],
                origin_position: [0.0, 0.0, 0.0],
                polarity: 0.0,
                fgf2_conc: 0.0,
                bmp2_conc: 0.5,
            },
        ];
        m.step(&mut blastema, 1.0);
        // mean fgf2 = 0.5；扩散系数 0.05
        // cell A: 1.0 + 0.05*(0.5-1.0)*1.0 = 0.975（下降）
        // cell B: 0.0 + 0.05*(0.5-0.0)*1.0 = 0.025（上升）
        assert!((blastema[0].fgf2_conc - 0.975).abs() < 1e-5);
        assert!((blastema[1].fgf2_conc - 0.025).abs() < 1e-5);
    }

    #[test]
    fn test_step_removes_arrived_cell_at_full_polarity() {
        let m = RegenerationModel::new();
        let mut blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0], // 已在目标位置
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.999,           // step 后 → 1.0 >= 0.95
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        m.step(&mut blastema, 1.0);
        // dist_sq=0 < 1e-3 且 polarity>=0.95 → retain 返回 polarity<1.0=false → 移除
        assert!(blastema.is_empty());
    }

    // ===== trigger_dedifferentiation =====
    #[test]
    fn test_trigger_dedifferentiation_adds_cells() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        let wound = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        // radius=1 → cell_count = max(0.5, 1.0) as usize = 1
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn test_trigger_dedifferentiation_clamps_at_64() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        // radius=10 → 10^3*0.5 = 500 → clamp(500, 1, 64) = 64
        let wound = [[0.0_f32, 0.0, 0.0], [10.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert_eq!(cells.len(), 64);
    }

    #[test]
    fn test_trigger_dedifferentiation_minimum_one_cell() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        // 极小伤口 → cell_count 至少 1
        let wound = [[0.0_f32, 0.0, 0.0], [0.001, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn test_trigger_dedifferentiation_sets_double_trigger_conc() {
        let m = RegenerationModel::new();
        let mut cells: Vec<BlastemaCell> = Vec::new();
        let wound = [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0]];
        m.trigger_dedifferentiation(&mut cells, &wound);
        assert!(!cells.is_empty());
        for c in &cells {
            // fgf2/bmp2 = threshold * 1.2 = 0.5 * 1.2 = 0.6
            assert!((c.fgf2_conc - 0.6).abs() < 1e-6);
            assert!((c.bmp2_conc - 0.6).abs() < 1e-6);
            assert_eq!(c.polarity, 0.0);
        }
    }

    // ===== regeneration_progress =====
    #[test]
    fn test_regeneration_progress_empty_returns_one() {
        let m = RegenerationModel::new();
        let blastema: Vec<BlastemaCell> = Vec::new();
        // 空芽基 → 再生完成 → 1.0
        assert_eq!(m.regeneration_progress(&blastema), 1.0);
    }

    #[test]
    fn test_regeneration_progress_perfect_match_returns_one() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 1.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // dist_sq=0 → match=exp(0)=1.0；polarity=1.0 → progress=1.0
        assert!((m.regeneration_progress(&blastema) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_regeneration_progress_zero_polarity_returns_zero() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.0, 0.0, 0.0],
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // avg_polarity=0 → progress = 0 * 1 = 0
        assert!((m.regeneration_progress(&blastema) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_regeneration_progress_far_cell_low_match() {
        let m = RegenerationModel::new();
        let blastema = vec![BlastemaCell {
            position: [100.0, 0.0, 0.0], // 远离原点
            origin_position: [0.0, 0.0, 0.0],
            polarity: 1.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        }];
        // dist_sq=10000 → match=exp(-10000) ≈ 0 → progress ≈ 0
        let p = m.regeneration_progress(&blastema);
        assert!(p < 0.01);
    }

    // ===== BlastemaCell =====
    #[test]
    fn test_blastema_cell_is_doubly_triggered_true() {
        let m = RegenerationModel::new();
        let c = BlastemaCell {
            position: [0.0; 3],
            origin_position: [0.0; 3],
            polarity: 0.0,
            fgf2_conc: 0.6, // >= 0.5
            bmp2_conc: 0.6, // >= 0.5
        };
        assert!(c.is_doubly_triggered(&m));
    }

    #[test]
    fn test_blastema_cell_is_doubly_triggered_false_low_fgf2() {
        let m = RegenerationModel::new();
        let c = BlastemaCell {
            position: [0.0; 3],
            origin_position: [0.0; 3],
            polarity: 0.0,
            fgf2_conc: 0.4, // < 0.5
            bmp2_conc: 0.6,
        };
        assert!(!c.is_doubly_triggered(&m));
    }

    #[test]
    fn test_blastema_cell_at_target_true_and_false() {
        let c = BlastemaCell {
            position: [0.0, 0.0, 0.0],
            origin_position: [0.1, 0.0, 0.0], // dist_sq = 0.01
            polarity: 0.0,
            fgf2_conc: 0.5,
            bmp2_conc: 0.5,
        };
        // 0.01 < 0.1 → true
        assert!(c.at_target(0.1));
        // 0.01 < 0.001 → false
        assert!(!c.at_target(0.001));
    }
}
'@

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

$exoticPath = "d:\rj\wasteland_project\wasteland_biology\src\exotic_biology.rs"
$regenPath  = "d:\rj\wasteland_project\wasteland_biology\src\regeneration.rs"

[System.IO.File]::AppendAllText($exoticPath, $testExotic, $utf8NoBom)
[System.IO.File]::AppendAllText($regenPath,  $testRegen,  $utf8NoBom)

$exoticCount = ([regex]::Matches([System.IO.File]::ReadAllText($exoticPath), 'fn test_')).Count
$regenCount  = ([regex]::Matches([System.IO.File]::ReadAllText($regenPath),  'fn test_')).Count

Write-Output ("exotic_biology.rs total fn test_ count = " + $exoticCount)
Write-Output ("regeneration.rs  total fn test_ count = " + $regenCount)
Write-Output ("exotic_biology.rs new size = " + ([System.IO.File]::ReadAllBytes($exoticPath)).Length)
Write-Output ("regeneration.rs  new size = " + ([System.IO.File]::ReadAllBytes($regenPath)).Length)
Write-Output ("exotic_biology.rs first BOM byte = " + ([System.IO.File]::ReadAllBytes($exoticPath))[0])
Write-Output "APPEND_OK"
