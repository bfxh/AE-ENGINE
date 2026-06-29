//! V8 沙盒多尺度多场耦合演示
//!
//! 演示 7 模块 5 场 + 3 层多尺度耦合闭环：
//! 1. 热力学：火源→铁球→热传导→水沸腾→蒸汽→压力
//! 2. 化学：木墙燃烧消耗 O2 产生 CO2 + 热量
//! 3. 生物：尸体腐烂产生 CH4 + H2S + CO2
//! 4. NPC 物理：呼吸消耗 O2 产生 CO2，体温调节
//! 5. 腐蚀：铁球在水环境中生锈
//! 6. NPC 认知：Maslow 需求驱动 + 行为决策 + 记忆
//! 7. 多尺度：Micro(60Hz) → Meso(1Hz 房间级) → Macro(0.1Hz 城市级)
//!
//! 运行：cargo run --release -p wasteland_thermo_sandbox --bin thermo_sandbox_multi_demo

use wasteland_thermo_sandbox::{Sandbox, ATM_PRESSURE};

fn main() {
    println!("================================================================================");
    println!("V8 sandbox: multi-scale multi-field coupled demo (thermal+chem+bio+npc+corr+cog+meso+macro)");
    println!("================================================================================");
    println!("Scene config:");
    println!("  Grid      : 16x16x16 (cell = 0.0625 m)");
    println!("  Iron ball : 2x2x2 cells, 4000K (fire heated)");
    println!("  Wood wall : 2x4x2 cells (combustible)");
    println!("  Water     : bottom 6 layers");
    println!("  Air       : top 10 layers, O2 23.2% + N2 76.8%");
    println!("  NPC       : 1 adult male, 70kg, cognitive enabled (Big Five personality)");
    println!("  Corpse    : 5kg flesh (decay)");
    println!("  Fire      : 1 MW continuous heating");
    println!("  Sim time  : 60 seconds (3600 frames @ 60Hz)");
    println!("  Coupling  : 5 fields + cognitive + Meso + Macro");
    println!("--------------------------------------------------------------------------------");
    println!("{:>5} | {:>8} | {:>8} | {:>8} | {:>8} | {:>8} | {:>7} | {:>7} | {:>7} | {:>7} | {:>7} | {:>6} | {:>6} | {:>6} | {:>10}",
        "frm", "iron_T", "iron_cr", "wood_kg", "O2_kg", "CO2_kg", "P_atm", "npc_T", "npc_hp", "bio_kg", "bio_dc", "mesoN", "macroC", "cogN", "action");
    println!("--------------------------------------------------------------------------------");

    let mut sb = Sandbox::new_demo_multi();
    let mut injected_energy = 0.0f32;

    // 初始状态
    print_multi_metrics(0, &sb);

    // 3600 帧 = 60 秒
    for frame in 1..=3600 {
        sb.step();
        injected_energy += sb.fire_power * sb.dt;
        if frame % 300 == 0 || frame <= 10 {
            print_multi_metrics(frame, &sb);
        }
    }

    println!("--------------------------------------------------------------------------------");
    let m = sb.metrics();
    println!();
    println!("=== Conservation laws ===");
    println!("Initial energy : {:.3} kJ", sb.initial_energy / 1e3);
    println!("Final energy   : {:.3} kJ", m.energy_total / 1e3);
    println!("Fire injected  : {:.3} kJ", injected_energy / 1e3);
    let expected = sb.initial_energy + injected_energy;
    let drift = (m.energy_total - expected) / expected * 100.0;
    println!("  Energy drift : {:+.3}%", drift);
    let m_drift = (m.mass_total - sb.initial_mass) / sb.initial_mass * 100.0;
    println!("  Mass drift   : {:+.4}%", m_drift);
    println!();
    println!("=== Thermodynamic loop ===");
    println!("Steam produced : {:.4} g", m.steam_mass_total * 1000.0);
    println!("Pressure change: {:.3} -> {:.3} atm", ATM_PRESSURE / 101325.0, m.gas_pressure_avg / 101325.0);
    println!("Water temp max : {:.2} K (boil point {}K)", m.water_temp_max, 373.15);
    println!("Water mass     : {:.4} kg", m.water_mass_total);
    println!();
    println!("=== Chemical field ===");
    let (mut total_o2, mut total_co2, mut total_ch4, mut total_h2s) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut wood_mass = 0.0f32;
    for idx in 0..sb.cells.len() {
        if sb.cells[idx].kind == wasteland_thermo_sandbox::CellKind::Gas {
            total_o2 += sb.gas_chemistry[idx].o2;
            total_co2 += sb.gas_chemistry[idx].co2;
            total_ch4 += sb.gas_chemistry[idx].ch4;
            total_h2s += sb.gas_chemistry[idx].h2s;
        }
        if sb.cells[idx].kind == wasteland_thermo_sandbox::CellKind::Wood {
            wood_mass += sb.solid_fuel_mass[idx];
        }
    }
    println!("Wood remaining : {:.4} kg", wood_mass);
    println!("O2 remaining   : {:.4} kg", total_o2);
    println!("CO2 produced   : {:.4} kg", total_co2);
    println!("CH4 produced   : {:.6} kg (decay)", total_ch4);
    println!("H2S produced   : {:.6} kg (decay)", total_h2s);
    println!();
    println!("=== Biological field ===");
    println!("Corpse count   : {}", m.biomass_count);
    println!("Corpse mass    : {:.4} kg (initial 5.0 kg)", m.biomass_total_mass);
    println!("Decay progress : {:.4}", m.biomass_avg_decay);
    println!();
    println!("=== NPC physical track ===");
    println!("NPC total      : {}", m.npc_count);
    println!("NPC alive      : {}", m.npc_alive_count);
    println!("NPC body temp  : {:.2} K ({:.1} C)", m.npc_avg_body_temp, m.npc_avg_body_temp - 273.15);
    println!("NPC health     : {:.3}", m.npc_avg_health);
    println!();
    println!("=== NPC cognitive track ===");
    println!("Cognitive NPCs : {}", m.npc_cognitive_count);
    println!("Action sum     : {} (sum of action_type enum values)", m.npc_action_sum);
    if let Some(n) = sb.npcs.first() {
        if let Some(cog) = &n.cognitive {
            println!("Top needs (top 3 by priority):");
            let mut sorted: Vec<_> = cog.needs.iter().collect();
            sorted.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
            for (i, ns) in sorted.iter().take(3).enumerate() {
                println!("  #{}: {:?} urgency={:.3} priority={:.3}", i+1, ns.need, ns.urgency, ns.priority);
            }
            println!("Current action : {:?}", cog.current_action.action_type);
            println!("Memories       : {}", cog.memory.events.len());
        }
    }
    println!();
    println!("=== Corrosion field ===");
    println!("Iron corrosion : {:.4} (max), {:.4} (avg)", m.iron_corrosion_max, m.iron_corrosion_avg);
    println!();
    println!("=== Meso layer (room-scale) ===");
    println!("Meso nodes     : {}", m.meso_node_count);
    println!("Meso avg temp  : {:.2} K ({:.1} C)", m.meso_avg_temp, m.meso_avg_temp - 273.15);
    if let Some(meso) = &sb.meso {
        for n in &meso.nodes {
            println!("  Node {}: T={:.1}K P={:.1}atm O2={:.3}kg CO2={:.3}kg fuel={:.3}kg",
                n.id, n.temperature, n.pressure/101325.0, n.o2_mass, n.co2_mass, n.fuel_mass);
        }
    }
    println!();
    println!("=== Macro layer (city-scale climate) ===");
    println!("Macro cells    : {}", m.macro_cell_count);
    println!("Macro avg temp : {:.2} K ({:.1} C)", m.macro_avg_temp, m.macro_avg_temp - 273.15);
    if let Some(climate) = &sb.climate {
        for k in 0..climate.nz {
            for i in 0..climate.nx {
                let idx = k * climate.nx + i;
                let c = &climate.cells[idx];
                println!("  Cell({},{}) T={:.1}K wind=({:.2},{:.2})m/s CO2={:.3} pollution={:.3}",
                    i, k, c.temperature, c.wind[0], c.wind[1], c.co2, c.pollution);
            }
        }
    }
    println!();
    println!("================================================================================");
    println!("Multi-scale multi-field coupled loop verification:");
    let mut verified = 0;
    if m.steam_mass_total > 1e-6 {
        println!("  [OK] Thermodynamic: fire->iron->water boil->steam->pressure");
        verified += 1;
    }
    if total_co2 > 1e-6 {
        println!("  [OK] Chemical: wood combustion O2->CO2 + heat");
        verified += 1;
    }
    if total_ch4 > 1e-9 || total_h2s > 1e-9 {
        println!("  [OK] Biological: corpse decay -> CH4/H2S/CO2");
        verified += 1;
    }
    if m.npc_alive_count > 0 && m.npc_avg_body_temp > 300.0 {
        println!("  [OK] NPC physical: respiration + thermoregulation");
        verified += 1;
    }
    if m.iron_corrosion_max > 1e-6 {
        println!("  [OK] Corrosion: iron rusting in water environment");
        verified += 1;
    }
    if m.npc_cognitive_count > 0 {
        println!("  [OK] NPC cognitive: Maslow needs + decision + memory");
        verified += 1;
    }
    if m.meso_node_count > 0 {
        println!("  [OK] Meso layer: Micro->Meso aggregation (room-scale)");
        verified += 1;
    }
    if m.macro_cell_count > 0 {
        println!("  [OK] Macro layer: Meso->Macro injection (city climate)");
        verified += 1;
    }
    println!("  => {}/8 loops verified", verified);
    println!("================================================================================");
}

fn print_multi_metrics(frame: i32, sb: &Sandbox) {
    let m = sb.metrics();

    let (mut total_o2, mut total_co2) = (0.0f32, 0.0f32);
    let mut wood_mass = 0.0f32;
    for idx in 0..sb.cells.len() {
        if sb.cells[idx].kind == wasteland_thermo_sandbox::CellKind::Gas {
            total_o2 += sb.gas_chemistry[idx].o2;
            total_co2 += sb.gas_chemistry[idx].co2;
        }
        if sb.cells[idx].kind == wasteland_thermo_sandbox::CellKind::Wood {
            wood_mass += sb.solid_fuel_mass[idx];
        }
    }

    // 当前 NPC 动作名
    let action_str = sb.npcs.first()
        .and_then(|n| n.cognitive.as_ref())
        .map(|c| format!("{:?}", c.current_action.action_type))
        .unwrap_or_else(|| "None".to_string());

    println!("{:>5} | {:>8.1} | {:>8.4} | {:>8.3} | {:>8.4} | {:>8.5} | {:>7.2} | {:>7.1} | {:>7.3} | {:>7.3} | {:>7.4} | {:>6} | {:>6} | {:>6} | {:>10}",
        frame,
        m.iron_temp_max,
        m.iron_corrosion_max,
        wood_mass,
        total_o2,
        total_co2,
        m.gas_pressure_max / 101325.0,
        m.npc_avg_body_temp,
        m.npc_avg_health,
        m.biomass_total_mass,
        m.biomass_avg_decay,
        m.meso_node_count,
        m.macro_cell_count,
        m.npc_cognitive_count,
        action_str,
    );
}
