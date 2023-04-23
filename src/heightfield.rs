use std::ops::DerefMut;

use recastnavigation_sys::{
  rcCalcGridSize, rcCreateHeightfield, rcRasterizeTriangles2,
};

use crate::{wrappers, Context, Vec3};

pub struct Heightfield {
  pub(crate) heightfield: wrappers::RawHeightfield,
}

impl Heightfield {
  pub fn new(
    context: &mut Context,
    min_bounds: Vec3<f32>,
    max_bounds: Vec3<f32>,
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

  pub fn grid_width(&self) -> i32 {
    self.heightfield.width
  }

  pub fn grid_height(&self) -> i32 {
    self.heightfield.height
  }

  pub fn min_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.heightfield.bmin[0],
      self.heightfield.bmin[1],
      self.heightfield.bmin[2],
    )
  }

  pub fn max_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.heightfield.bmax[0],
      self.heightfield.bmax[1],
      self.heightfield.bmax[2],
    )
  }

  pub fn cell_horizontal_size(&self) -> f32 {
    self.heightfield.cs
  }

  pub fn cell_height(&self) -> f32 {
    self.heightfield.ch
  }

  pub fn spans_len(&self) -> usize {
    (self.grid_width() * self.grid_height()) as usize
  }

  pub fn spans_iter(
    &self,
  ) -> impl Iterator<Item = Option<HeightfieldSpan>> + '_ {
    // SAFETY: `self.heightfield.spans` is guaranteed to have exactly width *
    // height entries, and pointer alignment is guaranteed by heightfield
    // creation.
    let raw_spans = unsafe {
      std::slice::from_raw_parts(self.heightfield.spans, self.spans_len())
    };

    raw_spans.iter().map(|raw_ptr| {
      // SAFETY: The span is properly aligned and allocated by creating/mutating
      // the heightfield successfully.
      unsafe { raw_ptr.as_mut() }
        .map(|span| HeightfieldSpan { heightfield: self, span })
    })
  }

  pub fn span(&self, index: usize) -> Option<HeightfieldSpan> {
    // SAFETY: `self.heightfield.spans` is guaranteed to have exactly width *
    // height entries, and pointer alignment is guaranteed by heightfield
    // creation.
    let raw_spans = unsafe {
      std::slice::from_raw_parts(self.heightfield.spans, self.spans_len())
    };

    // SAFETY: The span is properly aligned and allocated by creating/mutating
    // the heightfield successfully.
    unsafe { raw_spans[index].as_mut() }
      .map(|span| HeightfieldSpan { heightfield: self, span })
  }

  pub fn span_by_grid(
    &self,
    grid_x: i32,
    grid_y: i32,
  ) -> Option<HeightfieldSpan> {
    assert!(0 <= grid_x && grid_x < self.grid_width());
    assert!(0 <= grid_y && grid_y < self.grid_height());
    self.span((grid_x + grid_y * self.grid_width()) as usize)
  }

  pub fn rasterize_triangles(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
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
        num_triangles as i32,
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

#[derive(Clone, Copy)]
pub struct HeightfieldSpan<'heightfield> {
  heightfield: &'heightfield Heightfield,
  span: &'heightfield recastnavigation_sys::rcSpan,
}

impl<'heightfield> HeightfieldSpan<'heightfield> {
  pub fn height_min_u32(&self) -> u32 {
    self.span.smin()
  }

  pub fn height_max_u32(&self) -> u32 {
    self.span.smax()
  }

  pub fn height_min_f32(&self) -> f32 {
    self.height_min_u32() as f32 * self.heightfield.cell_height()
      + self.heightfield.min_bounds().y
  }

  pub fn height_max_f32(&self) -> f32 {
    self.height_max_u32() as f32 * self.heightfield.cell_height()
      + self.heightfield.min_bounds().y
  }

  pub fn area_id(&self) -> u32 {
    self.span.area()
  }

  pub fn next_span_in_column(&self) -> Option<Self> {
    // SAFETY: The span is properly aligned and allocated by creating/mutating
    // the heightfield successfully.
    unsafe { self.span.next.as_mut() }
      .map(|span| HeightfieldSpan { heightfield: self.heightfield, span })
  }

  pub fn collect(mut head: Option<Self>) -> Vec<Self> {
    let mut vec = Vec::new();
    while let Some(span) = head {
      vec.push(span);

      head = span.next_span_in_column();
    }
    vec
  }
}

impl<'hf> std::fmt::Debug for HeightfieldSpan<'hf> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HeightfieldSpan")
      .field("smin", &self.height_min_u32())
      .field("smax", &self.height_max_u32())
      .field("area", &self.area_id())
      .finish()
  }
}
