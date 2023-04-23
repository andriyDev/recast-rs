use std::ops::{Deref, DerefMut};

use recastnavigation_sys::{
  rcBuildContours, rcBuildPolyMesh, rcBuildPolyMeshDetail,
};

mod vector;
mod wrappers;

mod compact_heightfield;
mod heightfield;
mod heightfield_layer_set;

pub use compact_heightfield::{
  CompactHeightfield, CompactHeightfieldState, HasRegions, NoRegions,
};
pub use heightfield::{Heightfield, HeightfieldSpan};
pub use heightfield_layer_set::{HeightfieldLayer, HeightfieldLayerSet};

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

pub struct ContourBuildFlags {
  // Tessellate solid (impassable) edges during simplification.
  // By default, only this flag is set.
  tessellate_wall_edges: bool,
  // Tessellate edges between areas during simplification.
  tessellate_area_edges: bool,
}

pub struct ContourSet {
  contour_set: wrappers::RawContourSet,
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

#[cfg(test)]
mod tests {
  use crate::{
    CompactHeightfield, Context, ContourBuildFlags, ContourSet, HasRegions,
    Heightfield, HeightfieldLayerSet, HeightfieldSpan, NoRegions, PolyMesh,
    PolyMeshDetail, Vec3, WALKABLE_AREA_ID,
  };

  macro_rules! assert_span_column_eq {
      ($span_column: expr, $expected_column: expr) => {{
        let span_column = $span_column;
        let expected_column = $expected_column;

        assert_eq!(span_column.len(), expected_column.len(), "\n\nactual_spans={:?}\nexpected_spans={:?}", span_column, expected_column);

        for (index, (actual_span, expected_span)) in span_column.iter().zip(expected_column.iter()).enumerate() {
          assert_eq!(
            (actual_span.height_min_u32(), actual_span.height_max_u32(), actual_span.area_id()),
            *expected_span,
            "\n\nColumn differs at index {}\n\nactual_span={:?}\nexpected_span=HeightfieldSpan {{ smin: {}, smax: {}, area: {} }}",
            index, actual_span, expected_span.0, expected_span.1, expected_span.2
          );
        }
      }};
  }

  #[test]
  fn rasterize_triangles() {
    let mut context = Context::new();

    let min_bounds = Vec3::new(0.0, 0.0, 0.0);
    let max_bounds = Vec3::new(5.0, 5.0, 5.0);

    let mut heightfield =
      Heightfield::new(&mut context, min_bounds, max_bounds, 0.5, 0.5)
        .expect("creation succeeds");

    let vertices = [
      Vec3::new(0.0, 0.25, 0.0),
      Vec3::new(5.0, 0.25, 0.0),
      Vec3::new(5.0, 0.25, 5.0),
      Vec3::new(5.0, 0.25, 5.0),
      Vec3::new(0.0, 0.25, 5.0),
      Vec3::new(0.0, 0.25, 0.0),
    ];

    let area_ids = [WALKABLE_AREA_ID, WALKABLE_AREA_ID];

    heightfield
      .rasterize_triangles(&mut context, &vertices, &area_ids, 1)
      .expect("rasterization succeeds");

    assert_eq!(heightfield.grid_width(), 10);
    assert_eq!(heightfield.grid_height(), 10);
    assert_eq!(heightfield.min_bounds(), min_bounds);
    assert_eq!(heightfield.max_bounds(), max_bounds);
    assert_eq!(heightfield.cell_horizontal_size(), 0.5);
    assert_eq!(heightfield.cell_height(), 0.5);

    let columns = heightfield
      .spans_iter()
      .map(|column_head| HeightfieldSpan::collect(column_head))
      .collect::<Vec<Vec<HeightfieldSpan>>>();
    assert_eq!(columns.len(), 100);

    let index_at = |x, y| x + y * heightfield.grid_width() as usize;

    for x in 0..heightfield.grid_width() as usize {
      for y in 0..heightfield.grid_height() as usize {
        assert_span_column_eq!(
          &columns[index_at(x, y)],
          [(0, 1, WALKABLE_AREA_ID as u32)]
        );
      }
    }
  }

  #[test]
  fn erode_area() {
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

    let mut compact_heightfield =
      CompactHeightfield::<NoRegions>::create_from_heightfield(
        &heightfield,
        &mut context,
        3,
        0,
      )
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");
  }

  fn build_regions_base(
    build_fn: fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()>,
  ) {
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

    build_fn(compact_heightfield, &mut context)
      .expect("building regions succeeds");
  }

  #[test]
  fn build_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_regions(context, 1, 1, 1)
    }

    build_regions_base(build_fn);
  }

  #[test]
  fn build_layer_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_layer_regions(context, 1, 1)
    }

    build_regions_base(build_fn);
  }

  #[test]
  fn build_monotone_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_regions_monotone(context, 1, 1, 1)
    }

    build_regions_base(build_fn);
  }

  #[test]
  fn build_heightfield_layers() {
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

    let mut compact_heightfield =
      CompactHeightfield::<NoRegions>::create_from_heightfield(
        &heightfield,
        &mut context,
        3,
        0,
      )
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");

    let layer_set = HeightfieldLayerSet::new(
      &compact_heightfield,
      &mut context,
      /* border_size= */ 0,
      /* walkable_height= */ 3,
    )
    .expect("heightfield layers created");

    let layer_set = layer_set.as_vec();
    assert_eq!(layer_set.len(), 1);

    let layer = &layer_set[0];
    assert_eq!(layer.min_bounds(), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(layer.max_bounds(), Vec3::new(5.0, 1.0, 5.0));
    assert_eq!(layer.cell_horizontal_size(), 1.0);
    assert_eq!(layer.cell_height(), 1.0);
    assert_eq!(layer.grid_width(), 5);
    assert_eq!(layer.grid_height(), 5);
    assert_eq!(layer.grid_min_bounds(), Vec3::<i32>::new(1, 1, 1));
    // TODO: Figure out why this is shrunk by 2 on this side instead of 1.
    assert_eq!(layer.grid_max_bounds(), Vec3::<i32>::new(3, 1, 3));
  }

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

  #[test]
  fn build_poly_mesh_and_poly_mesh_detail() {
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

    let contour_set = ContourSet::new(
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

    let poly_mesh =
      PolyMesh::new(&contour_set, &mut context, 5).expect("poly mesh built");
    PolyMeshDetail::new(
      &poly_mesh,
      &mut context,
      &compact_heightfield_with_regions,
      /* sample_distance= */ 1.0,
      /* sample_max_error= */ 0.1,
    )
    .expect("poly mesh detail built");
  }
}
