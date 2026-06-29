use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct SmoothConfig {
    pub max_smooth_iterations: usize,
    pub tolerance: f32,
    pub corner_weight: f32,
    pub smoothness: f32,
}

impl Default for SmoothConfig {
    fn default() -> Self {
        Self { max_smooth_iterations: 10, tolerance: 0.01, corner_weight: 0.5, smoothness: 0.3 }
    }
}

pub fn smooth_path(path: &[[f32; 3]], config: &SmoothConfig) -> Vec<[f32; 3]> {
    if path.len() <= 2 {
        return path.to_vec();
    }

    let mut smoothed: Vec<[f32; 3]> = path.to_vec();

    for _ in 0..config.max_smooth_iterations {
        let mut changed = false;
        let mut new_path = smoothed.clone();

        for i in 1..(smoothed.len() - 1) {
            let prev = Vec3::from_array(smoothed[i - 1]);
            let curr = Vec3::from_array(smoothed[i]);
            let next = Vec3::from_array(smoothed[i + 1]);

            let mid = (prev + next) * 0.5;

            let d1 = curr - prev;
            let d2 = next - curr;
            let d1_n = d1.normalize_or_zero();
            let d2_n = d2.normalize_or_zero();

            let corner = 1.0 - (d1_n.dot(d2_n) * 0.5 + 0.5);
            let weight = config.smoothness * (1.0 + corner * config.corner_weight);

            let new_pos = curr.lerp(mid, weight.min(1.0));
            new_path[i] = new_pos.to_array();

            if (curr - new_pos).length() > config.tolerance {
                changed = true;
            }
        }

        smoothed = new_path;
        if !changed {
            break;
        }
    }

    smoothed
}

pub fn simplify_path(path: &[[f32; 3]], max_error: f32) -> Vec<[f32; 3]> {
    if path.len() <= 2 {
        return path.to_vec();
    }

    let mut result = vec![path[0]];
    let mut last_kept = 0;

    for i in 1..(path.len() - 1) {
        let p0 = Vec3::from_array(path[last_kept]);
        let p1 = Vec3::from_array(path[i]);
        let p2 = Vec3::from_array(path[i + 1]);

        let line_dir = (p2 - p0).normalize_or_zero();
        let error = (p1 - p0).cross(line_dir).length();

        if error > max_error {
            result.push(path[i]);
            last_kept = i;
        }
    }

    result.push(*path.last().unwrap());
    result
}

pub fn string_pull_path(path: &[[f32; 3]]) -> Vec<[f32; 3]> {
    if path.len() <= 2 {
        return path.to_vec();
    }

    let mut result = vec![path[0]];
    let mut portal_left = 0;
    let mut portal_right = 0;
    let mut apex = 0;
    let mut i = 2;

    while i < path.len() {
        let a = Vec3::from_array(path[apex]);
        let b = Vec3::from_array(path[portal_left]);
        let c = Vec3::from_array(path[i]);

        let ab = b - a;
        let ac = c - a;
        let cross = ab.cross(ac);

        if cross.y > 0.0
            && (portal_left == apex
                || {
                    let d = Vec3::from_array(path[portal_right]);
                    let ad = d - a;
                    ab.cross(ad).y >= 0.0
                })
        {
            portal_left = i;
        } else if cross.y < 0.0
            && (portal_right == apex
                || {
                    let d = Vec3::from_array(path[portal_left]);
                    let ad = d - a;
                    ab.cross(ad).y <= 0.0
                })
        {
            portal_right = i;
        }

        if portal_left != apex && portal_right != apex {
            let left_vec = Vec3::from_array(path[portal_left]);
            let right_vec = Vec3::from_array(path[portal_right]);
            let al = left_vec - a;
            let ar = right_vec - a;

            if al.cross(ar).y <= 0.0 {
                let chosen = if portal_left > portal_right { portal_left } else { portal_right };
                result.push(path[chosen]);
                apex = chosen;
                portal_left = apex;
                portal_right = apex;
                i = apex + 1;
                continue;
            }
        }
        i += 1;
    }

    result.push(*path.last().unwrap());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_straight_line() {
        let path = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let config = SmoothConfig::default();
        let smoothed = smooth_path(&path, &config);
        assert_eq!(smoothed.len(), 3);
    }

    #[test]
    fn test_smooth_zigzag() {
        let path = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]];
        let config = SmoothConfig::default();
        let smoothed = smooth_path(&path, &config);
        assert_eq!(smoothed.len(), 3);
        assert!(smoothed[1][1] < 1.0);
    }

    #[test]
    fn test_simplify_reduces_points() {
        let path = vec![
            [0.0, 0.0, 0.0],
            [0.5, 0.01, 0.0],
            [1.0, 0.0, 0.0],
            [1.5, -0.01, 0.0],
            [2.0, 0.0, 0.0],
        ];
        let simplified = simplify_path(&path, 0.1);
        assert!(simplified.len() < 5);
    }

    #[test]
    fn test_string_pull_preserves_endpoints() {
        let path = vec![[0.0, 0.0, 0.0], [1.0, 0.5, 0.0], [2.0, 0.0, 0.0]];
        let pulled = string_pull_path(&path);
        assert_eq!(pulled[0], [0.0, 0.0, 0.0]);
        assert_eq!(*pulled.last().unwrap(), [2.0, 0.0, 0.0]);
    }
}
