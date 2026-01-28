use noise::{NoiseFn, Perlin};
use std::f32::consts::TAU;

/// Represents an arena shape with optional Perlin noise-based edge distortion
/// for creating irregular, organic-looking boundaries.
#[derive(Clone)]
pub struct ArenaShape {
    pub base_radius: f32,
    pub distortion_strength: f32, // 0.0 to 0.3
    pub distortion_scale: f32,    // Noise frequency
    perlin: Perlin,
}

impl ArenaShape {
    /// Create a new arena shape with optional distortion.
    ///
    /// # Arguments
    /// * `base_radius` - The base radius of the arena
    /// * `distortion_strength` - How much the edge varies (0.0 = perfect circle, 0.3 = very irregular)
    /// * `distortion_scale` - Frequency of the distortion (higher = more bumps)
    /// * `seed` - Random seed for reproducible shapes
    pub fn new(base_radius: f32, distortion_strength: f32, distortion_scale: f32, seed: u64) -> Self {
        Self {
            base_radius,
            distortion_strength: distortion_strength.clamp(0.0, 0.3),
            distortion_scale,
            perlin: Perlin::new(seed as u32),
        }
    }

    /// Create a perfect circular arena (no distortion)
    pub fn circle(radius: f32) -> Self {
        Self {
            base_radius: radius,
            distortion_strength: 0.0,
            distortion_scale: 1.0,
            perlin: Perlin::new(0),
        }
    }

    /// Get the radius at a given angle (0 to TAU).
    /// When distortion_strength is 0, this returns base_radius.
    pub fn radius_at_angle(&self, angle: f32) -> f32 {
        if self.distortion_strength <= 0.0 {
            return self.base_radius;
        }

        // Sample Perlin noise on a circle
        let nx = angle.cos() * self.distortion_scale;
        let nz = angle.sin() * self.distortion_scale;
        let noise_value = self.perlin.get([nx as f64, nz as f64]) as f32; // -1 to 1

        self.base_radius * (1.0 + noise_value * self.distortion_strength)
    }

    /// Check if a point (x, z) is inside the arena boundary.
    pub fn contains(&self, x: f32, z: f32) -> bool {
        let distance = (x * x + z * z).sqrt();
        let angle = x.atan2(z);
        let radius_at_point = self.radius_at_angle(angle);
        distance <= radius_at_point
    }

    /// Check if a point is inside with a margin (positive = inside, negative = outside buffer)
    pub fn contains_with_margin(&self, x: f32, z: f32, margin: f32) -> bool {
        let distance = (x * x + z * z).sqrt();
        let angle = x.atan2(z);
        let radius_at_point = self.radius_at_angle(angle);
        distance <= radius_at_point - margin
    }

    /// Get the approximate area of the arena for density calculations.
    /// For circular arenas, this is exact. For distorted arenas, this samples
    /// the perimeter to get a reasonable approximation.
    pub fn approximate_area(&self) -> f32 {
        if self.distortion_strength <= 0.0 {
            return std::f32::consts::PI * self.base_radius * self.base_radius;
        }

        // Use numerical integration (shoelace formula) with sampling
        // Sample points around the perimeter
        let num_samples = 360;
        let angle_step = TAU / num_samples as f32;

        let mut area = 0.0;
        for i in 0..num_samples {
            let angle1 = i as f32 * angle_step;
            let angle2 = ((i + 1) % num_samples) as f32 * angle_step;

            let r1 = self.radius_at_angle(angle1);
            let r2 = self.radius_at_angle(angle2);

            let x1 = angle1.cos() * r1;
            let z1 = angle1.sin() * r1;
            let x2 = angle2.cos() * r2;
            let z2 = angle2.sin() * r2;

            // Shoelace formula contribution
            area += x1 * z2 - x2 * z1;
        }

        (area / 2.0).abs()
    }

    /// Get the approximate perimeter length for boulder count calculations.
    pub fn approximate_perimeter(&self) -> f32 {
        if self.distortion_strength <= 0.0 {
            return TAU * self.base_radius;
        }

        let num_samples = 360;
        let angle_step = TAU / num_samples as f32;

        let mut perimeter = 0.0;
        for i in 0..num_samples {
            let angle1 = i as f32 * angle_step;
            let angle2 = ((i + 1) % num_samples) as f32 * angle_step;

            let r1 = self.radius_at_angle(angle1);
            let r2 = self.radius_at_angle(angle2);

            let x1 = angle1.cos() * r1;
            let z1 = angle1.sin() * r1;
            let x2 = angle2.cos() * r2;
            let z2 = angle2.sin() * r2;

            let dx = x2 - x1;
            let dz = z2 - z1;
            perimeter += (dx * dx + dz * dz).sqrt();
        }

        perimeter
    }

    /// Get the minimum radius (for safe inner calculations)
    pub fn min_radius(&self) -> f32 {
        self.base_radius * (1.0 - self.distortion_strength)
    }

    /// Get the maximum radius (for outer bounds)
    pub fn max_radius(&self) -> f32 {
        self.base_radius * (1.0 + self.distortion_strength)
    }

    /// Get the edge proximity factor (0.0 at center, 1.0 at edge) for a given point.
    /// Used for edge falloff calculations.
    pub fn edge_proximity(&self, x: f32, z: f32) -> f32 {
        let distance = (x * x + z * z).sqrt();
        let angle = x.atan2(z);
        let radius_at_point = self.radius_at_angle(angle);
        (distance / radius_at_point).min(1.0)
    }
}

impl Default for ArenaShape {
    fn default() -> Self {
        Self::circle(40.0)
    }
}

impl std::fmt::Debug for ArenaShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArenaShape")
            .field("base_radius", &self.base_radius)
            .field("distortion_strength", &self.distortion_strength)
            .field("distortion_scale", &self.distortion_scale)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_contains() {
        let shape = ArenaShape::circle(10.0);
        assert!(shape.contains(0.0, 0.0));
        assert!(shape.contains(5.0, 5.0));
        assert!(!shape.contains(10.0, 10.0));
    }

    #[test]
    fn test_circle_area() {
        let shape = ArenaShape::circle(10.0);
        let expected = std::f32::consts::PI * 100.0;
        let actual = shape.approximate_area();
        assert!((actual - expected).abs() < 0.1);
    }

    #[test]
    fn test_distorted_area_reasonable() {
        let shape = ArenaShape::new(10.0, 0.2, 3.0, 42);
        let circular_area = std::f32::consts::PI * 100.0;
        let actual = shape.approximate_area();
        // Distorted area should be within 50% of circular area
        assert!(actual > circular_area * 0.5);
        assert!(actual < circular_area * 1.5);
    }
}
