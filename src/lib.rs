use std::ops::{Deref, DerefMut};

use recastnavigation_sys::{
  rcBuildCompactHeightfield, rcBuildContours, rcBuildDistanceField,
  rcBuildHeightfieldLayers, rcBuildLayerRegions, rcBuildRegions,
  rcBuildRegionsMonotone, rcCalcGridSize, rcCreateHeightfield,
  rcErodeWalkableArea, rcRasterizeTriangles2,
};

mod vector;
mod wrappers;

pub use recastnavigation_sys::{
  RC_NULL_AREA as INVALID_AREA_ID, RC_WALKABLE_AREA as WALKABLE_AREA_ID,
};
use vector::IVec3;
pub use vector::Vec3;

pub struct Context {
  context: wrappers::RawContext,
}

impl Context {
  pub fn new() -> Self {
    Context { context: wrappers::RawContext::new() }
  }
}

pub struct Heightfield {
  heightfield: wrappers::RawHeightfield,
}

impl Heightfield {
  pub fn new(
    context: &mut Context,
    min_bounds: Vec3,
    max_bounds: Vec3,
    cell_horizontal_size: f32,
    cell_height: f32,
  ) -> Result<Self, ()> {
    let mut grid_size_x = 0;
    let mut grid_size_y = 0;
    // SAFETY: rcCalcGridSize only modifies the grid_size_* variables which
    // are mutable. Bounds only read as 3 floats which matches Vec3.
    unsafe {
      rcCalcGridSize(
        &min_bounds.x,
        &max_bounds.x,
        cell_horizontal_size,
        &mut grid_size_x,
        &mut grid_size_y,
      )
    };

    let mut heightfield = wrappers::RawHeightfield::new()?;

    // SAFETY: rcCreateHeightfield only modifies memory it owns, or
    // `context.context` or `heightfield`, which are mutably borrowed. Bounds
    // only read as 3 floats which matches Vec3.
    let heightfield_created = unsafe {
      rcCreateHeightfield(
        context.context.deref_mut(),
        heightfield.deref_mut(),
        grid_size_x,
        grid_size_y,
        &min_bounds.x,
        &max_bounds.x,
        cell_horizontal_size,
        cell_height,
      )
    };

    if heightfield_created {
      Ok(Self { heightfield })
    } else {
      Err(())
    }
  }

  pub fn rasterize_triangles(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3],
    area_ids: &[u8],
    flag_merge_threshold: i32,
  ) -> Result<(), ()> {
    assert_eq!(
      vertices.len() % 3,
      0,
      "Vertices must come in triangles (groups of 3). Vertex count: {}",
      vertices.len()
    );

    let num_triangles = vertices.len() / 3;

    assert_eq!(
      num_triangles,
      area_ids.len(),
      "area_ids should have one entry per triangle."
    );

    // SAFETY: rcRasterizeTriangles2 only mutates `context.context` and
    // `self.heightfield` which are both passed by exclusive borrows. It also
    // only reads from `num_triangles` * 3 vertices and `num_triangles`
    // area_ids.
    let rasterized_triangles = unsafe {
      rcRasterizeTriangles2(
        context.context.deref_mut(),
        &vertices[0].x,
        area_ids.as_ptr(),
        vertices.len() as i32,
        self.heightfield.deref_mut(),
        flag_merge_threshold,
      )
    };

    if rasterized_triangles {
      Ok(())
    } else {
      Err(())
    }
  }

  // TODO: Add support for indexed triangles.

  pub fn create_compact_heightfield(
    mut self,
    context: &mut Context,
    walkable_height: i32,
    walkable_climb: i32,
  ) -> Result<CompactHeightfield, ()> {
    let mut compact_heightfield = wrappers::RawCompactHeightfield::new()?;

    // SAFETY: rcBuildCompactHeightfield only mutates `context.context`,
    // `heightfield.heightfield`, and `compact_heightfield` which are mutably
    // borrowed.
    let built_compact_heightfield = unsafe {
      rcBuildCompactHeightfield(
        context.context.deref_mut(),
        walkable_height,
        walkable_climb,
        self.heightfield.deref_mut(),
        compact_heightfield.deref_mut(),
      )
    };

    if built_compact_heightfield {
      Ok(CompactHeightfield { compact_heightfield })
    } else {
      Err(())
    }
  }
}

pub struct CompactHeightfield {
  compact_heightfield: wrappers::RawCompactHeightfield,
}

impl CompactHeightfield {
  pub fn erode_walkable_area(
    &mut self,
    context: &mut Context,
    radius: i32,
  ) -> Result<(), ()> {
    // SAFETY: rcErodeWalkableArea only mutates `context.context`, or
    // `self.compact_heightfield`.
    let eroded_area = unsafe {
      rcErodeWalkableArea(
        context.context.deref_mut(),
        radius,
        self.compact_heightfield.deref_mut(),
      )
    };

    if eroded_area {
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn build_heightfield_layer_set(
    &self,
    context: &mut Context,
    border_size: i32,
    walkable_height: i32,
  ) -> Result<HeightfieldLayerSet, ()> {
    let mut layer_set = wrappers::RawHeightfieldLayerSet::new()?;

    // SAFETY: rcBuildHeightfieldLayers only mutates `context.context` and
    // `layer_set`. It also only reads from `self.compact_heightfield`.
    let build_succeeded = unsafe {
      rcBuildHeightfieldLayers(
        context.context.deref_mut(),
        // TODO: Remove this gnarly cast once the compact_heightfield is passed
        // by const ref.
        // https://github.com/recastnavigation/recastnavigation/pull/625
        self.compact_heightfield.deref()
          as *const recastnavigation_sys::rcCompactHeightfield
          as *mut recastnavigation_sys::rcCompactHeightfield,
        border_size,
        walkable_height,
        layer_set.deref_mut(),
      )
    };

    if build_succeeded {
      Ok(HeightfieldLayerSet { layer_set })
    } else {
      Err(())
    }
  }

  fn build_distance_field(&mut self, context: &mut Context) -> Result<(), ()> {
    // SAFETY: rcBuildDistanceField only mutates `context.context`, or
    // `self.compact_heightfield`.
    let distance_field_built = unsafe {
      rcBuildDistanceField(
        context.context.deref_mut(),
        self.compact_heightfield.deref_mut(),
      )
    };

    if distance_field_built {
      Ok(())
    } else {
      Err(())
    }
  }

  pub fn build_regions(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
    merge_region_area: i32,
  ) -> Result<CompactHeightfieldWithRegions, ()> {
    self.build_distance_field(context)?;

    // SAFETY: rcBuildRegions only mutates `context.context`, or
    // `self.compact_heightfield`.
    let regions_built = unsafe {
      rcBuildRegions(
        context.context.deref_mut(),
        self.compact_heightfield.deref_mut(),
        border_size,
        min_region_area,
        merge_region_area,
      )
    };

    if regions_built {
      Ok(CompactHeightfieldWithRegions {
        compact_heightfield: self.compact_heightfield,
      })
    } else {
      Err(())
    }
  }

  pub fn build_layer_regions(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
  ) -> Result<CompactHeightfieldWithRegions, ()> {
    self.build_distance_field(context)?;

    // SAFETY: rcBuildLayerRegions only mutates `context.context`, or
    // `self.compact_heightfield`.
    let regions_built = unsafe {
      rcBuildLayerRegions(
        context.context.deref_mut(),
        self.compact_heightfield.deref_mut(),
        border_size,
        min_region_area,
      )
    };

    if regions_built {
      Ok(CompactHeightfieldWithRegions {
        compact_heightfield: self.compact_heightfield,
      })
    } else {
      Err(())
    }
  }

  pub fn build_regions_monotone(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
    merge_region_area: i32,
  ) -> Result<CompactHeightfieldWithRegions, ()> {
    self.build_distance_field(context)?;

    // SAFETY: rcBuildRegionsMonotone only mutates `context.context`, or
    // `self.compact_heightfield`.
    let regions_built = unsafe {
      rcBuildRegionsMonotone(
        context.context.deref_mut(),
        self.compact_heightfield.deref_mut(),
        border_size,
        min_region_area,
        merge_region_area,
      )
    };

    if regions_built {
      Ok(CompactHeightfieldWithRegions {
        compact_heightfield: self.compact_heightfield,
      })
    } else {
      Err(())
    }
  }
}

pub struct CompactHeightfieldWithRegions {
  compact_heightfield: wrappers::RawCompactHeightfield,
}

impl CompactHeightfieldWithRegions {
  pub fn build_contours(
    self,
    context: &mut Context,
    max_error: f32,
    max_edge_len: i32,
    build_flags: ContourBuildFlags,
  ) -> Result<ContourSet, ()> {
    let mut contour_set = wrappers::RawContourSet::new()?;

    let build_flags = (build_flags.tessellate_wall_edges as i32 * 0x01)
      | (build_flags.tessellate_area_edges as i32 * 0x02);

    let build_succeeded = unsafe {
      rcBuildContours(
        context.context.deref_mut(),
        // TODO: Remove this gnarly cast once the compact_heightfield is passed
        // by const ref.
        // https://github.com/recastnavigation/recastnavigation/pull/625
        self.compact_heightfield.deref()
          as *const recastnavigation_sys::rcCompactHeightfield
          as *mut recastnavigation_sys::rcCompactHeightfield,
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

pub struct HeightfieldLayerSet {
  layer_set: wrappers::RawHeightfieldLayerSet,
}

impl HeightfieldLayerSet {
  pub fn len(&self) -> usize {
    self.layer_set.nlayers as usize
  }

  pub fn get_layer(&self, index: usize) -> HeightfieldLayer<'_> {
    // SAFETY: `layers` is owned by `self` and the lifetime of the slice is
    // equal to the lifetime of `self`. `layers` has a length of `self.len()` as
    // per `rcBuildHeightfieldLayers`.
    let slice =
      unsafe { std::slice::from_raw_parts(self.layer_set.layers, self.len()) };
    HeightfieldLayer { layer: &slice[index] }
  }

  pub fn as_vec(&self) -> Vec<HeightfieldLayer<'_>> {
    // SAFETY: `layers` is owned by `self` and the lifetime of the slice is
    // equal to the lifetime of `self`. `layers` has a length of `self.len()` as
    // per `rcBuildHeightfieldLayers`.
    let slice =
      unsafe { std::slice::from_raw_parts(self.layer_set.layers, self.len()) };
    (0..self.len()).map(|i| HeightfieldLayer { layer: &slice[i] }).collect()
  }
}

pub struct HeightfieldLayer<'layer_set> {
  layer: &'layer_set recastnavigation_sys::rcHeightfieldLayer,
}

impl<'layer_set> HeightfieldLayer<'layer_set> {
  pub fn min_bounds(&self) -> Vec3 {
    Vec3::new(self.layer.bmin[0], self.layer.bmin[1], self.layer.bmin[2])
  }

  pub fn max_bounds(&self) -> Vec3 {
    Vec3::new(self.layer.bmax[0], self.layer.bmax[1], self.layer.bmax[2])
  }

  pub fn cell_horizontal_size(&self) -> f32 {
    self.layer.cs
  }

  pub fn cell_height(&self) -> f32 {
    self.layer.ch
  }

  pub fn grid_width(&self) -> i32 {
    self.layer.width
  }

  pub fn grid_height(&self) -> i32 {
    self.layer.height
  }

  pub fn grid_min_bounds(&self) -> IVec3 {
    IVec3::new(self.layer.minx, self.layer.hmin, self.layer.miny)
  }

  pub fn grid_max_bounds(&self) -> IVec3 {
    IVec3::new(self.layer.maxx, self.layer.hmax, self.layer.maxy)
  }

  pub fn heights(&self) -> &[u8] {
    // SAFETY: `layer` is valid and therefore `layer.heights` holds width *
    // height entries.
    unsafe {
      std::slice::from_raw_parts(
        self.layer.heights,
        (self.layer.width * self.layer.height) as usize,
      )
    }
  }

  pub fn areas(&self) -> &[u8] {
    // SAFETY: `layer` is valid and therefore `layer.areas` holds width * height
    // entries.
    unsafe {
      std::slice::from_raw_parts(
        self.layer.areas,
        (self.layer.width * self.layer.height) as usize,
      )
    }
  }

  pub fn packed_connection_info(&self) -> &[u8] {
    // SAFETY: `layer` is valid and therefore `layer.cons` holds width * height
    // entries.
    unsafe {
      std::slice::from_raw_parts(
        self.layer.cons,
        (self.layer.width * self.layer.height) as usize,
      )
    }
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

#[cfg(test)]
mod tests {
  use crate::{
    vector::IVec3, CompactHeightfield, CompactHeightfieldWithRegions, Context,
    ContourBuildFlags, Heightfield, Vec3, WALKABLE_AREA_ID,
  };

  #[test]
  fn heightfield_rasterizes_triangles() {
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
  }

  #[test]
  fn compact_heightfield_erodes_area() {
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

    let mut compact_heightfield = heightfield
      .create_compact_heightfield(&mut context, 3, 0)
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");
  }

  fn compact_heightfield_builds_regions_base(
    build_fn: fn(
      compact_heightfield: CompactHeightfield,
      context: &mut Context,
    ) -> Result<CompactHeightfieldWithRegions, ()>,
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

    let mut compact_heightfield = heightfield
      .create_compact_heightfield(&mut context, 3, 0)
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");

    build_fn(compact_heightfield, &mut context)
      .expect("building regions succeeds");
  }

  #[test]
  fn compact_heightfield_builds_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield,
      context: &mut Context,
    ) -> Result<CompactHeightfieldWithRegions, ()> {
      compact_heightfield.build_regions(context, 1, 1, 1)
    }

    compact_heightfield_builds_regions_base(build_fn);
  }

  #[test]
  fn compact_heightfield_builds_layer_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield,
      context: &mut Context,
    ) -> Result<CompactHeightfieldWithRegions, ()> {
      compact_heightfield.build_layer_regions(context, 1, 1)
    }

    compact_heightfield_builds_regions_base(build_fn);
  }

  #[test]
  fn compact_heightfield_builds_monotone_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield,
      context: &mut Context,
    ) -> Result<CompactHeightfieldWithRegions, ()> {
      compact_heightfield.build_regions_monotone(context, 1, 1, 1)
    }

    compact_heightfield_builds_regions_base(build_fn);
  }

  #[test]
  fn compact_heightfield_builds_heightfield_layers() {
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

    let mut compact_heightfield = heightfield
      .create_compact_heightfield(&mut context, 3, 0)
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");

    let layer_set = compact_heightfield
      .build_heightfield_layer_set(&mut context, 0, 3)
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
    // TODO: Figure out why this is shrunk by 2 on both sides instead of 1.
    assert_eq!(layer.grid_min_bounds(), IVec3::new(2, 1, 2));
    assert_eq!(layer.grid_max_bounds(), IVec3::new(3, 1, 3));
  }

  #[test]
  fn compact_heightfield_builds_contour_set() {
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

    let mut compact_heightfield = heightfield
      .create_compact_heightfield(&mut context, 3, 0)
      .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");

    let compact_heightfield_with_regions = compact_heightfield
      .build_regions(&mut context, 1, 1, 1)
      .expect("regions built");

    compact_heightfield_with_regions
      .build_contours(
        &mut context,
        1.0,
        10,
        ContourBuildFlags {
          tessellate_wall_edges: true,
          tessellate_area_edges: false,
        },
      )
      .expect("contours built");
  }
}
