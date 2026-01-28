use bevy::prelude::*;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::arena::config::PondConfig;
use crate::arena::shape::ArenaShape;

/// Data for a single terrain pond
#[derive(Debug, Clone)]
pub struct PondData {
    pub center: Vec2,
    pub radius: f32,
}

/// Resource containing all generated terrain ponds
#[derive(Resource, Debug, Clone)]
pub struct TerrainPonds {
    pub ponds: Vec<PondData>,
}

impl Default for TerrainPonds {
    fn default() -> Self {
        Self { ponds: Vec::new() }
    }
}

impl TerrainPonds {
    /// Generate terrain ponds based on configuration
    pub fn generate(config: &PondConfig, shape: &ArenaShape, rng: &mut ChaCha8Rng) -> Self {
        let mut ponds: Vec<PondData> = Vec::new();
        let count = rng.gen_range(config.min_count..=config.max_count);

        for _ in 0..count {
            // Try up to 100 attempts to find a valid position
            for _ in 0..100 {
                // Random angle
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);

                // Get the radius at this angle for irregular shapes
                let edge_radius = shape.radius_at_angle(angle);

                // Random distance from center, excluding center and edge zones
                let min_dist = config.center_exclusion;
                let max_dist = (edge_radius - config.edge_exclusion).max(min_dist + 1.0);

                if max_dist <= min_dist {
                    continue; // Arena too small for this configuration
                }

                let dist = rng.gen_range(min_dist..max_dist);

                let center = Vec2::new(angle.cos() * dist, angle.sin() * dist);

                // Random radius (half of diameter)
                let radius = rng.gen_range(config.min_diameter..config.max_diameter) / 2.0;

                // Check if this pond is valid (not overlapping with existing ponds)
                let mut valid = true;
                for existing in &ponds {
                    let existing_center = existing.center;
                    let dist_between = center.distance(existing_center);
                    let min_spacing = existing.radius + radius + config.pond_exclusion;

                    if dist_between < min_spacing {
                        valid = false;
                        break;
                    }
                }

                // Also verify it's fully within arena bounds
                if valid {
                    // Check several points on the pond edge
                    for i in 0..8 {
                        let check_angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                        let check_x = center.x + check_angle.cos() * radius;
                        let check_z = center.y + check_angle.sin() * radius;

                        if !shape.contains_with_margin(check_x, check_z, config.edge_exclusion) {
                            valid = false;
                            break;
                        }
                    }
                }

                if valid {
                    ponds.push(PondData { center, radius });
                    break;
                }
            }
        }

        Self { ponds }
    }

    /// Check if a point is inside any pond
    pub fn is_in_pond(&self, x: f32, z: f32) -> bool {
        let pos = Vec2::new(x, z);
        for pond in &self.ponds {
            if pos.distance(pond.center) < pond.radius {
                return true;
            }
        }
        false
    }

    /// Get the distance to the nearest pond edge (negative if inside a pond)
    /// Returns None if no ponds exist
    pub fn distance_to_nearest_pond(&self, x: f32, z: f32) -> Option<f32> {
        if self.ponds.is_empty() {
            return None;
        }

        let pos = Vec2::new(x, z);
        let mut min_dist = f32::MAX;

        for pond in &self.ponds {
            let dist_to_center = pos.distance(pond.center);
            let dist_to_edge = dist_to_center - pond.radius;
            min_dist = min_dist.min(dist_to_edge);
        }

        Some(min_dist)
    }

    /// Get the signed distance to the nearest pond (negative inside, positive outside)
    /// Also returns the pond index if close enough to matter
    pub fn signed_distance_to_pond(&self, x: f32, z: f32) -> (f32, Option<usize>) {
        if self.ponds.is_empty() {
            return (f32::MAX, None);
        }

        let pos = Vec2::new(x, z);
        let mut min_dist = f32::MAX;
        let mut closest_idx = None;

        for (i, pond) in self.ponds.iter().enumerate() {
            let dist_to_center = pos.distance(pond.center);
            let dist_to_edge = dist_to_center - pond.radius;

            if dist_to_edge < min_dist {
                min_dist = dist_to_edge;
                closest_idx = Some(i);
            }
        }

        (min_dist, closest_idx)
    }
}
