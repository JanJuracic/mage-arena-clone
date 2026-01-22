use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::arena::shape::ArenaShape;

/// Generic parameter with min/max/default bounds for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter<T> {
    pub min: T,
    pub max: T,
    pub default: T,
}

impl<T: PartialOrd + Copy> Parameter<T> {
    /// Clamp a value to the parameter's bounds
    pub fn clamp(&self, value: T) -> T {
        if value < self.min {
            self.min
        } else if value > self.max {
            self.max
        } else {
            value
        }
    }

    /// Get the default value
    pub fn get_default(&self) -> T {
        self.default
    }
}

/// Seasonal theme for visual consistency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BiomeTheme {
    #[default]
    Summer,
    Autumn,
    Random,
}

/// Atmosphere preset defining time-of-day lighting and colors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmospherePreset {
    pub name: String,

    // Ground color (hue/saturation shifts from base green)
    pub ground_hue_shift: f32,   // -0.1 to 0.1 (negative = more blue, positive = more yellow)
    pub ground_saturation: f32,  // 0.5 to 1.0
    pub ground_brightness: f32,  // 0.2 to 0.4

    // Sky gradient
    pub sky_top_color: [f32; 3],    // RGB 0-1
    pub sky_bottom_color: [f32; 3], // RGB 0-1

    // Sun direction (Euler angles)
    pub sun_yaw: f32,   // 0 = North, PI/2 = East, PI = South, 3PI/2 = West
    pub sun_pitch: f32, // Negative = down from horizon (e.g., -0.3 = low, -0.8 = high)

    // Sun lighting
    pub sun_color: [f32; 3], // RGB tint
    pub sun_illuminance: f32, // 1000-3000 typical

    // Ambient lighting
    pub ambient_color: [f32; 3], // RGB tint
    pub ambient_brightness: f32, // 500-2000 typical
}

impl Default for AtmospherePreset {
    fn default() -> Self {
        Self {
            name: "Noon".to_string(),
            ground_hue_shift: 0.0,
            ground_saturation: 0.85,
            ground_brightness: 0.32,
            sky_top_color: [0.2, 0.4, 0.85],
            sky_bottom_color: [0.7, 0.8, 0.95],
            sun_yaw: 0.0,
            sun_pitch: -1.2,
            sun_color: [1.0, 1.0, 0.98],
            sun_illuminance: 3000.0,
            ambient_color: [0.5, 0.55, 0.65],
            ambient_brightness: 1800.0,
        }
    }
}

/// Returns the default atmosphere presets (Dawn, Noon, Dusk, Night)
pub fn default_atmosphere_presets() -> Vec<AtmospherePreset> {
    use std::f32::consts::FRAC_PI_2;
    use std::f32::consts::FRAC_PI_4;

    vec![
        // Dawn - warm orange light from East, low angle
        AtmospherePreset {
            name: "Dawn".to_string(),
            ground_hue_shift: 0.03,
            ground_saturation: 0.7,
            ground_brightness: 0.28,
            sky_top_color: [0.4, 0.5, 0.7],
            sky_bottom_color: [0.95, 0.6, 0.4],
            sun_yaw: FRAC_PI_2, // East
            sun_pitch: -0.35,   // Low angle (~20°)
            sun_color: [1.0, 0.85, 0.6],
            sun_illuminance: 1800.0,
            ambient_color: [0.6, 0.5, 0.5],
            ambient_brightness: 1200.0,
        },
        // Noon - bright white overhead
        AtmospherePreset {
            name: "Noon".to_string(),
            ground_hue_shift: 0.0,
            ground_saturation: 0.85,
            ground_brightness: 0.32,
            sky_top_color: [0.2, 0.4, 0.85],
            sky_bottom_color: [0.7, 0.8, 0.95],
            sun_yaw: 0.0,
            sun_pitch: -1.2, // High angle (~70°)
            sun_color: [1.0, 1.0, 0.98],
            sun_illuminance: 3000.0,
            ambient_color: [0.5, 0.55, 0.65],
            ambient_brightness: 1800.0,
        },
        // Dusk - red-orange light from West, low angle
        AtmospherePreset {
            name: "Dusk".to_string(),
            ground_hue_shift: 0.05,
            ground_saturation: 0.65,
            ground_brightness: 0.25,
            sky_top_color: [0.3, 0.35, 0.6],
            sky_bottom_color: [0.95, 0.45, 0.25],
            sun_yaw: -FRAC_PI_2, // West (3PI/2)
            sun_pitch: -0.3,     // Very low angle (~17°)
            sun_color: [1.0, 0.6, 0.35],
            sun_illuminance: 1500.0,
            ambient_color: [0.55, 0.45, 0.45],
            ambient_brightness: 1000.0,
        },
        // Night - blue moonlight, dark
        AtmospherePreset {
            name: "Night".to_string(),
            ground_hue_shift: -0.05,
            ground_saturation: 0.4,
            ground_brightness: 0.18,
            sky_top_color: [0.05, 0.08, 0.15],
            sky_bottom_color: [0.15, 0.12, 0.2],
            sun_yaw: FRAC_PI_4, // Moon position
            sun_pitch: -0.6,    // Medium angle
            sun_color: [0.7, 0.75, 0.9],
            sun_illuminance: 400.0,
            ambient_color: [0.3, 0.35, 0.5],
            ambient_brightness: 400.0,
        },
    ]
}

/// Size preset defining arena dimensions and shape distortion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizePreset {
    pub name: String,
    pub base_radius: f32,
    pub distortion_strength: f32, // 0.0 to 0.3
    pub distortion_scale: f32,    // Noise frequency for shape
}

impl Default for SizePreset {
    fn default() -> Self {
        Self {
            name: "Medium".to_string(),
            base_radius: 40.0,
            distortion_strength: 0.15,
            distortion_scale: 4.0,
        }
    }
}

/// Terrain preset defining height variation characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPreset {
    pub name: String,
    pub height_scale: f32,
    pub noise_scale: f32,
    pub octaves: u32,
    pub edge_falloff: f32, // How quickly terrain flattens at edges (0.0-1.0)
}

impl Default for TerrainPreset {
    fn default() -> Self {
        Self {
            name: "Rolling".to_string(),
            height_scale: 1.5,
            noise_scale: 0.08,
            octaves: 3,
            edge_falloff: 0.5,
        }
    }
}

/// Density configuration for spawn calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DensityConfig {
    pub obstacles_per_100_sqm: f32,
    pub grass_per_100_sqm: f32,
    pub mushrooms_per_100_sqm: f32,
}

impl Default for DensityConfig {
    fn default() -> Self {
        Self {
            obstacles_per_100_sqm: 0.8,   // ~40 for 5000 sqm arena (2x density)
            grass_per_100_sqm: 10.0,      // ~500 for 5000 sqm arena
            mushrooms_per_100_sqm: 1.0,   // ~50 for 5000 sqm arena
        }
    }
}

impl DensityConfig {
    /// Calculate spawn count based on arena area
    pub fn calculate_count(&self, density: f32, area: f32) -> u32 {
        (density * area / 100.0).round().max(1.0) as u32
    }

    pub fn obstacle_count(&self, area: f32) -> u32 {
        self.calculate_count(self.obstacles_per_100_sqm, area)
    }

    pub fn grass_count(&self, area: f32) -> u32 {
        self.calculate_count(self.grass_per_100_sqm, area)
    }

    pub fn mushroom_count(&self, area: f32) -> u32 {
        self.calculate_count(self.mushrooms_per_100_sqm, area)
    }
}

/// Obstacle spawning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObstacleConfigData {
    pub min_spacing: Parameter<f32>,
    pub spawn_exclusion: f32,
    pub center_exclusion: f32,
    pub tree_weight: f32,
    pub rock_weight: f32,
    pub boulder_weight: f32,
}

impl Default for ObstacleConfigData {
    fn default() -> Self {
        Self {
            min_spacing: Parameter {
                min: 4.0,
                max: 10.0,
                default: 6.0,
            },
            spawn_exclusion: 4.0,
            center_exclusion: 10.0,
            tree_weight: 0.60,
            rock_weight: 0.20,
            boulder_weight: 0.20,
        }
    }
}

/// Boulder boundary configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryConfigData {
    pub boulder_spacing: Parameter<f32>,
    pub boulder_size_variation: f32,
    pub inward_offset: Parameter<f32>,
    pub rotation_variation: f32,
    pub height_scale_min: f32,
    pub height_scale_max: f32,
}

impl Default for BoundaryConfigData {
    fn default() -> Self {
        Self {
            boulder_spacing: Parameter {
                min: 0.0,
                max: 2.0,
                default: 0.5,
            },
            boulder_size_variation: 0.15,
            inward_offset: Parameter {
                min: 1.0,
                max: 3.0,
                default: 2.0,
            },
            rotation_variation: std::f32::consts::PI,
            height_scale_min: 0.5,
            height_scale_max: 1.5,
        }
    }
}

/// Decoration spawning configuration (scale parameters, not counts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationConfigData {
    pub grass_scale: Parameter<f32>,
}

impl Default for DecorationConfigData {
    fn default() -> Self {
        Self {
            grass_scale: Parameter {
                min: 0.5,
                max: 1.0,
                default: 0.7,
            },
        }
    }
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfigData {
    pub active_theme: BiomeTheme,
    pub dead_tree_chance_summer: f32,
    pub dead_tree_chance_autumn: f32,
}

impl Default for ThemeConfigData {
    fn default() -> Self {
        Self {
            active_theme: BiomeTheme::Summer,
            dead_tree_chance_summer: 0.05,
            dead_tree_chance_autumn: 0.20,
        }
    }
}

/// Main configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaConfigFile {
    // Preset definitions
    pub size_presets: Vec<SizePreset>,
    pub terrain_presets: Vec<TerrainPreset>,
    #[serde(default = "default_atmosphere_presets")]
    pub atmosphere_presets: Vec<AtmospherePreset>,

    // Active selections (preset name or "Random")
    pub active_size: String,
    pub active_terrain: String,
    #[serde(default = "default_active_atmosphere")]
    pub active_atmosphere: String,

    // Density-based spawning
    pub density: DensityConfig,

    // Other configuration
    pub obstacles: ObstacleConfigData,
    pub boundary: BoundaryConfigData,
    pub decoration: DecorationConfigData,
    pub themes: ThemeConfigData,

    // Generation settings
    pub subdivisions: u32,
    pub seed: Option<u64>,
}

fn default_active_atmosphere() -> String {
    "Random".to_string()
}

impl Default for ArenaConfigFile {
    fn default() -> Self {
        Self {
            size_presets: vec![
                SizePreset {
                    name: "Small".to_string(),
                    base_radius: 30.0,
                    distortion_strength: 0.1,
                    distortion_scale: 3.0,
                },
                SizePreset {
                    name: "Medium".to_string(),
                    base_radius: 40.0,
                    distortion_strength: 0.15,
                    distortion_scale: 4.0,
                },
                SizePreset {
                    name: "Large".to_string(),
                    base_radius: 55.0,
                    distortion_strength: 0.2,
                    distortion_scale: 5.0,
                },
            ],
            terrain_presets: vec![
                TerrainPreset {
                    name: "Flat".to_string(),
                    height_scale: 0.5,
                    noise_scale: 0.06,
                    octaves: 2,
                    edge_falloff: 0.3,
                },
                TerrainPreset {
                    name: "Rolling".to_string(),
                    height_scale: 1.5,
                    noise_scale: 0.08,
                    octaves: 3,
                    edge_falloff: 0.5,
                },
                TerrainPreset {
                    name: "Hilly".to_string(),
                    height_scale: 2.5,
                    noise_scale: 0.10,
                    octaves: 4,
                    edge_falloff: 0.7,
                },
            ],
            atmosphere_presets: default_atmosphere_presets(),
            active_size: "Random".to_string(),
            active_terrain: "Random".to_string(),
            active_atmosphere: "Random".to_string(),
            density: DensityConfig::default(),
            obstacles: ObstacleConfigData::default(),
            boundary: BoundaryConfigData::default(),
            decoration: DecorationConfigData::default(),
            themes: ThemeConfigData::default(),
            subdivisions: 64,
            seed: None,
        }
    }
}

impl ArenaConfigFile {
    /// Load configuration from a RON file
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        ron::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))
    }

    /// Load configuration with fallback to defaults
    pub fn load_or_default(path: &str) -> Self {
        match Self::load_from_file(path) {
            Ok(config) => {
                info!("Loaded arena config from '{}'", path);
                config
            }
            Err(e) => {
                warn!("{}", e);
                warn!("Using default arena configuration");
                Self::default()
            }
        }
    }

    /// Get the resolved seed (either from config or generate random)
    pub fn get_seed(&self) -> u64 {
        self.seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(42)
        })
    }

    /// Get the resolved theme (resolve Random to a concrete theme)
    pub fn get_resolved_theme(&self, rng: &mut impl rand::Rng) -> BiomeTheme {
        match self.themes.active_theme {
            BiomeTheme::Random => {
                if rng.gen_bool(0.5) {
                    BiomeTheme::Summer
                } else {
                    BiomeTheme::Autumn
                }
            }
            theme => theme,
        }
    }

    /// Get the resolved size preset
    pub fn get_resolved_size_preset(&self, rng: &mut impl rand::Rng) -> SizePreset {
        if self.active_size == "Random" {
            if self.size_presets.is_empty() {
                return SizePreset::default();
            }
            let index = rng.gen_range(0..self.size_presets.len());
            self.size_presets[index].clone()
        } else {
            self.size_presets
                .iter()
                .find(|p| p.name == self.active_size)
                .cloned()
                .unwrap_or_default()
        }
    }

    /// Get the resolved terrain preset
    pub fn get_resolved_terrain_preset(&self, rng: &mut impl rand::Rng) -> TerrainPreset {
        if self.active_terrain == "Random" {
            if self.terrain_presets.is_empty() {
                return TerrainPreset::default();
            }
            let index = rng.gen_range(0..self.terrain_presets.len());
            self.terrain_presets[index].clone()
        } else {
            self.terrain_presets
                .iter()
                .find(|p| p.name == self.active_terrain)
                .cloned()
                .unwrap_or_default()
        }
    }

    /// Get the resolved atmosphere preset
    pub fn get_resolved_atmosphere_preset(&self, rng: &mut impl rand::Rng) -> AtmospherePreset {
        if self.active_atmosphere == "Random" {
            if self.atmosphere_presets.is_empty() {
                return AtmospherePreset::default();
            }
            let index = rng.gen_range(0..self.atmosphere_presets.len());
            self.atmosphere_presets[index].clone()
        } else {
            self.atmosphere_presets
                .iter()
                .find(|p| p.name == self.active_atmosphere)
                .cloned()
                .unwrap_or_default()
        }
    }
}

/// Runtime arena configuration resource (values after parameter resolution)
#[derive(Resource, Debug, Clone)]
pub struct ArenaConfig {
    // Shape parameters
    pub shape: ArenaShape,

    // Terrain parameters
    pub subdivisions: u32,
    pub height_scale: f32,
    pub noise_scale: f32,
    pub octaves: u32,
    pub edge_falloff: f32,

    // Computed values
    pub area: f32,

    // Generation metadata
    pub seed: u64,
    pub theme: BiomeTheme,
    pub size_preset_name: String,
    pub terrain_preset_name: String,
    pub atmosphere_preset_name: String,

    // Resolved atmosphere values
    pub ground_hue_shift: f32,
    pub ground_saturation: f32,
    pub ground_brightness: f32,
    pub sky_top_color: [f32; 3],
    pub sky_bottom_color: [f32; 3],
    pub sun_yaw: f32,
    pub sun_pitch: f32,
    pub sun_color: [f32; 3],
    pub sun_illuminance: f32,
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
}

impl ArenaConfig {
    pub fn from_config_file(
        file: &ArenaConfigFile,
        seed: u64,
        theme: BiomeTheme,
        size_preset: &SizePreset,
        terrain_preset: &TerrainPreset,
        atmosphere_preset: &AtmospherePreset,
    ) -> Self {
        let shape = ArenaShape::new(
            size_preset.base_radius,
            size_preset.distortion_strength,
            size_preset.distortion_scale,
            seed,
        );
        let area = shape.approximate_area();

        Self {
            shape,
            subdivisions: file.subdivisions,
            height_scale: terrain_preset.height_scale,
            noise_scale: terrain_preset.noise_scale,
            octaves: terrain_preset.octaves,
            edge_falloff: terrain_preset.edge_falloff,
            area,
            seed,
            theme,
            size_preset_name: size_preset.name.clone(),
            terrain_preset_name: terrain_preset.name.clone(),
            atmosphere_preset_name: atmosphere_preset.name.clone(),
            ground_hue_shift: atmosphere_preset.ground_hue_shift,
            ground_saturation: atmosphere_preset.ground_saturation,
            ground_brightness: atmosphere_preset.ground_brightness,
            sky_top_color: atmosphere_preset.sky_top_color,
            sky_bottom_color: atmosphere_preset.sky_bottom_color,
            sun_yaw: atmosphere_preset.sun_yaw,
            sun_pitch: atmosphere_preset.sun_pitch,
            sun_color: atmosphere_preset.sun_color,
            sun_illuminance: atmosphere_preset.sun_illuminance,
            ambient_color: atmosphere_preset.ambient_color,
            ambient_brightness: atmosphere_preset.ambient_brightness,
        }
    }

    /// Get the base radius (for backwards compatibility)
    pub fn radius(&self) -> f32 {
        self.shape.base_radius
    }
}

/// Runtime obstacle configuration resource
#[derive(Resource, Debug, Clone)]
pub struct ObstacleConfig {
    pub count: u32,
    pub min_spacing: f32,
    pub spawn_exclusion: f32,
    pub center_exclusion: f32,
    pub tree_weight: f32,
    pub rock_weight: f32,
    pub boulder_weight: f32,
}

impl ObstacleConfig {
    pub fn from_config_file(file: &ArenaConfigFile, area: f32) -> Self {
        Self {
            count: file.density.obstacle_count(area),
            min_spacing: file.obstacles.min_spacing.get_default(),
            spawn_exclusion: file.obstacles.spawn_exclusion,
            center_exclusion: file.obstacles.center_exclusion,
            tree_weight: file.obstacles.tree_weight,
            rock_weight: file.obstacles.rock_weight,
            boulder_weight: file.obstacles.boulder_weight,
        }
    }
}

/// Runtime boundary configuration resource
#[derive(Resource, Debug, Clone)]
pub struct BoundaryConfig {
    pub boulder_spacing: f32,
    pub boulder_size_variation: f32,
    pub inward_offset: f32,
    pub rotation_variation: f32,
    pub height_scale_min: f32,
    pub height_scale_max: f32,
}

impl BoundaryConfig {
    pub fn from_config_file(file: &ArenaConfigFile) -> Self {
        Self {
            boulder_spacing: file.boundary.boulder_spacing.get_default(),
            boulder_size_variation: file.boundary.boulder_size_variation,
            inward_offset: file.boundary.inward_offset.get_default(),
            rotation_variation: file.boundary.rotation_variation,
            height_scale_min: file.boundary.height_scale_min,
            height_scale_max: file.boundary.height_scale_max,
        }
    }
}

/// Runtime decoration configuration resource
#[derive(Resource, Debug, Clone)]
pub struct DecorationConfig {
    pub grass_count: u32,
    pub mushroom_count: u32,
    pub grass_scale: f32,
    pub dead_tree_chance: f32,
}

impl DecorationConfig {
    pub fn from_config_file(file: &ArenaConfigFile, theme: BiomeTheme, area: f32) -> Self {
        let dead_tree_chance = match theme {
            BiomeTheme::Summer => file.themes.dead_tree_chance_summer,
            BiomeTheme::Autumn => file.themes.dead_tree_chance_autumn,
            BiomeTheme::Random => file.themes.dead_tree_chance_summer, // Shouldn't reach here
        };

        Self {
            grass_count: file.density.grass_count(area),
            mushroom_count: file.density.mushroom_count(area),
            grass_scale: file.decoration.grass_scale.get_default(),
            dead_tree_chance,
        }
    }
}

/// Store the raw config file for reference
#[derive(Resource, Debug, Clone, Default)]
pub struct ArenaConfigFileRes(pub ArenaConfigFile);
