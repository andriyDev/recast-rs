use std::ops::DerefMut;

use recastnavigation_sys::{
  rcCalcGridSize, rcCreateHeightfield, rcRasterizeTriangles2,
};

mod vector;
mod wrappers;

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

    let mut heightfield = wrappers::RawHeightfield::new()
      .expect("allocation of RawHeightfield should succeed");

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
}

#[cfg(test)]
mod tests {
  use crate::{Context, Heightfield, Vec3, WALKABLE_AREA_ID};

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
}
