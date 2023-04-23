mod vector;
mod wrappers;

mod compact_heightfield;
mod contour_set;
mod heightfield;
mod heightfield_layer_set;
mod poly_mesh;

pub use compact_heightfield::{
  CompactHeightfield, CompactHeightfieldState, HasRegions, NoRegions,
};
pub use contour_set::{ContourBuildFlags, ContourSet};
pub use heightfield::{Heightfield, HeightfieldSpan};
pub use heightfield_layer_set::{HeightfieldLayer, HeightfieldLayerSet};
pub use poly_mesh::{PolyMesh, PolyMeshDetail};

pub use recastnavigation_sys::{
  RC_NULL_AREA as INVALID_AREA_ID, RC_WALKABLE_AREA as WALKABLE_AREA_ID,
};
pub use vector::Vec3;

pub struct Context {
  context: wrappers::RawContext,
}

impl Context {
  pub fn new() -> Self {
    Context { context: wrappers::RawContext::new() }
  }
}
