use bevy::prelude::*;

use crate::arena::config::BiomeTheme;

/// Model metadata for spawning calculations
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub handle: Handle<Scene>,
    /// Radius for enemy movement avoidance (trunk/solid area)
    pub avoidance_radius: f32,
    /// Radius for spawn exclusion (canopy/total footprint)
    pub spawn_radius: f32,
}

/// All terrain assets organized by category and theme
#[derive(Resource, Default)]
pub struct TerrainAssets {
    // Boundary boulders (large, for wall)
    pub boulders: Vec<ModelMetadata>,
    // Interior boulders (same models as boundary, but smaller for obstacles)
    pub interior_boulders: Vec<ModelMetadata>,
    // Interior rocks (smaller, for obstacles)
    pub rocks: Vec<ModelMetadata>,
    // Trees by theme
    pub trees_summer: Vec<ModelMetadata>,
    pub trees_autumn: Vec<ModelMetadata>,
    pub trees_dead: Vec<ModelMetadata>,
    // Grass by theme
    pub grass_summer: Vec<Handle<Scene>>,
    pub grass_autumn: Vec<Handle<Scene>>,
    // Mushrooms
    pub mushrooms_summer: Vec<Handle<Scene>>,
    pub mushrooms_autumn: Vec<Handle<Scene>>,
}

impl TerrainAssets {
    /// Get trees for the specified theme
    pub fn get_trees_for_theme(&self, theme: BiomeTheme) -> &[ModelMetadata] {
        match theme {
            BiomeTheme::Summer => &self.trees_summer,
            BiomeTheme::Autumn => &self.trees_autumn,
            BiomeTheme::Random => &self.trees_summer, // Shouldn't reach here, resolved earlier
        }
    }

    /// Get grass for the specified theme
    pub fn get_grass_for_theme(&self, theme: BiomeTheme) -> &[Handle<Scene>] {
        match theme {
            BiomeTheme::Summer => &self.grass_summer,
            BiomeTheme::Autumn => &self.grass_autumn,
            BiomeTheme::Random => &self.grass_summer,
        }
    }

    /// Get mushrooms for the specified theme
    pub fn get_mushrooms_for_theme(&self, theme: BiomeTheme) -> &[Handle<Scene>] {
        match theme {
            BiomeTheme::Summer => &self.mushrooms_summer,
            BiomeTheme::Autumn => &self.mushrooms_autumn,
            BiomeTheme::Random => &self.mushrooms_summer,
        }
    }

    /// Get dead trees (available in both themes with different probabilities)
    pub fn get_dead_trees(&self) -> &[ModelMetadata] {
        &self.trees_dead
    }
}

/// System to load all terrain assets at startup
pub fn load_terrain_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Helper to load a scene with metadata
    let load_with_meta =
        |path: &str, avoidance_radius: f32, spawn_radius: f32| -> ModelMetadata {
            ModelMetadata {
                handle: asset_server.load(format!("{}#Scene0", path)),
                avoidance_radius,
                spawn_radius,
            }
        };

    // Helper to load just a scene handle
    let load_scene = |path: &str| -> Handle<Scene> {
        asset_server.load(format!("{}#Scene0", path))
    };

    // Boundary boulders (large, for perimeter wall)
    let boulders = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_01.glb",
            2.0,
            3.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_02.glb",
            2.0,
            3.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_03.glb",
            2.0,
            3.0,
        ),
    ];

    // Interior boulders (same models as boundary, but treated as obstacles with smaller radii)
    let interior_boulders = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_01.glb",
            1.5,
            2.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_02.glb",
            1.5,
            2.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Boulder_03.glb",
            1.5,
            2.5,
        ),
    ];

    // Interior rocks (smaller, for terrain variety)
    let rocks = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Rock_05.glb",
            1.2,
            1.8,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Rock_06.glb",
            1.2,
            1.8,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Rock_07.glb",
            1.2,
            1.8,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Rock_08.glb",
            1.2,
            1.8,
        ),
    ];

    // Summer trees (green variants)
    let trees_summer = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Oak_large_green.glb",
            1.2,
            3.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Oak_medium_green.glb",
            1.0,
            2.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Windswept_green.glb",
            1.0,
            3.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Windswept_dark_green.glb",
            1.0,
            3.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Cypress_small.glb",
            0.8,
            1.5,
        ),
    ];

    // Autumn trees (orange/red variants)
    let trees_autumn = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Oak_large_orange.glb",
            1.2,
            3.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Oak_medium_orange.glb",
            1.0,
            2.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Windswept_orange.glb",
            1.0,
            3.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Windswept_red.glb",
            1.0,
            3.0,
        ),
    ];

    // Dead trees (used sparingly in both themes)
    let trees_dead = vec![
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Dead_5_small.glb",
            0.6,
            1.5,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Dead_5_medium.glb",
            0.8,
            2.0,
        ),
        load_with_meta(
            "models/environment/colliders/Gradient_Tree_Dead_5_big.glb",
            1.0,
            2.5,
        ),
    ];

    // Summer grass
    let grass_summer = vec![
        load_scene("models/environment/non-colliders/Gradient_Grass_small_summer.glb"),
        load_scene("models/environment/non-colliders/Gradient_Grass_medium_summer.glb"),
        load_scene("models/environment/non-colliders/Gradient_Grass_large_summer.glb"),
    ];

    // Autumn grass
    let grass_autumn = vec![
        load_scene("models/environment/non-colliders/Gradient_Grass_small_autumn.glb"),
        load_scene("models/environment/non-colliders/Gradient_Grass_medium_autumn.glb"),
        load_scene("models/environment/non-colliders/Gradient_Grass_large_autumn.glb"),
    ];

    // Mushrooms for summer (brown only, subtle)
    let mushrooms_summer = vec![
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Brown_small.glb"),
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Brown_medium.glb"),
    ];

    // Mushrooms for autumn (all varieties)
    let mushrooms_autumn = vec![
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Brown_small.glb"),
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Brown_medium.glb"),
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Brown_large.glb"),
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Red_small.glb"),
        load_scene("models/environment/non-colliders/Gradient_Mushroom_Red_large.glb"),
    ];

    commands.insert_resource(TerrainAssets {
        boulders,
        interior_boulders,
        rocks,
        trees_summer,
        trees_autumn,
        trees_dead,
        grass_summer,
        grass_autumn,
        mushrooms_summer,
        mushrooms_autumn,
    });
}
