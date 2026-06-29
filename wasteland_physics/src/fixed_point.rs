use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

const FRACTIONAL_BITS: u32 = 32;
const SCALE: i128 = 1 << FRACTIONAL_BITS;
const SCALE_F32: f32 = SCALE as f32;
const SCALE_F64: f64 = SCALE as f64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FixedPoint {
    pub raw: i64,
}

impl FixedPoint {
    pub const ZERO: Self = Self { raw: 0 };
    pub const ONE: Self = Self { raw: SCALE as i64 };
    pub const NEG_ONE: Self = Self { raw: -(SCALE as i64) };
    pub const MIN: Self = Self { raw: i64::MIN };
    pub const MAX: Self = Self { raw: i64::MAX };
    pub const EPSILON: Self = Self { raw: 1 };

    pub fn from_f32(value: f32) -> Self {
        let raw = (value as f64 * SCALE_F64) as i64;
        Self { raw }
    }

    pub fn from_f64(value: f64) -> Self {
        let raw = (value * SCALE_F64) as i64;
        Self { raw }
    }

    pub fn from_i32(value: i32) -> Self {
        Self { raw: (value as i64) << FRACTIONAL_BITS }
    }

    pub fn to_f32(self) -> f32 {
        self.raw as f32 / SCALE_F32
    }

    pub fn to_f64(self) -> f64 {
        self.raw as f64 / SCALE_F64
    }

    pub fn abs(self) -> Self {
        Self { raw: self.raw.saturating_abs() }
    }

    pub fn max(self, other: Self) -> Self {
        Self { raw: self.raw.max(other.raw) }
    }

    pub fn min(self, other: Self) -> Self {
        Self { raw: self.raw.min(other.raw) }
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self { raw: self.raw.clamp(min.raw, max.raw) }
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.raw.checked_add(other.raw).map(|raw| Self { raw })
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.raw.checked_sub(other.raw).map(|raw| Self { raw })
    }

    pub fn checked_mul(self, other: Self) -> Option<Self> {
        let result = (self.raw as i128).checked_mul(other.raw as i128)?.checked_div(SCALE)?;
        Some(Self { raw: result as i64 })
    }

    pub fn checked_div(self, other: Self) -> Option<Self> {
        if other.raw == 0 {
            return None;
        }
        let result = (self.raw as i128).checked_mul(SCALE)?.checked_div(other.raw as i128)?;
        Some(Self { raw: result as i64 })
    }

    pub fn sqrt(self) -> Self {
        if self.raw <= 0 {
            return Self::ZERO;
        }
        let radicand = (self.raw as i128) * SCALE;
        let root = integer_sqrt_i128(radicand);
        Self { raw: root as i64 }
    }

    pub fn powi(self, n: i32) -> Self {
        if n == 0 {
            return Self::ONE;
        }
        if n < 0 {
            return Self::ONE / self.powi(-n);
        }
        let mut result = Self::ONE;
        let mut base = self;
        let mut exp = n;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.saturating_mul(base);
            }
            base = base.saturating_mul(base);
            exp >>= 1;
        }
        result
    }

    pub fn sin(self) -> Self {
        let pi = FixedPoint::from_raw(13493037704);
        let two_pi = pi * FixedPoint::from_i32(2);
        let half_pi = pi / FixedPoint::from_i32(2);
        let mut angle = self;
        angle.raw = angle.raw.rem_euclid(two_pi.raw);
        let mut negate = false;
        if angle.raw > pi.raw {
            angle.raw = two_pi.raw - angle.raw;
            negate = true;
        }
        if angle.raw > half_pi.raw {
            angle.raw = pi.raw - angle.raw;
        }
        let idx = (angle.raw >> (FRACTIONAL_BITS - SIN_TABLE_BITS)) as usize;
        let idx = idx.min(SIN_TABLE_SIZE - 2);
        let frac = angle.raw & ((1i64 << (FRACTIONAL_BITS - SIN_TABLE_BITS)) - 1);
        let denom = 1i64 << (FRACTIONAL_BITS - SIN_TABLE_BITS);
        let base = SIN_TABLE[idx];
        let next = SIN_TABLE[idx + 1];
        let interp = base + (next - base).wrapping_mul(frac) / denom;
        Self { raw: if negate { -interp } else { interp } }
    }

    pub fn cos(self) -> Self {
        let pi = FixedPoint::from_raw(13493037704);
        let half_pi = pi / FixedPoint::from_i32(2);
        (self + half_pi).sin()
    }

    pub fn tan(self) -> Self {
        let c = self.cos();
        if c.raw == 0 {
            return if self.sin().raw >= 0 { Self::MAX } else { Self::MIN };
        }
        self.sin() / c
    }

    pub fn atan2(y: Self, x: Self) -> Self {
        if x.raw == 0 && y.raw == 0 {
            return Self::ZERO;
        }
        let abs_y = y.abs();
        let abs_x = x.abs();
        let angle = if abs_x.raw >= abs_y.raw {
            let ratio = abs_y / abs_x;
            Self::atan_small(ratio)
        } else {
            let pi = FixedPoint::from_raw(13493037704);
            let half_pi = pi / FixedPoint::from_i32(2);
            half_pi - Self::atan_small(abs_x / abs_y)
        };
        if x.raw < 0 {
            let pi = FixedPoint::from_raw(13493037704);
            if y.raw >= 0 { pi - angle } else { -(pi - angle) }
        } else if y.raw >= 0 {
            angle
        } else {
            -angle
        }
    }

    fn atan_small(x: Self) -> Self {
        let x2 = x * x;
        let x3 = x2 * x;
        let x5 = x3 * x2;
        let x7 = x5 * x2;
        let x9 = x7 * x2;
        x - x3 / FixedPoint::from_i32(3) + x5 / FixedPoint::from_i32(5)
            - x7 / FixedPoint::from_i32(7)
            + x9 / FixedPoint::from_i32(9)
    }

    pub const fn from_raw(raw: i64) -> Self {
        Self { raw }
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self { raw: self.raw.saturating_add(other.raw) }
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self { raw: self.raw.saturating_sub(other.raw) }
    }

    pub fn saturating_mul(self, other: Self) -> Self {
        self.checked_mul(other).unwrap_or({
            if (self.raw > 0 && other.raw > 0) || (self.raw < 0 && other.raw < 0) {
                Self::MAX
            } else {
                Self::MIN
            }
        })
    }

    pub fn saturating_div(self, other: Self) -> Self {
        if other.raw == 0 {
            if self.raw == 0 {
                Self::ZERO
            } else if self.raw > 0 {
                Self::MAX
            } else {
                Self::MIN
            }
        } else {
            self.checked_div(other).unwrap_or({
                if (self.raw > 0 && other.raw > 0) || (self.raw < 0 && other.raw < 0) {
                    Self::MAX
                } else {
                    Self::MIN
                }
            })
        }
    }

    pub fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self).saturating_mul(t)
    }

    pub fn signum(self) -> Self {
        if self.raw > 0 {
            Self::ONE
        } else if self.raw < 0 {
            Self::NEG_ONE
        } else {
            Self::ZERO
        }
    }

    pub fn floor(self) -> Self {
        let mask = (1i64 << FRACTIONAL_BITS) - 1;
        Self { raw: self.raw & !mask }
    }

    pub fn fract(self) -> Self {
        let mask = (1i64 << FRACTIONAL_BITS) - 1;
        let frac_bits = self.raw & mask;
        if frac_bits < 0 {
            Self { raw: frac_bits + (1i64 << FRACTIONAL_BITS) }
        } else {
            Self { raw: frac_bits }
        }
    }

    pub fn ln(self) -> Self {
        if self.raw <= 0 {
            return Self::MIN;
        }
        let one = Self::ONE;
        let t = (self - one) / (self + one);
        let t2 = t * t;
        let t3 = t2 * t;
        let t5 = t3 * t2;
        let t7 = t5 * t2;
        let t9 = t7 * t2;
        let two = Self::from_i32(2);
        let three = Self::from_i32(3);
        let five = Self::from_i32(5);
        let seven = Self::from_i32(7);
        let nine = Self::from_i32(9);
        two * (t + t3 / three + t5 / five + t7 / seven + t9 / nine)
    }

    pub fn exp(self) -> Self {
        if self.raw == 0 {
            return Self::ONE;
        }
        let mut result = Self::ONE + self;
        let mut term = self;
        let mut n = Self::from_i32(2);
        for _ in 2..12 {
            term = term * self / n;
            if term.abs().raw < Self::EPSILON.raw {
                break;
            }
            result += term;
            n += Self::ONE;
        }
        result
    }
}

impl Add for FixedPoint {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.saturating_add(rhs)
    }
}

impl AddAssign for FixedPoint {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedPoint {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for FixedPoint {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for FixedPoint {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.saturating_mul(rhs)
    }
}

impl MulAssign for FixedPoint {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for FixedPoint {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        if rhs.raw == 0 {
            // 处理除数为零的情况，返回符号无穷大或零
            if self.raw == 0 {
                Self::ZERO
            } else if self.raw > 0 {
                Self::MAX
            } else {
                Self::MIN
            }
        } else {
            self.checked_div(rhs)
                .unwrap_or_else(|| panic!("FixedPoint div overflow: {:?} / {:?}", self, rhs))
        }
    }
}

impl DivAssign for FixedPoint {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Neg for FixedPoint {
    type Output = Self;
    fn neg(self) -> Self {
        Self { raw: self.raw.saturating_neg() }
    }
}

impl Default for FixedPoint {
    fn default() -> Self {
        Self::ZERO
    }
}

impl std::fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_f32())
    }
}

use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FixedVec3 {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub z: FixedPoint,
}

impl FixedVec3 {
    pub const ZERO: Self = Self { x: FixedPoint::ZERO, y: FixedPoint::ZERO, z: FixedPoint::ZERO };

    pub fn new(x: FixedPoint, y: FixedPoint, z: FixedPoint) -> Self {
        Self { x, y, z }
    }

    pub fn from_f32(x: f32, y: f32, z: f32) -> Self {
        Self { x: FixedPoint::from_f32(x), y: FixedPoint::from_f32(y), z: FixedPoint::from_f32(z) }
    }

    pub fn from_glam(v: Vec3) -> Self {
        Self::from_f32(v.x, v.y, v.z)
    }

    pub fn to_glam(self) -> Vec3 {
        Vec3::new(self.x.to_f32(), self.y.to_f32(), self.z.to_f32())
    }

    pub fn length(self) -> FixedPoint {
        let sum = self.x * self.x + self.y * self.y + self.z * self.z;
        sum.sqrt()
    }

    pub fn length_squared(self) -> FixedPoint {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len.raw == 0 {
            return Self::ZERO;
        }
        Self { x: self.x / len, y: self.y / len, z: self.z / len }
    }

    pub fn dot(self, other: Self) -> FixedPoint {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn distance(self, other: Self) -> FixedPoint {
        (self - other).length()
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
            z: self.z.clamp(min.z, max.z),
        }
    }

    pub fn lerp(self, other: Self, t: FixedPoint) -> Self {
        Self { x: self.x.lerp(other.x, t), y: self.y.lerp(other.y, t), z: self.z.lerp(other.z, t) }
    }

    pub fn saturating_mul(self, scalar: FixedPoint) -> Self {
        Self { x: self.x.saturating_mul(scalar), y: self.y.saturating_mul(scalar), z: self.z.saturating_mul(scalar) }
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self { x: self.x.saturating_add(other.x), y: self.y.saturating_add(other.y), z: self.z.saturating_add(other.z) }
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self { x: self.x.saturating_sub(other.x), y: self.y.saturating_sub(other.y), z: self.z.saturating_sub(other.z) }
    }

    pub fn abs(self) -> Self {
        Self { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }

    pub fn max_component(self) -> FixedPoint {
        self.x.max(self.y.max(self.z))
    }

    pub fn min_component(self) -> FixedPoint {
        self.x.min(self.y.min(self.z))
    }

    pub fn floor(self) -> Self {
        Self { x: self.x.floor(), y: self.y.floor(), z: self.z.floor() }
    }

    pub fn fract(self) -> Self {
        Self { x: self.x.fract(), y: self.y.fract(), z: self.z.fract() }
    }
}

impl Add for FixedVec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl AddAssign for FixedVec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for FixedVec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl SubAssign for FixedVec3 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<FixedPoint> for FixedVec3 {
    type Output = Self;
    fn mul(self, rhs: FixedPoint) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl MulAssign<FixedPoint> for FixedVec3 {
    fn mul_assign(&mut self, rhs: FixedPoint) {
        *self = *self * rhs;
    }
}

impl Div<FixedPoint> for FixedVec3 {
    type Output = Self;
    fn div(self, rhs: FixedPoint) -> Self {
        Self { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}

impl DivAssign<FixedPoint> for FixedVec3 {
    fn div_assign(&mut self, rhs: FixedPoint) {
        *self = *self / rhs;
    }
}

impl Mul<FixedVec3> for FixedPoint {
    type Output = FixedVec3;
    fn mul(self, rhs: FixedVec3) -> FixedVec3 {
        rhs * self
    }
}

impl Neg for FixedVec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y, z: -self.z }
    }
}

impl Default for FixedVec3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl std::fmt::Display for FixedVec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FixedQuat {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub z: FixedPoint,
    pub w: FixedPoint,
}

impl FixedQuat {
    pub const IDENTITY: Self =
        Self { x: FixedPoint::ZERO, y: FixedPoint::ZERO, z: FixedPoint::ZERO, w: FixedPoint::ONE };

    pub fn from_glam(q: glam::Quat) -> Self {
        Self {
            x: FixedPoint::from_f32(q.x),
            y: FixedPoint::from_f32(q.y),
            z: FixedPoint::from_f32(q.z),
            w: FixedPoint::from_f32(q.w),
        }
    }

    pub fn to_glam(self) -> glam::Quat {
        glam::Quat::from_xyzw(self.x.to_f32(), self.y.to_f32(), self.z.to_f32(), self.w.to_f32())
    }

    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len.raw == 0 {
            return Self::IDENTITY;
        }
        Self { x: self.x / len, y: self.y / len, z: self.z / len, w: self.w / len }
    }

    pub fn rotate_vec3(self, v: FixedVec3) -> FixedVec3 {
        let qv = FixedVec3::new(self.x, self.y, self.z);
        let uv = qv.cross(v);
        let uuv = qv.cross(uv);
        v + uv * (self.w + self.w) + uuv * (FixedPoint::ONE + FixedPoint::ONE)
    }

    pub fn slerp(self, other: FixedQuat, t: f32) -> FixedQuat {
        let dot =
            (self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w).to_f32();
        let dot = dot.clamp(-1.0, 1.0);
        let theta = dot.acos();
        let sin_theta = theta.sin();
        if sin_theta.abs() < 0.001 {
            let a = 1.0 - t;
            let b = t;
            return FixedQuat {
                x: FixedPoint::from_f32(self.x.to_f32() * a + other.x.to_f32() * b),
                y: FixedPoint::from_f32(self.y.to_f32() * a + other.y.to_f32() * b),
                z: FixedPoint::from_f32(self.z.to_f32() * a + other.z.to_f32() * b),
                w: FixedPoint::from_f32(self.w.to_f32() * a + other.w.to_f32() * b),
            }
            .normalize();
        }
        let a = ((1.0 - t) * theta).sin() / sin_theta;
        let b = (t * theta).sin() / sin_theta;
        FixedQuat {
            x: FixedPoint::from_f32(self.x.to_f32() * a + other.x.to_f32() * b),
            y: FixedPoint::from_f32(self.y.to_f32() * a + other.y.to_f32() * b),
            z: FixedPoint::from_f32(self.z.to_f32() * a + other.z.to_f32() * b),
            w: FixedPoint::from_f32(self.w.to_f32() * a + other.w.to_f32() * b),
        }
        .normalize()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FixedMat3 {
    pub x_axis: FixedVec3,
    pub y_axis: FixedVec3,
    pub z_axis: FixedVec3,
}

impl FixedMat3 {
    pub const ZERO: Self =
        Self { x_axis: FixedVec3::ZERO, y_axis: FixedVec3::ZERO, z_axis: FixedVec3::ZERO };

    pub const IDENTITY: Self = Self {
        x_axis: FixedVec3 { x: FixedPoint::ONE, y: FixedPoint::ZERO, z: FixedPoint::ZERO },
        y_axis: FixedVec3 { x: FixedPoint::ZERO, y: FixedPoint::ONE, z: FixedPoint::ZERO },
        z_axis: FixedVec3 { x: FixedPoint::ZERO, y: FixedPoint::ZERO, z: FixedPoint::ONE },
    };

    pub fn from_cols(x_axis: FixedVec3, y_axis: FixedVec3, z_axis: FixedVec3) -> Self {
        Self { x_axis, y_axis, z_axis }
    }

    pub fn from_glam(m: glam::Mat3) -> Self {
        Self {
            x_axis: FixedVec3::from_glam(m.x_axis),
            y_axis: FixedVec3::from_glam(m.y_axis),
            z_axis: FixedVec3::from_glam(m.z_axis),
        }
    }

    pub fn to_glam(self) -> glam::Mat3 {
        glam::Mat3::from_cols(self.x_axis.to_glam(), self.y_axis.to_glam(), self.z_axis.to_glam())
    }

    pub fn col(&self, index: usize) -> &FixedVec3 {
        match index {
            0 => &self.x_axis,
            1 => &self.y_axis,
            2 => &self.z_axis,
            _ => &self.z_axis,
        }
    }

    pub fn col_mut(&mut self, index: usize) -> &mut FixedVec3 {
        match index {
            0 => &mut self.x_axis,
            1 => &mut self.y_axis,
            2 => &mut self.z_axis,
            _ => &mut self.z_axis,
        }
    }

    pub fn transpose(&self) -> Self {
        Self {
            x_axis: FixedVec3::new(self.x_axis.x, self.y_axis.x, self.z_axis.x),
            y_axis: FixedVec3::new(self.x_axis.y, self.y_axis.y, self.z_axis.y),
            z_axis: FixedVec3::new(self.x_axis.z, self.y_axis.z, self.z_axis.z),
        }
    }

    pub fn saturating_mul(self, scalar: FixedPoint) -> Self {
        Self {
            x_axis: self.x_axis.saturating_mul(scalar),
            y_axis: self.y_axis.saturating_mul(scalar),
            z_axis: self.z_axis.saturating_mul(scalar),
        }
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self {
            x_axis: self.x_axis.saturating_add(other.x_axis),
            y_axis: self.y_axis.saturating_add(other.y_axis),
            z_axis: self.z_axis.saturating_add(other.z_axis),
        }
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self {
            x_axis: self.x_axis.saturating_sub(other.x_axis),
            y_axis: self.y_axis.saturating_sub(other.y_axis),
            z_axis: self.z_axis.saturating_sub(other.z_axis),
        }
    }

    pub fn determinant(&self) -> FixedPoint {
        let a = self.x_axis.x.raw as i128;
        let b = self.x_axis.y.raw as i128;
        let c = self.x_axis.z.raw as i128;
        let d = self.y_axis.x.raw as i128;
        let e = self.y_axis.y.raw as i128;
        let f = self.y_axis.z.raw as i128;
        let g = self.z_axis.x.raw as i128;
        let h = self.z_axis.y.raw as i128;
        let i = self.z_axis.z.raw as i128;
        let result = a.checked_mul(e.checked_mul(i).unwrap_or(0).checked_sub(f.checked_mul(h).unwrap_or(0)).unwrap_or(0))
            .unwrap_or(0)
            .checked_sub(b.checked_mul(d.checked_mul(i).unwrap_or(0).checked_sub(f.checked_mul(g).unwrap_or(0)).unwrap_or(0)).unwrap_or(0))
            .unwrap_or(0)
            .checked_add(c.checked_mul(d.checked_mul(h).unwrap_or(0).checked_sub(e.checked_mul(g).unwrap_or(0)).unwrap_or(0)).unwrap_or(0))
            .unwrap_or(0);
        FixedPoint { raw: (result / SCALE / SCALE) as i64 }
    }

    pub fn inverse(&self) -> Self {
        let det = self.determinant();
        if det.raw == 0 {
            return Self::ZERO;
        }
        let inv_det = FixedPoint::ONE / det;
        let a = self.x_axis.x.raw as i128;
        let b = self.x_axis.y.raw as i128;
        let c = self.x_axis.z.raw as i128;
        let d = self.y_axis.x.raw as i128;
        let e = self.y_axis.y.raw as i128;
        let f = self.y_axis.z.raw as i128;
        let g = self.z_axis.x.raw as i128;
        let h = self.z_axis.y.raw as i128;
        let i = self.z_axis.z.raw as i128;
        Self {
            x_axis: FixedVec3::new(
                FixedPoint::from_raw(((e * i - f * h) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((c * h - b * i) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((b * f - c * e) / SCALE) as i64) * inv_det,
            ),
            y_axis: FixedVec3::new(
                FixedPoint::from_raw(((f * g - d * i) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((a * i - c * g) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((c * d - a * f) / SCALE) as i64) * inv_det,
            ),
            z_axis: FixedVec3::new(
                FixedPoint::from_raw(((d * h - e * g) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((b * g - a * h) / SCALE) as i64) * inv_det,
                FixedPoint::from_raw(((a * e - b * d) / SCALE) as i64) * inv_det,
            ),
        }
    }
}

impl Add for FixedMat3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x_axis: self.x_axis + rhs.x_axis,
            y_axis: self.y_axis + rhs.y_axis,
            z_axis: self.z_axis + rhs.z_axis,
        }
    }
}

impl Sub for FixedMat3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x_axis: self.x_axis - rhs.x_axis,
            y_axis: self.y_axis - rhs.y_axis,
            z_axis: self.z_axis - rhs.z_axis,
        }
    }
}

impl Mul<FixedPoint> for FixedMat3 {
    type Output = Self;
    fn mul(self, rhs: FixedPoint) -> Self {
        Self { x_axis: self.x_axis * rhs, y_axis: self.y_axis * rhs, z_axis: self.z_axis * rhs }
    }
}

impl Mul<FixedMat3> for FixedPoint {
    type Output = FixedMat3;
    fn mul(self, rhs: FixedMat3) -> FixedMat3 {
        rhs * self
    }
}

impl Div<FixedPoint> for FixedMat3 {
    type Output = Self;
    fn div(self, rhs: FixedPoint) -> Self {
        Self { x_axis: self.x_axis / rhs, y_axis: self.y_axis / rhs, z_axis: self.z_axis / rhs }
    }
}

impl Mul<FixedMat3> for FixedMat3 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let a = self.transpose();
        Self {
            x_axis: FixedVec3::new(
                a.x_axis.dot(rhs.x_axis),
                a.y_axis.dot(rhs.x_axis),
                a.z_axis.dot(rhs.x_axis),
            ),
            y_axis: FixedVec3::new(
                a.x_axis.dot(rhs.y_axis),
                a.y_axis.dot(rhs.y_axis),
                a.z_axis.dot(rhs.y_axis),
            ),
            z_axis: FixedVec3::new(
                a.x_axis.dot(rhs.z_axis),
                a.y_axis.dot(rhs.z_axis),
                a.z_axis.dot(rhs.z_axis),
            ),
        }
    }
}

impl Mul<FixedVec3> for FixedMat3 {
    type Output = FixedVec3;
    fn mul(self, rhs: FixedVec3) -> FixedVec3 {
        FixedVec3::new(
            self.x_axis.x * rhs.x + self.y_axis.x * rhs.y + self.z_axis.x * rhs.z,
            self.x_axis.y * rhs.x + self.y_axis.y * rhs.y + self.z_axis.y * rhs.z,
            self.x_axis.z * rhs.x + self.y_axis.z * rhs.y + self.z_axis.z * rhs.z,
        )
    }
}

impl Neg for FixedMat3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self { x_axis: -self.x_axis, y_axis: -self.y_axis, z_axis: -self.z_axis }
    }
}

impl Default for FixedMat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

fn integer_sqrt_i128(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) >> 1;
    while y < x {
        x = y;
        y = (x + n / x) >> 1;
    }
    x
}

const SIN_TABLE_BITS: u32 = 10;
const SIN_TABLE_SIZE: usize = 1 << SIN_TABLE_BITS;

use std::sync::LazyLock;

static SIN_TABLE: LazyLock<[i64; SIN_TABLE_SIZE]> = LazyLock::new(|| {
    let mut table = [0i64; SIN_TABLE_SIZE];
    let pi_over_2 = std::f64::consts::FRAC_PI_2;
    let scale = (1u64 << FRACTIONAL_BITS) as f64;
    for (i, item) in table.iter_mut().enumerate().take(SIN_TABLE_SIZE) {
        let angle = (i as f64) / (SIN_TABLE_SIZE as f64) * pi_over_2;
        *item = (angle.sin() * scale) as i64;
    }
    table
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_trig() {
        let pi = FixedPoint::from_raw(13493037704);
        let half_pi = pi / FixedPoint::from_i32(2);
        let zero = FixedPoint::ZERO;

        let sin0 = zero.sin();
        assert!((sin0.to_f32()).abs() < 0.001);

        let sin_half_pi = half_pi.sin();
        assert!((sin_half_pi.to_f32() - 1.0).abs() < 0.001);

        let cos0 = zero.cos();
        assert!((cos0.to_f32() - 1.0).abs() < 0.001);

        let cos_half_pi = half_pi.cos();
        assert!((cos_half_pi.to_f32()).abs() < 0.001);
    }

    #[test]
    fn test_fixed_sqrt() {
        let four = FixedPoint::from_i32(4);
        let sqrt4 = four.sqrt();
        assert!((sqrt4.to_f32() - 2.0).abs() < 0.001);

        let two = FixedPoint::from_i32(2);
        let sqrt2 = two.sqrt();
        assert!((sqrt2.to_f32() - 1.414).abs() < 0.01);
    }

    #[test]
    fn test_fixed_powi() {
        let two = FixedPoint::from_i32(2);
        let pow3 = two.powi(3);
        assert!((pow3.to_f32() - 8.0).abs() < 0.001);

        let four = FixedPoint::from_i32(4);
        let pow_neg1 = four.powi(-1);
        assert!((pow_neg1.to_f32() - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_basic_arithmetic() {
        let a = FixedPoint::from_f32(3.5);
        let b = FixedPoint::from_f32(2.0);
        assert!((a + b).to_f32() - 5.5 < 0.001);
        assert!((a - b).to_f32() - 1.5 < 0.001);
        assert!((a * b).to_f32() - 7.0 < 0.001);
        assert!((a / b).to_f32() - 1.75 < 0.001);
    }

    #[test]
    fn test_vec3_operations() {
        let v = FixedVec3::from_f32(1.0, 0.0, 0.0);
        let len = v.length();
        assert!((len.to_f32() - 1.0).abs() < 0.001);

        let n = v.normalize();
        assert!((n.x.to_f32() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_determinism() {
        let a = FixedPoint::from_f32(1.0 / 3.0);
        let b = a * FixedPoint::from_i32(3);
        assert!((b.to_f32() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_overflow_detection() {
        let a = FixedPoint::MAX;
        let b = FixedPoint::ONE;
        assert!(a.checked_add(b).is_none());
    }
}
