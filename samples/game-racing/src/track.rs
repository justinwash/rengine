use rengine::Vec2;

use crate::track_visuals::{convert_points, OUTLINE_L, OUTLINE_R, TRACK_SCALE};

/// A closed cubic Bézier spline representing the racing line.
/// Points are stored as (position, in_tangent, out_tangent) matching the Godot Curve2D format.
pub struct Track {
    /// Evenly-spaced baked points along the curve.
    pub points: Vec<Vec2>,
    /// Total length of the baked curve.
    pub length: f32,
    /// Cumulative distance at each baked point.
    pub distances: Vec<f32>,
    /// Pre-computed curvature at each baked point (rad/unit).
    pub curvatures: Vec<f32>,
    /// Pre-computed target speed at each baked point.
    pub speed_profile: Vec<f32>,
    /// Track half-width for rendering the road surface.
    pub half_width: f32,
    /// Right track boundary polyline (in rengine coords).
    pub boundary_r: Vec<Vec2>,
    /// Left track boundary polyline (in rengine coords).
    pub boundary_l: Vec<Vec2>,
}

/// Raw control point for the cubic Bézier spline.
struct ControlPoint {
    pos: Vec2,
    in_cp: Vec2,  // control point relative to pos
    out_cp: Vec2, // control point relative to pos
}

impl Track {
    pub fn new() -> Self {
        let control_points = Self::godot_track_points();
        let bake_spacing = 5.0 * TRACK_SCALE;
        let (points, distances) = Self::bake_curve(&control_points, bake_spacing);
        let length = *distances.last().unwrap_or(&0.0);
        let curvatures = Self::compute_curvatures(&points);
        let speed_profile = Self::compute_speed_profile(&curvatures);
        let boundary_r = convert_points(OUTLINE_R);
        let boundary_l = convert_points(OUTLINE_L);

        Self {
            points,
            length,
            distances,
            curvatures,
            speed_profile,
            half_width: 45.0,
            boundary_r,
            boundary_l,
        }
    }

    /// Parsed directly from the Godot track.tscn Curve2D `_data`.
    /// Format per point: in_x, in_y, out_x, out_y, pos_x, pos_y  (6 floats)
    fn godot_track_points() -> Vec<ControlPoint> {
        // Raw data from Godot's PackedVector2Array (Godot uses Y-down).
        // Each group: in_handle_x, in_handle_y, out_handle_x, out_handle_y, pos_x, pos_y
        // We negate all Y values to convert from Godot Y-down to rengine Y-up.
        let raw: &[(f32, f32, f32, f32, f32, f32)] = &[
            (31.4186, 2.73205, -31.4186, -2.73205, 1101.0, 911.0),
            (46.5815, 2.25394, -46.5815, -2.25394, 796.0, 905.0),
            (9.01578, 12.7724, -9.01578, -12.7724, 725.0, 864.0),
            (24.0421, 15.7776, -24.0421, -15.7776, 702.0, 799.0),
            (49.5868, 0.751315, -49.5868, -0.751315, 595.0, 794.0),
            (53.3433, 17.2802, -53.3433, -17.2802, 429.0, 787.0),
            (14.0108, 30.9126, -14.0108, -30.9126, 315.0, 692.0),
            (-1.50263, 42.8249, 1.50263, -42.8249, 297.0, 563.0),
            (1.30789, 18.5684, -1.30789, -18.5684, 315.0, 446.0),
            (32.175, 16.3697, -32.175, -16.3697, 278.0, 373.0),
            (20.8855, 45.1579, -20.8855, -45.1579, 173.0, 335.0),
            (25.6579, 62.0921, -25.6579, -62.0921, 170.0, 226.0),
            (-35.7073, 15.6967, 35.7073, -15.6967, 207.0, 108.0),
            (-39.1894, -10.2481, 39.1894, 10.2481, 362.0, 97.0),
            (-22.0658, -25.6579, 22.0658, 25.6579, 445.0, 166.0),
            (-30.7895, -33.3553, 30.7895, 33.3553, 626.0, 467.0),
            (-34.5681, -11.7148, 34.5681, 11.7148, 765.0, 588.0),
            (-73.2741, -20.6854, 73.2741, 20.6854, 924.0, 586.0),
            (-47.7133, -34.4462, 47.7133, 34.4462, 1040.0, 685.0),
            (-59.5041, 1.65289, 59.5041, -1.65289, 1208.0, 720.0),
            (-78.7805, -8.36738, 78.7805, 8.36738, 1406.0, 704.0),
            (-16.0, -19.0, 16.0, 19.0, 1535.0, 757.0),
            (39.2094, -46.6857, -39.2094, 46.6857, 1523.0, 865.0),
            (30.0639, -1.32557, -30.0639, 1.32557, 1397.0, 911.0),
            (0.0, 0.0, 0.0, 0.0, 1161.0, 911.0),
            (0.0, 0.0, 0.0, 0.0, 1101.0, 911.0),
        ];

        raw.iter()
            .map(|&(in_x, in_y, out_x, out_y, px, py)| ControlPoint {
                // Negate Y to convert Godot Y-down → rengine Y-up, then apply world scale
                pos: Vec2::new(px, -py) * TRACK_SCALE,
                in_cp: Vec2::new(in_x, -in_y) * TRACK_SCALE,
                out_cp: Vec2::new(out_x, -out_y) * TRACK_SCALE,
            })
            .collect()
    }

    /// Evaluate a cubic Bézier segment at parameter t ∈ [0,1].
    fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let u = 1.0 - t;
        p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
    }

    /// Bake the closed Bézier spline into evenly-spaced sample points.
    fn bake_curve(cps: &[ControlPoint], spacing: f32) -> (Vec<Vec2>, Vec<f32>) {
        // First, densely sample the curve
        let segments = cps.len() - 1; // last point connects back (it IS the first)
        let samples_per_seg = 64;
        let mut dense: Vec<Vec2> = Vec::new();

        for seg in 0..segments {
            let next = (seg + 1) % cps.len();
            let p0 = cps[seg].pos;
            let p1 = cps[seg].pos + cps[seg].out_cp;
            let p2 = cps[next].pos + cps[next].in_cp;
            let p3 = cps[next].pos;

            let count = if seg == segments - 1 {
                samples_per_seg + 1
            } else {
                samples_per_seg
            };
            for i in 0..count {
                let t = i as f32 / samples_per_seg as f32;
                dense.push(Self::cubic_bezier(p0, p1, p2, p3, t));
            }
        }

        // Now resample at even spacing
        let mut cum_dist: Vec<f32> = vec![0.0];
        for i in 1..dense.len() {
            let d = dense[i].distance(dense[i - 1]);
            cum_dist.push(cum_dist[i - 1] + d);
        }
        let total_length = *cum_dist.last().unwrap();

        let num_points = (total_length / spacing).ceil() as usize;
        let mut points = Vec::with_capacity(num_points);
        let mut distances = Vec::with_capacity(num_points);
        let mut dense_idx = 0;

        for i in 0..num_points {
            let target_dist = i as f32 * spacing;
            while dense_idx + 1 < dense.len() && cum_dist[dense_idx + 1] < target_dist {
                dense_idx += 1;
            }
            if dense_idx + 1 >= dense.len() {
                break;
            }
            let seg_len = cum_dist[dense_idx + 1] - cum_dist[dense_idx];
            let t = if seg_len > 0.001 {
                (target_dist - cum_dist[dense_idx]) / seg_len
            } else {
                0.0
            };
            let p = dense[dense_idx].lerp(dense[dense_idx + 1], t);
            points.push(p);
            distances.push(target_dist);
        }

        (points, distances)
    }

    /// Compute curvature at each baked point using 3-point formula.
    fn compute_curvatures(points: &[Vec2]) -> Vec<f32> {
        let n = points.len();
        let mut curvatures = vec![0.0f32; n];
        if n < 3 {
            return curvatures;
        }
        for i in 0..n {
            let prev = if i == 0 { n - 1 } else { i - 1 };
            let next = (i + 1) % n;
            let v1 = (points[i] - points[prev]).normalize_or_zero();
            let v2 = (points[next] - points[i]).normalize_or_zero();
            let cross = v1.x * v2.y - v1.y * v2.x;
            let dot = v1.dot(v2);
            let angle = cross.atan2(dot);
            let dist = points[i].distance(points[prev]) + points[next].distance(points[i]);
            curvatures[i] = if dist > 0.1 { angle.abs() / dist } else { 0.0 };
        }
        curvatures
    }

    /// Compute target speed at each point using:
    /// 1. Forward pass: v = sqrt(a_lat / curvature) — cornering limit
    /// 2. Backward pass: propagate braking constraints so cars slow before corners
    /// 3. Forward pass: propagate acceleration constraints
    fn compute_speed_profile(curvatures: &[f32]) -> Vec<f32> {
        // Lowered from 175 to better match actual physics grip.
        let max_lateral_accel = 150.0 * TRACK_SCALE;
        let min_speed = 40.0 * TRACK_SCALE;
        let max_speed = 250.0 * TRACK_SCALE;
        let bake_spacing = 5.0 * TRACK_SCALE;
        // Max deceleration (braking): ~0.9g equivalent in our units
        let max_decel = 3000.0 * TRACK_SCALE;
        // Max acceleration: more limited than braking
        let max_accel = 1200.0 * TRACK_SCALE;

        let n = curvatures.len();
        if n == 0 {
            return vec![];
        }

        // Pass 1: cornering speed limit at each point
        let mut profile: Vec<f32> = curvatures
            .iter()
            .map(|&c| {
                let s = (max_lateral_accel / c.max(0.0005)).sqrt();
                s.clamp(min_speed, max_speed)
            })
            .collect();

        // Pass 2 (backward): ensure we can brake from point i to point i+1.
        // Walk backward around the loop: if profile[prev] is too fast to
        // decelerate to profile[cur], reduce profile[prev].
        // v_prev^2 = v_cur^2 + 2 * a * d  =>  v_prev = sqrt(v_cur^2 + 2*a*d)
        for _ in 0..2 {
            // Two passes around the loop to handle wrap-around
            for i in (0..n).rev() {
                let next = (i + 1) % n;
                let v_next = profile[next];
                let max_v = (v_next * v_next + 2.0 * max_decel * bake_spacing).sqrt();
                if profile[i] > max_v {
                    profile[i] = max_v;
                }
            }
        }

        // Pass 3 (forward): ensure we can accelerate from point i to point i+1.
        // v_next^2 = v_cur^2 + 2 * a * d  =>  v_next = sqrt(v_cur^2 + 2*a*d)
        for _ in 0..2 {
            for i in 0..n {
                let next = (i + 1) % n;
                let v_cur = profile[i];
                let max_v = (v_cur * v_cur + 2.0 * max_accel * bake_spacing).sqrt();
                if profile[next] > max_v {
                    profile[next] = max_v;
                }
            }
        }

        // Final clamp
        for v in &mut profile {
            *v = v.clamp(min_speed, max_speed);
        }

        profile
    }

    /// Find the closest baked point index to a world position.
    pub fn closest_index(&self, pos: Vec2) -> usize {
        let mut best = 0;
        let mut best_dist = f32::MAX;
        for (i, p) in self.points.iter().enumerate() {
            let d = pos.distance_squared(*p);
            if d < best_dist {
                best_dist = d;
                best = i;
            }
        }
        best
    }

    /// Get the offset (distance along curve) for a baked point index.
    pub fn offset_at(&self, idx: usize) -> f32 {
        self.distances[idx.min(self.distances.len() - 1)]
    }

    /// Sample a position at a given offset distance along the track.
    pub fn sample(&self, offset: f32) -> Vec2 {
        let offset = wrap_offset(offset, self.length);
        // Binary search for the segment
        let idx = self
            .distances
            .partition_point(|&d| d < offset)
            .min(self.points.len() - 1);
        if idx == 0 {
            return self.points[0];
        }
        let seg_len = self.distances[idx] - self.distances[idx - 1];
        let t = if seg_len > 0.001 {
            (offset - self.distances[idx - 1]) / seg_len
        } else {
            0.0
        };
        self.points[idx - 1].lerp(self.points[idx], t)
    }

    /// Get tangent direction at an offset.
    pub fn tangent_at(&self, offset: f32) -> Vec2 {
        let ahead = 10.0;
        let p0 = self.sample(offset);
        let p1 = self.sample(offset + ahead);
        (p1 - p0).normalize_or_zero()
    }

    /// Get perpendicular (left-pointing) direction at an offset.
    pub fn normal_at(&self, offset: f32) -> Vec2 {
        let t = self.tangent_at(offset);
        Vec2::new(-t.y, t.x)
    }

    /// Get closest offset on the curve to a world position.
    pub fn closest_offset(&self, pos: Vec2) -> f32 {
        let idx = self.closest_index(pos);
        self.distances[idx]
    }

    /// Get curvature at an offset.
    pub fn curvature_at_offset(&self, offset: f32) -> f32 {
        let n = self.points.len();
        if n == 0 {
            return 0.0;
        }
        let offset = wrap_offset(offset, self.length);
        let idx = self.distances.partition_point(|&d| d < offset).min(n - 1);
        self.curvatures[idx]
    }

    /// Get target speed at an offset.
    pub fn target_speed_at_offset(&self, offset: f32) -> f32 {
        let n = self.points.len();
        if n == 0 {
            return 200.0;
        }
        let offset = wrap_offset(offset, self.length);
        let idx = self.distances.partition_point(|&d| d < offset).min(n - 1);
        self.speed_profile[idx]
    }

    /// Grid positions for the starting grid (staggered 2-wide).
    pub fn grid_positions(&self, count: usize) -> Vec<(Vec2, f32)> {
        // Start/finish is near the beginning of the curve
        let start_offset = 0.0;
        let row_spacing = 40.0 * TRACK_SCALE;
        let col_offset = 12.0 * TRACK_SCALE;

        let mut positions = Vec::with_capacity(count);
        for i in 0..count {
            let row = i / 2;
            let col = i % 2;

            let offset = wrap_offset(start_offset - (row as f32 * row_spacing), self.length);
            let center = self.sample(offset);
            let normal = self.normal_at(offset);
            let tangent = self.tangent_at(offset);
            let lateral = if col == 0 { -col_offset } else { col_offset };
            let pos = center + normal * lateral;
            let angle = tangent.y.atan2(tangent.x);

            positions.push((pos, angle));
        }
        positions
    }

    /// Distance to the nearest track boundary.
    /// Returns (distance_to_nearest_edge, direction_toward_track_center).
    pub fn boundary_info(&self, pos: Vec2) -> (f32, Vec2) {
        let dr = closest_dist_to_polyline(&self.boundary_r, pos);
        let dl = closest_dist_to_polyline(&self.boundary_l, pos);
        let nearest_dist = dr.min(dl);
        let center = self.sample(self.closest_offset(pos));
        let to_center = (center - pos).normalize_or_zero();
        (nearest_dist, to_center)
    }

    /// Distance from a position to the nearest point on the racing line.
    pub fn racing_line_dist(&self, pos: Vec2) -> f32 {
        let idx = self.closest_index(pos);
        pos.distance(self.points[idx])
    }
}

/// Wrap an offset into [0, length) range.
pub fn wrap_offset(offset: f32, length: f32) -> f32 {
    if length <= 0.0 {
        return 0.0;
    }
    let m = offset % length;
    if m < 0.0 {
        m + length
    } else {
        m
    }
}

/// Find the minimum distance from a point to a closed polyline.
fn closest_dist_to_polyline(poly: &[Vec2], pos: Vec2) -> f32 {
    let n = poly.len();
    if n == 0 {
        return f32::MAX;
    }
    let mut best = f32::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let a = poly[i];
        let b = poly[j];
        let ab = b - a;
        let ap = pos - a;
        let len_sq = ab.length_squared();
        let t = if len_sq > 0.0001 {
            ap.dot(ab) / len_sq
        } else {
            0.0
        }
        .clamp(0.0, 1.0);
        let closest = a + ab * t;
        let d = pos.distance(closest);
        if d < best {
            best = d;
        }
    }
    best
}
