use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoaVec3 {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
}

impl SoaVec3 {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            x: Vec::with_capacity(capacity),
            y: Vec::with_capacity(capacity),
            z: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.x.len()
    }

    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    pub fn push(&mut self, x: f32, y: f32, z: f32) {
        self.x.push(x);
        self.y.push(y);
        self.z.push(z);
    }

    pub fn clear(&mut self) {
        self.x.clear();
        self.y.clear();
        self.z.clear();
    }

    pub fn get(&self, index: usize) -> (f32, f32, f32) {
        (self.x[index], self.y[index], self.z[index])
    }

    pub fn set(&mut self, index: usize, x: f32, y: f32, z: f32) {
        self.x[index] = x;
        self.y[index] = y;
        self.z[index] = z;
    }

    pub fn reserve(&mut self, additional: usize) {
        self.x.reserve(additional);
        self.y.reserve(additional);
        self.z.reserve(additional);
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn add_batch_avx2(&mut self, indices: &[usize], dx: f32, dy: f32, dz: f32) {
        use std::arch::x86_64::*;
        let vdx = _mm256_set1_ps(dx);
        let vdy = _mm256_set1_ps(dy);
        let vdz = _mm256_set1_ps(dz);

        for chunk in indices.chunks(8) {
            let mut x_vals = [0.0f32; 8];
            let mut y_vals = [0.0f32; 8];
            let mut z_vals = [0.0f32; 8];
            for (i, &idx) in chunk.iter().enumerate() {
                x_vals[i] = self.x[idx];
                y_vals[i] = self.y[idx];
                z_vals[i] = self.z[idx];
            }
            let vx = _mm256_loadu_ps(x_vals.as_ptr());
            let vy = _mm256_loadu_ps(y_vals.as_ptr());
            let vz = _mm256_loadu_ps(z_vals.as_ptr());

            let rx = _mm256_add_ps(vx, vdx);
            let ry = _mm256_add_ps(vy, vdy);
            let rz = _mm256_add_ps(vz, vdz);

            _mm256_storeu_ps(x_vals.as_mut_ptr(), rx);
            _mm256_storeu_ps(y_vals.as_mut_ptr(), ry);
            _mm256_storeu_ps(z_vals.as_mut_ptr(), rz);

            for (i, &idx) in chunk.iter().enumerate() {
                self.x[idx] = x_vals[i];
                self.y[idx] = y_vals[i];
                self.z[idx] = z_vals[i];
            }
        }
    }

    pub fn add_batch_scalar(&mut self, indices: &[usize], dx: f32, dy: f32, dz: f32) {
        for &idx in indices {
            self.x[idx] += dx;
            self.y[idx] += dy;
            self.z[idx] += dz;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoaScalar {
    pub values: Vec<f32>,
}

impl SoaScalar {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { values: Vec::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn push(&mut self, value: f32) {
        self.values.push(value);
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn scale_batch_avx2(&mut self, indices: &[usize], factor: f32) {
        use std::arch::x86_64::*;
        let vfactor = _mm256_set1_ps(factor);

        for chunk in indices.chunks(8) {
            let mut vals = [0.0f32; 8];
            for (i, &idx) in chunk.iter().enumerate() {
                vals[i] = self.values[idx];
            }
            let v = _mm256_loadu_ps(vals.as_ptr());
            let r = _mm256_mul_ps(v, vfactor);
            _mm256_storeu_ps(vals.as_mut_ptr(), r);
            for (i, &idx) in chunk.iter().enumerate() {
                self.values[idx] = vals[i];
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoaMat3 {
    pub m00: Vec<f32>,
    pub m01: Vec<f32>,
    pub m02: Vec<f32>,
    pub m10: Vec<f32>,
    pub m11: Vec<f32>,
    pub m12: Vec<f32>,
    pub m20: Vec<f32>,
    pub m21: Vec<f32>,
    pub m22: Vec<f32>,
}

impl SoaMat3 {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            m00: Vec::with_capacity(capacity),
            m01: Vec::with_capacity(capacity),
            m02: Vec::with_capacity(capacity),
            m10: Vec::with_capacity(capacity),
            m11: Vec::with_capacity(capacity),
            m12: Vec::with_capacity(capacity),
            m20: Vec::with_capacity(capacity),
            m21: Vec::with_capacity(capacity),
            m22: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.m00.len()
    }

    pub fn is_empty(&self) -> bool {
        self.m00.is_empty()
    }

    pub fn clear(&mut self) {
        self.m00.clear();
        self.m01.clear();
        self.m02.clear();
        self.m10.clear();
        self.m11.clear();
        self.m12.clear();
        self.m20.clear();
        self.m21.clear();
        self.m22.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSoA {
    pub positions: SoaVec3,
    pub velocities: SoaVec3,
    pub masses: SoaScalar,
    pub densities: SoaScalar,
    pub pressures: SoaScalar,
    pub temperatures: SoaScalar,
    pub active: Vec<bool>,
    pub count: usize,
}

impl ParticleSoA {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions: SoaVec3::with_capacity(capacity),
            velocities: SoaVec3::with_capacity(capacity),
            masses: SoaScalar::with_capacity(capacity),
            densities: SoaScalar::with_capacity(capacity),
            pressures: SoaScalar::with_capacity(capacity),
            temperatures: SoaScalar::with_capacity(capacity),
            active: Vec::with_capacity(capacity),
            count: 0,
        }
    }

    pub fn push(&mut self, pos: (f32, f32, f32), vel: (f32, f32, f32), mass: f32) {
        self.positions.push(pos.0, pos.1, pos.2);
        self.velocities.push(vel.0, vel.1, vel.2);
        self.masses.push(mass);
        self.densities.push(0.0);
        self.pressures.push(0.0);
        self.temperatures.push(293.0);
        self.active.push(true);
        self.count += 1;
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.count {
            let last = self.count - 1;
            if index != last {
                self.positions.set(
                    index,
                    self.positions.x[last],
                    self.positions.y[last],
                    self.positions.z[last],
                );
                self.velocities.set(
                    index,
                    self.velocities.x[last],
                    self.velocities.y[last],
                    self.velocities.z[last],
                );
                self.masses.values[index] = self.masses.values[last];
                self.densities.values[index] = self.densities.values[last];
                self.pressures.values[index] = self.pressures.values[last];
                self.temperatures.values[index] = self.temperatures.values[last];
                self.active[index] = self.active[last];
            }
            self.positions.x.truncate(last);
            self.positions.y.truncate(last);
            self.positions.z.truncate(last);
            self.velocities.x.truncate(last);
            self.velocities.y.truncate(last);
            self.velocities.z.truncate(last);
            self.masses.values.truncate(last);
            self.densities.values.truncate(last);
            self.pressures.values.truncate(last);
            self.temperatures.values.truncate(last);
            self.active.truncate(last);
            self.count -= 1;
        }
    }
}

#[cfg(test)]
#[cfg(target_arch = "x86_64")]
mod tests {
    use super::*;

    #[test]
    fn test_soa_vec3_push_get() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(1.0, 2.0, 3.0);
        v.push(4.0, 5.0, 6.0);
        assert_eq!(v.len(), 2);
        assert_eq!(v.get(0), (1.0, 2.0, 3.0));
        assert_eq!(v.get(1), (4.0, 5.0, 6.0));
    }

    #[test]
    fn test_soa_vec3_clear() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(1.0, 2.0, 3.0);
        v.clear();
        assert!(v.is_empty());
    }

    #[test]
    fn test_particle_soa_push_remove() {
        let mut p = ParticleSoA::with_capacity(8);
        p.push((1.0, 2.0, 3.0), (0.1, 0.2, 0.3), 1.0);
        p.push((4.0, 5.0, 6.0), (0.4, 0.5, 0.6), 2.0);
        assert_eq!(p.count, 2);
        p.remove(0);
        assert_eq!(p.count, 1);
    }

    #[test]
    fn test_soa_scalar() {
        let mut s = SoaScalar::with_capacity(4);
        s.push(1.0);
        s.push(2.0);
        assert_eq!(s.len(), 2);
        assert_eq!(s.values[0], 1.0);
    }

    #[test]
    fn test_soa_vec3_add() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(1.0, 2.0, 3.0);
        v.push(4.0, 5.0, 6.0);
        let (ax, ay, az) = v.get(0);
        let (bx, by, bz) = v.get(1);
        let (rx, ry, rz) = (ax + bx, ay + by, az + bz);
        assert!((rx - 5.0).abs() < 0.001);
        assert!((ry - 7.0).abs() < 0.001);
        assert!((rz - 9.0).abs() < 0.001);
    }

    #[test]
    fn test_soa_vec3_sub() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(5.0, 8.0, 10.0);
        v.push(2.0, 3.0, 4.0);
        let (ax, ay, az) = v.get(0);
        let (bx, by, bz) = v.get(1);
        let (rx, ry, rz) = (ax - bx, ay - by, az - bz);
        assert!((rx - 3.0).abs() < 0.001);
        assert!((ry - 5.0).abs() < 0.001);
        assert!((rz - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_soa_vec3_dot() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(1.0, 2.0, 3.0);
        v.push(4.0, 5.0, 6.0);
        let (ax, ay, az) = v.get(0);
        let (bx, by, bz) = v.get(1);
        let dot = ax * bx + ay * by + az * bz;
        assert!((dot - 32.0).abs() < 0.001);
    }

    #[test]
    fn test_soa_vec3_length_squared() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(3.0, 4.0, 0.0);
        let (x, y, z) = v.get(0);
        let len_sq = x * x + y * y + z * z;
        assert!((len_sq - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_soa_vec3_set() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(0.0, 0.0, 0.0);
        v.set(0, 7.0, 8.0, 9.0);
        assert_eq!(v.get(0), (7.0, 8.0, 9.0));
    }

    #[test]
    fn test_soa_vec3_reserve() {
        let mut v = SoaVec3::with_capacity(0);
        v.reserve(100);
        assert!(v.x.capacity() >= 100);
        assert!(v.y.capacity() >= 100);
        assert!(v.z.capacity() >= 100);
    }

    #[test]
    fn test_soa_vec3_add_batch_scalar() {
        let mut v = SoaVec3::with_capacity(4);
        v.push(1.0, 2.0, 3.0);
        v.push(4.0, 5.0, 6.0);
        v.push(7.0, 8.0, 9.0);
        let indices = vec![0, 2];
        v.add_batch_scalar(&indices, 10.0, 20.0, 30.0);
        assert_eq!(v.get(0), (11.0, 22.0, 33.0));
        assert_eq!(v.get(1), (4.0, 5.0, 6.0));
        assert_eq!(v.get(2), (17.0, 28.0, 39.0));
    }

    #[test]
    fn test_soa_scalar_scale_batch() {
        let mut s = SoaScalar::with_capacity(4);
        s.push(1.0);
        s.push(2.0);
        s.push(3.0);
        let indices = vec![0, 1, 2];
        for &idx in &indices {
            s.values[idx] *= 2.0;
        }
        assert!((s.values[0] - 2.0).abs() < 0.001);
        assert!((s.values[1] - 4.0).abs() < 0.001);
        assert!((s.values[2] - 6.0).abs() < 0.001);
    }
}
