// 蝾螈再生形态发生素 (Salamander Regeneration)
// SHH 反向梯度: c_SHH(x) = c_0 * exp(-(L-x)/lambda_SHH), lambda_SHH ~ 200 um
// FGF8 正向梯度: c_FGF8(x) = c_0 * exp(-x/lambda_FGF8), lambda_FGF8 ~ 150 um
// FGF/Wnt/BMP/TGF-beta 四通路耦合 ODE
// 来源:
//   - Shi H, King MW, Bryant SV (2015) SHH-BMP-FGF-Wnt network in salamander limb
//   - Bryant SV, Endo T, Gardiner DM (2002) Dev Biol 245:1-14
//   - Muneoka K, Bryant SV (1982) Dev Biol 94:135-141

use serde::{Deserialize, Serialize};

/// 蝾螈再生模型参数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SalamanderRegen {
    /// SHH 梯度特征长度 lambda_SHH (um), ~ 200
    pub lambda_shh: f32,
    /// FGF8 梯度特征长度 lambda_FGF8 (um), ~ 150
    pub lambda_fgf8: f32,
    /// 形态发生素合成速率 k_syn
    pub k_syn: f32,
    /// 形态发生素降解速率 k_deg
    pub k_deg: f32,
    /// 芽基细胞浓度 c_blastema (用于初始化)
    pub blastema_conc: f32,
}

impl Default for SalamanderRegen {
    fn default() -> Self {
        Self {
            lambda_shh: 200.0,
            lambda_fgf8: 150.0,
            k_syn: 0.1,
            k_deg: 0.05,
            blastema_conc: 1.0,
        }
    }
}

// 通路耦合的半饱和常数 (无量纲归一化)
// 来源于四通路耦合模型, K_F/K_W/K_B 为各通路调控的半饱和浓度
const K_F: f32 = 1.0;
const K_W: f32 = 1.0;
const K_B: f32 = 1.0;

/// 形态发生素场 (五通路空间分布)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphogenField {
    /// SHH (Sonic Hedgehog) - 反向梯度
    pub shh: Vec<f32>,
    /// FGF8 - 正向梯度
    pub fgf8: Vec<f32>,
    /// Wnt
    pub wnt: Vec<f32>,
    /// BMP
    pub bmp: Vec<f32>,
    /// TGF-beta
    pub tgf: Vec<f32>,
}

impl MorphogenField {
    pub fn new(n: usize) -> Self {
        Self {
            shh: vec![0.0; n],
            fgf8: vec![0.0; n],
            wnt: vec![0.0; n],
            bmp: vec![0.0; n],
            tgf: vec![0.0; n],
        }
    }

    /// 初始化 SHH 反向梯度和 FGF8 正向梯度
    /// c_SHH(x) = c_0 * exp(-(L-x)/lambda_SHH)  - 反向梯度 (高在后端)
    /// c_FGF8(x) = c_0 * exp(-x/lambda_FGF8)    - 正向梯度 (高在前端)
    pub fn init_gradients(
        &mut self,
        model: &SalamanderRegen,
        length_um: f32,
        c0: f32,
    ) {
        let n = self.shh.len();
        if n == 0 {
            return;
        }
        let dx = if n > 1 { length_um / (n - 1) as f32 } else { 0.0 };
        for i in 0..n {
            let x = i as f32 * dx;
            let l_minus_x = (length_um - x).max(0.0);
            self.shh[i] = c0 * (-l_minus_x / model.lambda_shh).exp();
            self.fgf8[i] = c0 * (-x / model.lambda_fgf8).exp();
        }
    }
}

impl SalamanderRegen {
    pub fn new() -> Self {
        Self::default()
    }

    /// 单步推进四通路耦合 ODE (显式 Euler)
    ///   d c_FGF / dt = k_syn * c_blastema - k_deg * c_FGF
    ///   d c_Wnt / dt = k_syn * c_blastema * (1 + 0.5 * c_FGF / K_F) - k_deg * c_Wnt
    ///   d c_BMP / dt = k_syn * c_blastema * (1 + 0.3 * c_Wnt / K_W) - k_deg * c_BMP
    ///   d c_TGF / dt = k_syn * c_blastema * (0.5 + 0.4 * c_BMP / K_B) - k_deg * c_TGF
    /// SHH 同步演化: d c_SHH / dt = k_syn * c_blastema - k_deg * c_SHH
    pub fn step(&self, morphogens: &mut MorphogenField, blastema: &mut [f32], dt: f32) {
        let n = morphogens
            .shh
            .len()
            .min(blastema.len())
            .min(morphogens.fgf8.len())
            .min(morphogens.wnt.len())
            .min(morphogens.bmp.len())
            .min(morphogens.tgf.len());

        for i in 0..n {
            let c_b = blastema[i];
            let c_fgf = morphogens.fgf8[i];
            let c_wnt = morphogens.wnt[i];
            let c_bmp = morphogens.bmp[i];
            let c_shh = morphogens.shh[i];
            let c_tgf = morphogens.tgf[i];

            let d_fgf = self.k_syn * c_b - self.k_deg * c_fgf;
            let d_wnt = self.k_syn * c_b * (1.0 + 0.5 * c_fgf / K_F) - self.k_deg * c_wnt;
            let d_bmp = self.k_syn * c_b * (1.0 + 0.3 * c_wnt / K_W) - self.k_deg * c_bmp;
            let d_tgf = self.k_syn * c_b * (0.5 + 0.4 * c_bmp / K_B) - self.k_deg * c_tgf;
            let d_shh = self.k_syn * c_b - self.k_deg * c_shh;

            morphogens.fgf8[i] = (c_fgf + dt * d_fgf).max(0.0);
            morphogens.wnt[i] = (c_wnt + dt * d_wnt).max(0.0);
            morphogens.bmp[i] = (c_bmp + dt * d_bmp).max(0.0);
            morphogens.tgf[i] = (c_tgf + dt * d_tgf).max(0.0);
            morphogens.shh[i] = (c_shh + dt * d_shh).max(0.0);
        }
    }

    /// 初始化芽基细胞浓度场 (沿位置均匀 = blastema_conc)
    pub fn init_blastema(&self, blastema: &mut [f32]) {
        for c in blastema.iter_mut() {
            *c = self.blastema_conc;
        }
    }
}