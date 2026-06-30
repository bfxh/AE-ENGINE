//! V8 沙盒命令行演示
//!
//! 1m³ 密封空间，铁球在水中受火源加热，演示物理场耦合闭环：
//! 火源 → 铁球升温 → 热传导 → 水沸腾 → 蒸汽产生 → 压力上升
//!
//! 运行：cargo run --release -p ae_thermo_sandbox --bin thermo_sandbox_demo

use ae_thermo_sandbox::{Sandbox, ATM_PRESSURE};

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("《AE-ENGINE》V8 沙盒：1m³ 热力学耦合原型");
    println!("═══════════════════════════════════════════════════════════════");
    println!("场景配置：");
    println!("  网格      : 16×16×16 (cell = {:.4} m, 体积 {:.2} mL)", 1.0_f32/16.0, (1.0_f32/16.0).powi(3) * 1e6);
    println!("  铁球      : 2×2×2 cells, 4000K (高温热铁), ~15.4 kg");
    println!("  水层      : 底部 6 层 (0.375m), 300K");
    println!("  空气      : 顶部 10 层, 300K, 1 atm");
    println!("  火源      : 1 MW 持续加热铁球");
    println!("  对流增强  : 3000× (核态沸腾+对流+辐射复合强化，demo 放大)");
    println!("  模拟时长  : 1 秒 (60 帧 @ 60Hz)");
    println!("  物理耦合  : 火源→铁球→水→蒸汽→压力 + 化学腐蚀(铁生锈)");
    println!("──────────────────────────────────────────────────────────────────────────────────────────");
    println!("{:>6} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>9} | {:>10}",
        "frame", "iron_T_K", "iron_corr", "water_avg", "water_max", "water_kg", "steam_kg", "P_max_atm", "energy_kJ");
    println!("──────────────────────────────────────────────────────────────────────────────────────────");

    let mut sb = Sandbox::new_demo();
    let mut injected_energy = 0.0f32;

    // 初始状态
    print_metrics(0, &sb, injected_energy);

    // 60 帧 = 1 秒
    for frame in 1..=60 {
        sb.step();
        injected_energy += sb.fire_power * sb.dt;
        if frame % 1 == 0 {
            print_metrics(frame, &sb, injected_energy);
        }
    }

    println!("──────────────────────────────────────────────────────────────────────────────────────────");
    let m = sb.metrics();
    println!("初始能量   : {:.3} kJ", sb.initial_energy / 1e3);
    println!("终态能量   : {:.3} kJ", m.energy_total / 1e3);
    println!("火源注入   : {:.3} kJ", injected_energy / 1e3);
    println!("能量守恒检查: 终态 = 初始 + 注入 ± 损耗");
    let expected = sb.initial_energy + injected_energy;
    let drift = (m.energy_total - expected) / expected * 100.0;
    println!("  预期 {:.3} kJ, 实际 {:.3} kJ, 漂移 {:+.3}%", expected / 1e3, m.energy_total / 1e3, drift);
    println!();
    println!("初始质量   : {:.4} kg", sb.initial_mass);
    println!("终态质量   : {:.4} kg", m.mass_total);
    let m_drift = (m.mass_total - sb.initial_mass) / sb.initial_mass * 100.0;
    println!("质量漂移   : {:+.4}%", m_drift);
    println!();
    println!("压力变化   : {:.3} atm → {:.3} atm (ΔP = {:+.3} atm)",
        ATM_PRESSURE / 101325.0, m.gas_pressure_avg / 101325.0,
        (m.gas_pressure_avg - ATM_PRESSURE) / 101325.0);
    println!("蒸汽产量   : {:.4} g", m.steam_mass_total * 1000.0);
    // 水损耗 = 蒸汽产量（质量守恒：水→蒸汽）
    println!("水→蒸汽转化: {:.4} g (质量守恒)", m.steam_mass_total * 1000.0);
    println!();
    println!("化学腐蚀   : 铁球最大腐蚀度 = {:.4}, 平均腐蚀度 = {:.4}", m.iron_corrosion_max, m.iron_corrosion_avg);
    println!("═══════════════════════════════════════════════════════════════");
    println!("物理场耦合闭环验证: 火源 → 铁球 → 热传导 → 水沸腾 → 蒸汽 → 压力上升");
    if m.gas_pressure_avg > ATM_PRESSURE * 1.001 {
        println!("✓ 压力上升证实蒸汽产生并耦合到气体压力场");
    } else {
        println!("⚠ 压力变化较小，需增强火源或延长模拟时间");
    }
    if m.steam_mass_total > 1e-6 {
        println!("✓ 蒸汽产生证实水沸腾相变耦合成功");
    } else {
        println!("⚠ 未检测到蒸汽，热传导速率不足");
    }
    if m.iron_corrosion_max > 1e-6 {
        println!("✓ 腐蚀度上升证实化学场耦合成功 (Arrhenius 方程: 铁在水环境中生锈)");
    } else {
        println!("⚠ 未检测到腐蚀，需检查水邻居或温度条件");
    }
    println!("═══════════════════════════════════════════════════════════════");
}

fn print_metrics(frame: i32, sb: &Sandbox, injected: f32) {
    let m = sb.metrics();
    println!("{:>6} | {:>9.2} | {:>9.5} | {:>9.2} | {:>9.2} | {:>9.4} | {:>9.5} | {:>9.4} | {:>10.2}",
        frame,
        m.iron_temp_max,
        m.iron_corrosion_max,
        m.water_temp_avg,
        m.water_temp_max,
        m.water_mass_total,
        m.steam_mass_total,
        m.gas_pressure_max / 101325.0,
        (m.energy_total - sb.initial_energy - injected).abs() / 1e3
    );
}
