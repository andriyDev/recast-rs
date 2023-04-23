use std::ops::{Deref, DerefMut};

use recastnavigation_sys::rcBuildContours;

use crate::{wrappers, CompactHeightfield, Context, HasRegions};

pub struct ContourBuildFlags {
  // Tessellate solid (impassable) edges during simplification.
  // By default, only this flag is set.
  pub tessellate_wall_edges: bool,
  // Tessellate edges between areas during simplification.
  pub tessellate_area_edges: bool,
}

pub struct ContourSet {
  pub(crate) contour_set: wrappers::RawContourSet,
}

impl ContourSet {
  pub fn new(
    compact_heightfield: &CompactHeightfield<HasRegions>,
    context: &mut Context,
    max_error: f32,
    max_edge_len: i32,
    build_flags: ContourBuildFlags,
  ) -> Result<ContourSet, ()> {
    let mut contour_set = wrappers::RawContourSet::new()?;

    let build_flags = (build_flags.tessellate_wall_edges as i32 * 0x01)
      | (build_flags.tessellate_area_edges as i32 * 0x02);

    // SAFETY: rcBuildContours only modifies `context.context` and
    // `contour_set`, both of which are taken by mutable borrows.
    // `compact_heightfield.compact_heightfield` is only read and is passed by
    // immutable borrow.
    let build_succeeded = unsafe {
      rcBuildContours(
        context.context.deref_mut(),
        compact_heightfield.compact_heightfield.deref(),
        max_error,
        max_edge_len,
        contour_set.deref_mut(),
        build_flags,
      )
    };

    if build_succeeded {
      Ok(ContourSet { contour_set })
    } else {
      Err(())
    }
  }
}
