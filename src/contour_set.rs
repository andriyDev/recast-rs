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

#[cfg(test)]
mod tests {
  use crate::{
    CompactHeightfield, Context, ContourBuildFlags, ContourSet, Heightfield,
    NoRegions, Vec3, WALKABLE_AREA_ID,
  };

  #[test]
  fn build_contour_set() {
    let mut context = Context::new();

    let min_bounds = Vec3::new(0.0, 0.0, 0.0);
    let max_bounds = Vec3::new(5.0, 5.0, 5.0);

    let mut heightfield =
      Heightfield::new(&mut context, min_bounds, max_bounds, 1.0, 1.0)
        .expect("creation succeeds");

    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(5.0, 0.5, 0.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(0.0, 0.5, 5.0),
    ];

    let area_ids = [WALKABLE_AREA_ID, WALKABLE_AREA_ID];

    heightfield
      .rasterize_triangles(&mut context, &vertices, &area_ids, 1)
      .expect("rasterization succeeds");

    let compact_heightfield =
      CompactHeightfield::<NoRegions>::create_from_heightfield(
        &heightfield,
        &mut context,
        3,
        0,
      )
      .expect("creating CompactHeightfield succeeds");

    let compact_heightfield_with_regions = compact_heightfield
      .build_regions(&mut context, 0, 1, 1)
      .expect("regions built");

    ContourSet::new(
      &compact_heightfield_with_regions,
      &mut context,
      /* max_error= */ 1.0,
      /* max_edge_len= */ 10,
      ContourBuildFlags {
        tessellate_wall_edges: true,
        tessellate_area_edges: false,
      },
    )
    .expect("contours built");
  }
}
