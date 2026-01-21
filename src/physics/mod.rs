pub mod ground;
pub mod terrain;

use bevy::prelude::*;

pub use ground::{GroundSensorBundle, GroundSensorConfig, GroundState};
pub use terrain::TerrainSampler;

pub struct PhysicsUtilPlugin;

impl Plugin for PhysicsUtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, ground::ground_sensing_system);
    }
}
