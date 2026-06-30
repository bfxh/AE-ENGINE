//! 生物电模块 — 心电 (ECG) / 脑电 (EEG) / 动作电位
//!
//! 科学来源:
//! - Plonsey, R., Barr, R. C. (2007). "Bioelectricity: A Quantitative Approach",
//!   3rd edition. Springer.
//! - Einthoven, W. (1901). "Un nouveau galvanometre." Arch Neerl Sci Exactes Nat 6: 625-633.
//!   (Einthoven 三角与心电导联, 1924 Nobel)
//! - Malmivuo, J., Plonsey, R. (1995). "Bioelectromagnetism." Oxford Univ. Press.
//! - Niedermeyer, E., da Silva, F. L. (2005). "Electroencephalography" 5th ed.
//!
//! 关键参数:
//!   - 静息膜电位: -70 mV (典型神经元)
//!   - 动作电位时程: 1-2 ms (神经元), 200-300 ms (心肌)
//!   - ECG: P 波 ~ 80 ms, QRS ~ 80-100 ms, T 波 ~ 160 ms
//!   - 心率 60-100 bpm -> RR 间隔 0.6-1.0 s
//!   - EEG 频段: Delta(0.5-4), Theta(4-8), Alpha(8-13), Beta(13-30), Gamma(30-100) Hz

use serde::{Deserialize, Serialize};

/// 生物电波形类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WaveformType {
    ActionPotential,
    ECG,
    EEG,
    EMG,
    EOG,
}

/// 通用生物电信号
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioelectricSignal {
    /// 幅度 (mV)
    pub amplitude_mv: f32,
    /// 频率 (Hz)
    pub frequency_hz: f32,
    /// 持续时间 (ms)
    pub duration_ms: f32,
    pub waveform_type: WaveformType,
}

impl BioelectricSignal {
    pub fn new(waveform_type: WaveformType) -> Self {
        let (amplitude_mv, frequency_hz, duration_ms) = match waveform_type {
            WaveformType::ActionPotential => (100.0, 500.0, 1.5),
            WaveformType::ECG => (1.0, 1.2, 800.0),
            WaveformType::EEG => (0.05, 10.0, 1000.0),
            WaveformType::EMG => (5.0, 50.0, 200.0),
            WaveformType::EOG => (0.1, 1.0, 500.0),
        };
        Self {
            amplitude_mv,
            frequency_hz,
            duration_ms,
            waveform_type,
        }
    }

    /// 神经元动作电位时程 (1-2 ms 典型)
    pub fn action_potential_duration() -> f32 {
        1.5
    }

    /// 典型静息膜电位
    pub fn resting_potential() -> f32 {
        -70.0
    }
}

/// EEG 频段分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EEGBand {
    /// Delta: 0.5 - 4 Hz (深睡眠)
    Delta,
    /// Theta: 4 - 8 Hz (困倦/记忆)
    Theta,
    /// Alpha: 8 - 13 Hz (放松闭眼)
    Alpha,
    /// Beta: 13 - 30 Hz (清醒专注)
    Beta,
    /// Gamma: 30 - 100 Hz (高级认知)
    Gamma,
}

impl EEGBand {
    /// 频率范围 (min, max) Hz
    pub fn range_hz(&self) -> (f32, f32) {
        match self {
            Self::Delta => (0.5, 4.0),
            Self::Theta => (4.0, 8.0),
            Self::Alpha => (8.0, 13.0),
            Self::Beta => (13.0, 30.0),
            Self::Gamma => (30.0, 100.0),
        }
    }

    /// 根据频率归类
    pub fn classify(freq: f32) -> Self {
        if freq < 4.0 {
            Self::Delta
        } else if freq < 8.0 {
            Self::Theta
        } else if freq < 13.0 {
            Self::Alpha
        } else if freq < 30.0 {
            Self::Beta
        } else {
            Self::Gamma
        }
    }
}

/// ECG 信号 (单心跳)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ECGSignal {
    /// P 波幅度 (mV)
    pub p_wave: f32,
    /// QRS 复合波幅度 (mV)
    pub qrs_complex: f32,
    /// T 波幅度 (mV)
    pub t_wave: f32,
    /// 心率 (bpm)
    pub heart_rate: f32,
    /// RR 间隔 (s)
    pub rr_interval: f32,
}

impl ECGSignal {
    /// 根据心率生成 ECG 信号
    /// heart_rate 单位 bpm, dt 用于计算 RR
    pub fn generate_ecg(heart_rate: f32, _dt: f32) -> Self {
        let rr = if heart_rate > 0.0 {
            60.0 / heart_rate
        } else {
            1.0
        };
        Self {
            p_wave: 0.15,
            qrs_complex: 1.2,
            t_wave: 0.3,
            heart_rate,
            rr_interval: rr,
        }
    }

    pub fn new() -> Self {
        Self::generate_ecg(75.0, 0.0)
    }
}

impl Default for ECGSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// EEG 信号 (各频段功率)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EEGSignal {
    pub delta_power: f32,
    pub theta_power: f32,
    pub alpha_power: f32,
    pub beta_power: f32,
    pub gamma_power: f32,
}

impl EEGSignal {
    pub fn new() -> Self {
        Self {
            delta_power: 0.2,
            theta_power: 0.2,
            alpha_power: 0.3,
            beta_power: 0.2,
            gamma_power: 0.1,
        }
    }

    /// 总功率 (用于归一化)
    pub fn total_power(&self) -> f32 {
        self.delta_power
            + self.theta_power
            + self.alpha_power
            + self.beta_power
            + self.gamma_power
    }

    /// 找出功率最大的频段
    pub fn dominant_band(&self) -> EEGBand {
        let mut best = EEGBand::Delta;
        let mut max = self.delta_power;
        if self.theta_power > max {
            best = EEGBand::Theta;
            max = self.theta_power;
        }
        if self.alpha_power > max {
            best = EEGBand::Alpha;
            max = self.alpha_power;
        }
        if self.beta_power > max {
            best = EEGBand::Beta;
            max = self.beta_power;
        }
        if self.gamma_power > max {
            best = EEGBand::Gamma;
        }
        best
    }
}

impl Default for EEGSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具函数: 由心率分类 EEG 频段
pub fn classify_eeg_band(freq: f32) -> EEGBand {
    EEGBand::classify(freq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_potential_duration_default() {
        let d = BioelectricSignal::action_potential_duration();
        assert!(d >= 1.0 && d <= 2.0);
    }

    #[test]
    fn test_resting_potential_value() {
        assert_eq!(BioelectricSignal::resting_potential(), -70.0);
    }

    #[test]
    fn test_signal_new_action_potential() {
        let s = BioelectricSignal::new(WaveformType::ActionPotential);
        assert!(s.amplitude_mv > 0.0);
        assert!(s.duration_ms < 5.0);
    }

    #[test]
    fn test_signal_new_ecg() {
        let s = BioelectricSignal::new(WaveformType::ECG);
        assert!(s.amplitude_mv < 5.0);
        assert!(s.duration_ms > 500.0);
    }

    #[test]
    fn test_eeg_band_ranges() {
        let (lo, hi) = EEGBand::Delta.range_hz();
        assert_eq!(lo, 0.5);
        assert_eq!(hi, 4.0);

        let (lo, hi) = EEGBand::Gamma.range_hz();
        assert_eq!(lo, 30.0);
        assert_eq!(hi, 100.0);
    }

    #[test]
    fn test_classify_delta() {
        assert_eq!(classify_eeg_band(1.0), EEGBand::Delta);
        assert_eq!(EEGBand::classify(2.0), EEGBand::Delta);
    }

    #[test]
    fn test_classify_theta() {
        assert_eq!(EEGBand::classify(5.0), EEGBand::Theta);
        assert_eq!(EEGBand::classify(7.9), EEGBand::Theta);
    }

    #[test]
    fn test_classify_alpha() {
        assert_eq!(EEGBand::classify(10.0), EEGBand::Alpha);
        assert_eq!(EEGBand::classify(12.9), EEGBand::Alpha);
    }

    #[test]
    fn test_classify_beta() {
        assert_eq!(EEGBand::classify(20.0), EEGBand::Beta);
        assert_eq!(EEGBand::classify(29.9), EEGBand::Beta);
    }

    #[test]
    fn test_classify_gamma() {
        assert_eq!(EEGBand::classify(40.0), EEGBand::Gamma);
        assert_eq!(EEGBand::classify(80.0), EEGBand::Gamma);
    }

    #[test]
    fn test_ecg_default_heart_rate() {
        let e = ECGSignal::new();
        assert!(e.heart_rate > 60.0 && e.heart_rate < 100.0);
    }

    #[test]
    fn test_ecg_generate_rr_interval() {
        let e = ECGSignal::generate_ecg(60.0, 0.0);
        assert!((e.rr_interval - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_ecg_generate_high_hr_short_rr() {
        let e = ECGSignal::generate_ecg(120.0, 0.0);
        assert!(e.rr_interval < 0.6);
    }

    #[test]
    fn test_ecg_qrs_largest_amplitude() {
        let e = ECGSignal::new();
        assert!(e.qrs_complex > e.p_wave);
        assert!(e.qrs_complex > e.t_wave);
    }

    #[test]
    fn test_ecg_zero_hr_fallback() {
        let e = ECGSignal::generate_ecg(0.0, 0.0);
        assert_eq!(e.rr_interval, 1.0);
    }

    #[test]
    fn test_eeg_default_powers_positive() {
        let e = EEGSignal::new();
        assert!(e.delta_power >= 0.0);
        assert!(e.alpha_power > 0.0);
    }

    #[test]
    fn test_eeg_total_power() {
        let e = EEGSignal::new();
        let total = e.total_power();
        assert!(total > 0.0);
    }

    #[test]
    fn test_eeg_dominant_band_default() {
        let e = EEGSignal::new();
        // alpha_power = 0.3 最大
        assert_eq!(e.dominant_band(), EEGBand::Alpha);
    }

    #[test]
    fn test_eeg_dominant_band_when_gamma_high() {
        let e = EEGSignal {
            delta_power: 0.1,
            theta_power: 0.1,
            alpha_power: 0.1,
            beta_power: 0.1,
            gamma_power: 0.5,
        };
        assert_eq!(e.dominant_band(), EEGBand::Gamma);
    }

    #[test]
    fn test_ecg_serialization_roundtrip() {
        let e = ECGSignal::new();
        let json = serde_json::to_string(&e).expect("serialize");
        let back: ECGSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }

    #[test]
    fn test_eeg_serialization_roundtrip() {
        let e = EEGSignal::new();
        let json = serde_json::to_string(&e).expect("serialize");
        let back: EEGSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }

    #[test]
    fn test_signal_serialization_roundtrip() {
        let s = BioelectricSignal::new(WaveformType::EEG);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: BioelectricSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }

    #[test]
    fn test_waveform_type_variants() {
        let _ = WaveformType::ActionPotential;
        let _ = WaveformType::ECG;
        let _ = WaveformType::EEG;
        let _ = WaveformType::EMG;
        let _ = WaveformType::EOG;
    }

    #[test]
    fn test_alpha_band_boundaries() {
        // 边界值 13.0 应归 Beta
        assert_eq!(EEGBand::classify(13.0), EEGBand::Beta);
        // 12.9 归 Alpha
        assert_eq!(EEGBand::classify(12.9), EEGBand::Alpha);
    }
}
