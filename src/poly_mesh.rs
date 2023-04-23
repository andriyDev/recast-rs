use std::ops::{Deref, DerefMut};

use recastnavigation_sys::{rcBuildPolyMesh, rcBuildPolyMeshDetail};

use crate::{
  wrappers, CompactHeightfield, CompactHeightfieldState, Context, ContourSet,
};

pub struct PolyMesh {
  poly_mesh: wrappers::RawPolyMesh,
}

impl PolyMesh {
  pub fn new(
    contour_set: &ContourSet,
    context: &mut Context,
    max_vertices_per_polygon: i32,
  ) -> Result<PolyMesh, ()> {
    let mut poly_mesh = wrappers::RawPolyMesh::new()?;

    // SAFETY: rcBuildPolyMesh only modifies `context.context` and `poly_mesh`,
    // both of which are taken by mutable borrows. `contour_set.contour_set` is
    // only read and is passed by immutable borrow.
    let build_succeeded = unsafe {
      rcBuildPolyMesh(
        context.context.deref_mut(),
        contour_set.contour_set.deref(),
        max_vertices_per_polygon,
        poly_mesh.deref_mut(),
      )
    };

    if build_succeeded {
      Ok(PolyMesh { poly_mesh })
    } else {
      Err(())
    }
  }
}

pub struct PolyMeshDetail {
  poly_mesh_detail: wrappers::RawPolyMeshDetail,
}

impl PolyMeshDetail {
  pub fn new(
    poly_mesh: &PolyMesh,
    context: &mut Context,
    compact_heightfield: &CompactHeightfield<impl CompactHeightfieldState>,
    sample_distance: f32,
    sample_max_error: f32,
  ) -> Result<PolyMeshDetail, ()> {
    let mut poly_mesh_detail = wrappers::RawPolyMeshDetail::new()?;

    // SAFETY: rcBuildPolyMeshDetail only modifies `context.context` and
    // `poly_mesh_detail`, both of which are taken by mutable borrows.
    // `poly_mesh.poly_mesh` and `compact_heightfield.compact_heightfield` are
    // only read and are passed by immutable borrows.
    let build_succeeded = unsafe {
      rcBuildPolyMeshDetail(
        context.context.deref_mut(),
        poly_mesh.poly_mesh.deref(),
        compact_heightfield.compact_heightfield.deref(),
        sample_distance,
        sample_max_error,
        poly_mesh_detail.deref_mut(),
      )
    };

    if build_succeeded {
      Ok(PolyMeshDetail { poly_mesh_detail })
    } else {
      Err(())
    }
  }
}
