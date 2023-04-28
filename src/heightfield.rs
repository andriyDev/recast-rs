use std::ops::DerefMut;

use recastnavigation_sys::{
  rcCalcGridSize, rcCreateHeightfield, rcFilterLedgeSpans,
  rcFilterLowHangingWalkableObstacles, rcFilterWalkableLowHeightSpans,
  rcRasterizeTriangles, rcRasterizeTriangles1, rcRasterizeTriangles2,
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

  // SAFETY: All indices in `triangles` must be in the range of `vertices`
  // indices.
  pub unsafe fn rasterize_indexed_triangles_u16_unchecked(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
    triangles: &[Vec3<u16>],
    area_ids: &[u8],
    flag_merge_threshold: i32,
  ) -> Result<(), ()> {
    assert_eq!(
      triangles.len(),
      area_ids.len(),
      "area_ids should have one entry per triangle."
    );

    // SAFETY: rcRasterizeTriangles1 only mutates `context.context` and
    // `self.heightfield` which are both passed by exclusive borrows. `vertices`
    // and `triangles` are only read, and only in the valid range of elements
    // (due to the function safety guarantee).
    let rasterized_triangles = unsafe {
      rcRasterizeTriangles1(
        context.context.deref_mut(),
        vertices.as_ptr() as *const f32,
        vertices.len() as i32,
        triangles.as_ptr() as *const u16,
        area_ids.as_ptr(),
        triangles.len() as i32,
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

  // SAFETY: All indices in `triangles` must be in the range of `vertices`
  // indices.
  pub unsafe fn rasterize_indexed_triangles_i32_unchecked(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
    triangles: &[Vec3<i32>],
    area_ids: &[u8],
    flag_merge_threshold: i32,
  ) -> Result<(), ()> {
    assert_eq!(
      triangles.len(),
      area_ids.len(),
      "area_ids should have one entry per triangle."
    );

    // SAFETY: rcRasterizeTriangles1 only mutates `context.context` and
    // `self.heightfield` which are both passed by exclusive borrows. `vertices`
    // and `triangles` are only read, and only in the valid range of elements
    // (due to the function safety guarantee).
    let rasterized_triangles = unsafe {
      rcRasterizeTriangles(
        context.context.deref_mut(),
        vertices.as_ptr() as *const f32,
        vertices.len() as i32,
        triangles.as_ptr() as *const i32,
        area_ids.as_ptr(),
        triangles.len() as i32,
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

  pub fn rasterize_indexed_triangles_u16(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
    triangles: &[Vec3<u16>],
    area_ids: &[u8],
    flag_merge_threshold: i32,
  ) -> Result<(), ()> {
    for triangle in triangles {
      assert!(
        triangle.x < vertices.len() as u16
          && triangle.y < vertices.len() as u16
          && triangle.z < vertices.len() as u16,
        "Triangle indexes out-of-bounds vertex. Triangle={:?}, vertices_len={}",
        *triangle,
        vertices.len()
      );
    }

    // SAFETY: We have checked that all indices in `triangles` are valid.
    // Therefore, the function is guaranteed to be safe.
    unsafe {
      self.rasterize_indexed_triangles_u16_unchecked(
        context,
        vertices,
        triangles,
        area_ids,
        flag_merge_threshold,
      )
    }
  }

  pub fn rasterize_indexed_triangles_i32(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
    triangles: &[Vec3<i32>],
    area_ids: &[u8],
    flag_merge_threshold: i32,
  ) -> Result<(), ()> {
    for triangle in triangles {
      assert!(
        0 <= triangle.x
          && triangle.x < vertices.len() as i32
          && 0 <= triangle.y
          && triangle.y < vertices.len() as i32
          && 0 <= triangle.z
          && triangle.z < vertices.len() as i32,
        "Triangle indexes out-of-bounds vertex. Triangle={:?}, vertices_len={}",
        *triangle,
        vertices.len()
      );
    }

    // SAFETY: We have checked that all indices in `triangles` are valid.
    // Therefore, the function is guaranteed to be safe.
    unsafe {
      self.rasterize_indexed_triangles_i32_unchecked(
        context,
        vertices,
        triangles,
        area_ids,
        flag_merge_threshold,
      )
    }
  }

  pub fn filter_low_hanging_walkable_obstacles(
    &mut self,
    context: &mut Context,
    walkable_climb: i32,
  ) {
    // SAFETY: Only `context.context` and `self.heightfield` are mutated, and
    // these are passed by exclusive borrows.
    unsafe {
      rcFilterLowHangingWalkableObstacles(
        context.context.deref_mut(),
        walkable_climb,
        self.heightfield.deref_mut(),
      )
    };
  }

  pub fn filter_ledge_spans(
    &mut self,
    context: &mut Context,
    walkable_height: i32,
    walkable_climb: i32,
  ) {
    // SAFETY: Only `context.context` and `self.heightfield` are mutated, and
    // these are passed by exclusive borrows.
    unsafe {
      rcFilterLedgeSpans(
        context.context.deref_mut(),
        walkable_height,
        walkable_climb,
        self.heightfield.deref_mut(),
      )
    };
  }

  pub fn filter_walkable_low_height_spans(
    &mut self,
    context: &mut Context,
    walkable_height: i32,
  ) {
    // SAFETY: Only `context.context` and `self.heightfield` are mutated, and
    // these are passed by exclusive borrows.
    unsafe {
      rcFilterWalkableLowHeightSpans(
        context.context.deref_mut(),
        walkable_height,
        self.heightfield.deref_mut(),
      )
    };
  }
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

#[cfg(test)]
mod tests {
  use std::panic::AssertUnwindSafe;

  use crate::{Context, Heightfield, HeightfieldSpan, Vec3, WALKABLE_AREA_ID};

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
  fn rasterize_indexed_triangles() {
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
      Vec3::new(0.0, 0.25, 5.0),
      Vec3::new(3.0, 3.25, 0.0),
      Vec3::new(5.0, 3.25, 0.0),
      Vec3::new(5.0, 3.25, 5.0),
      Vec3::new(3.0, 3.25, 5.0),
    ];

    let triangles = [
      Vec3::new(0, 1, 2),
      Vec3::new(2, 3, 0),
      Vec3::new(4, 5, 6),
      Vec3::new(6, 7, 4),
    ];

    let area_ids =
      [WALKABLE_AREA_ID, WALKABLE_AREA_ID, WALKABLE_AREA_ID, WALKABLE_AREA_ID];

    heightfield
      .rasterize_indexed_triangles_i32(
        &mut context,
        &vertices,
        &triangles,
        &area_ids,
        1,
      )
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

    for x in 0..6 as usize {
      for y in 0..heightfield.grid_height() as usize {
        assert_span_column_eq!(
          &columns[index_at(x, y)],
          [(0, 1, WALKABLE_AREA_ID as u32)]
        );
      }
    }

    for x in 6..heightfield.grid_width() as usize {
      for y in 0..heightfield.grid_height() as usize {
        assert_span_column_eq!(
          &columns[index_at(x, y)],
          [(0, 1, WALKABLE_AREA_ID as u32), (6, 7, WALKABLE_AREA_ID as u32)]
        );
      }
    }
  }

  #[test]
  fn checks_for_invalid_triangles_before_rasterizing_indexed_u16() {
    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 0.0),
    ];

    let triangles = [Vec3::new(0, 1, 2)];

    let triangle_area_ids = [WALKABLE_AREA_ID];

    let mut context = Context::new();
    let mut heightfield = Heightfield::new(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(5.0, 5.0, 5.0),
      1.0,
      1.0,
    )
    .expect("creating heightfield successful");

    heightfield
      .rasterize_indexed_triangles_u16(
        &mut context,
        &vertices,
        &triangles,
        &triangle_area_ids,
        1,
      )
      .expect("rasterizes triangles");

    let invalid_triangles =
      [Vec3::new(3, 1, 2), Vec3::new(0, 3, 2), Vec3::new(0, 1, 3)];

    for invalid_triangle_slice in invalid_triangles.chunks(1) {
      let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = heightfield.rasterize_indexed_triangles_u16(
          &mut context,
          &vertices,
          invalid_triangle_slice,
          &triangle_area_ids,
          1,
        );
      }));

      assert!(
        result.is_err(),
        "Expected invalid triangle to break an assert, but succeeded. Triangle={:?}",
        invalid_triangle_slice[0]
      );
    }
  }

  #[test]
  fn checks_for_invalid_triangles_before_rasterizing_indexed_i32() {
    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 0.0),
    ];

    let triangles = [Vec3::new(0, 1, 2)];

    let triangle_area_ids = [WALKABLE_AREA_ID];

    let mut context = Context::new();
    let mut heightfield = Heightfield::new(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(5.0, 5.0, 5.0),
      1.0,
      1.0,
    )
    .expect("creating heightfield successful");

    heightfield
      .rasterize_indexed_triangles_i32(
        &mut context,
        &vertices,
        &triangles,
        &triangle_area_ids,
        1,
      )
      .expect("rasterizes triangles");

    let invalid_triangles = [
      Vec3::new(-1, 1, 2),
      Vec3::new(3, 1, 2),
      Vec3::new(0, -2, 2),
      Vec3::new(0, 3, 2),
      Vec3::new(0, 1, -3),
      Vec3::new(0, 1, 3),
    ];

    for invalid_triangle_slice in invalid_triangles.chunks(1) {
      let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = heightfield.rasterize_indexed_triangles_i32(
          &mut context,
          &vertices,
          invalid_triangle_slice,
          &triangle_area_ids,
          1,
        );
      }));

      assert!(
        result.is_err(),
        "Expected invalid triangle to break an assert, but succeeded. Triangle={:?}",
        invalid_triangle_slice[0]
      );
    }
  }

  #[test]
  fn filters_low_hanging_obstacles() {
    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(0.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 0.0),
      // An unwalkable ledge near the ground.
      Vec3::new(3.0, 2.5, 0.0),
      Vec3::new(3.0, 2.5, 5.0),
      Vec3::new(5.0, 2.5, 5.0),
      Vec3::new(5.0, 2.5, 0.0),
      // An unwalkable ledge high above the ground.
      Vec3::new(0.0, 3.5, 0.0),
      Vec3::new(0.0, 3.5, 5.0),
      Vec3::new(2.0, 3.5, 5.0),
      Vec3::new(2.0, 3.5, 0.0),
    ];

    let triangles = [
      Vec3::new(0, 1, 2),
      Vec3::new(2, 3, 0),
      Vec3::new(4, 5, 6),
      Vec3::new(6, 7, 4),
      Vec3::new(8, 9, 10),
      Vec3::new(10, 11, 8),
    ];

    const W: u8 = WALKABLE_AREA_ID;
    let triangle_area_ids = [W, W, 0, 0, 0, 0];

    let mut context = Context::new();
    let mut heightfield = Heightfield::new(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(5.0, 10.0, 5.0),
      /* cell_horizontal_size= */ 1.0,
      /* cell_height= */ 1.0,
    )
    .expect("creates heightfield successfully");

    // SAFETY: Triangle indices are all in range.
    unsafe {
      heightfield.rasterize_indexed_triangles_i32_unchecked(
        &mut context,
        &vertices,
        &triangles,
        &triangle_area_ids,
        /* flag_merge_threshold= */ 1,
      )
    }
    .expect("rasterization succeeds");

    // No filtering so far, so ledge is not walkable.
    assert_eq!(
      heightfield
        .span_by_grid(4, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      0
    );

    heightfield.filter_low_hanging_walkable_obstacles(
      &mut context,
      /* walkable_climb= */ 2,
    );

    // Low ledge is now walkable.
    assert_eq!(
      heightfield
        .span_by_grid(4, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      WALKABLE_AREA_ID as _
    );

    // High ledge is still not walkable.
    assert_eq!(
      heightfield
        .span_by_grid(1, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      0
    );
  }

  // TODO: Fix this test.
  #[test]
  fn filters_ledges() {
    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(0.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 0.0),
      // Climable ledge.
      Vec3::new(3.0, 2.5, 0.0),
      Vec3::new(3.0, 2.5, 5.0),
      Vec3::new(5.0, 2.5, 5.0),
      Vec3::new(5.0, 2.5, 0.0),
      // Unclimbable ledge.
      Vec3::new(0.0, 7.5, 0.0),
      Vec3::new(0.0, 7.5, 5.0),
      Vec3::new(2.0, 7.5, 5.0),
      Vec3::new(2.0, 7.5, 0.0),
    ];

    let triangles = [
      Vec3::new(0, 1, 2),
      Vec3::new(2, 3, 0),
      Vec3::new(4, 5, 6),
      Vec3::new(6, 7, 4),
      Vec3::new(8, 9, 10),
      Vec3::new(10, 11, 8),
    ];

    const W: u8 = WALKABLE_AREA_ID;
    let triangle_area_ids = [W, W, W, W, W, W];

    let mut context = Context::new();
    let mut heightfield = Heightfield::new(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(5.0, 10.0, 5.0),
      /* cell_horizontal_size= */ 1.0,
      /* cell_height= */ 1.0,
    )
    .expect("creates heightfield successfully");

    // SAFETY: Triangle indices are all in range.
    unsafe {
      heightfield.rasterize_indexed_triangles_i32_unchecked(
        &mut context,
        &vertices,
        &triangles,
        &triangle_area_ids,
        /* flag_merge_threshold= */ 1,
      )
    }
    .expect("rasterization succeeds");

    // No filtering so far, so unclimbable ledge is still walkable.
    assert_eq!(
      heightfield
        .span_by_grid(1, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      WALKABLE_AREA_ID as _
    );

    heightfield.filter_ledge_spans(
      &mut context,
      /* walkable_height= */ 3,
      /* walkable_climb= */ 3,
    );

    // Unclimbable ledge is no longer walkable.
    assert_eq!(
      heightfield
        .span_by_grid(1, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      0
    );

    // Climbable ledge is still walkable
    assert_eq!(
      heightfield
        .span_by_grid(3, 2)
        .expect("floor rasterized")
        .next_span_in_column()
        .expect("ledge rasterized")
        .area_id(),
      WALKABLE_AREA_ID as _
    );
  }

  #[test]
  fn filters_low_height_spans() {
    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(0.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 5.0),
      Vec3::new(5.0, 0.5, 0.0),
      // Low ceiling.
      Vec3::new(3.0, 3.5, 0.0),
      Vec3::new(3.0, 3.5, 5.0),
      Vec3::new(5.0, 3.5, 5.0),
      Vec3::new(5.0, 3.5, 0.0),
      // Just high enough ceiling.
      Vec3::new(1.0, 5.5, 0.0),
      Vec3::new(1.0, 5.5, 5.0),
      Vec3::new(3.0, 5.5, 5.0),
      Vec3::new(3.0, 5.5, 0.0),
    ];

    let triangles = [
      Vec3::new(0, 1, 2),
      Vec3::new(2, 3, 0),
      Vec3::new(4, 5, 6),
      Vec3::new(6, 7, 4),
      Vec3::new(8, 9, 10),
      Vec3::new(10, 11, 8),
    ];

    const W: u8 = WALKABLE_AREA_ID;
    let triangle_area_ids = [W, W, W, W, W, W];

    let mut context = Context::new();
    let mut heightfield = Heightfield::new(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(5.0, 10.0, 5.0),
      /* cell_horizontal_size= */ 1.0,
      /* cell_height= */ 1.0,
    )
    .expect("creates heightfield successfully");

    // SAFETY: Triangle indices are all in range.
    unsafe {
      heightfield.rasterize_indexed_triangles_i32_unchecked(
        &mut context,
        &vertices,
        &triangles,
        &triangle_area_ids,
        /* flag_merge_threshold= */ 1,
      )
    }
    .expect("rasterization succeeds");

    // No filtering so far, so low ceiling height is ok.
    assert_eq!(
      heightfield
        .span_by_grid(4, 2)
        .expect("span should be present since rasterization occured")
        .area_id(),
      WALKABLE_AREA_ID as _
    );

    heightfield.filter_walkable_low_height_spans(
      &mut context,
      /* walkable_height= */ 3,
    );

    // Low ceiling height has marked this span as unwalkable.
    assert_eq!(
      heightfield
        .span_by_grid(4, 2)
        .expect("span should be present since rasterization occured")
        .area_id(),
      0
    );
    // The ceiling here is high enough to be walkable.
    assert_eq!(
      heightfield
        .span_by_grid(2, 2)
        .expect("span should be present since rasterization occured")
        .area_id(),
      WALKABLE_AREA_ID as _
    );
    // No ceiling here, so should still be walkable.
    assert_eq!(
      heightfield
        .span_by_grid(0, 2)
        .expect("span should be present since rasterization occured")
        .area_id(),
      WALKABLE_AREA_ID as _
    );
  }
}
