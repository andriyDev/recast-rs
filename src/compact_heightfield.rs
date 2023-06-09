use std::ops::{Deref, DerefMut, Range};

use recastnavigation_sys::{
  rcBuildCompactHeightfield, rcBuildDistanceField, rcBuildLayerRegions,
  rcBuildRegions, rcBuildRegionsMonotone, rcCompactSpan, rcErodeWalkableArea,
  rcMarkBoxArea, rcMarkConvexPolyArea, rcMarkCylinderArea,
  rcMedianFilterWalkableArea,
};

use crate::{wrappers, Context, Heightfield, Vec3};

// A Recast CompactHeightfield. This is generally created from a Heightfield and
// represents the non-solid areas of the world.
pub struct CompactHeightfield<TypeState: CompactHeightfieldState> {
  pub(crate) compact_heightfield: wrappers::RawCompactHeightfield,
  marker: std::marker::PhantomData<TypeState>,
}

// Type state for CompactHeightfield representing when a CompactHeightfield has
// no region data. In this state, CompactHeightfields are still mutable.
pub enum NoRegions {}
// Type state for CompactHeightfield representing when a CompactHeightfield has
// region data. In this state, CompactHeightfields are generally immutable and
// can only be used to generate other objects.
pub enum HasRegions {}

// Type state trait for CompactHeightfield.
pub trait CompactHeightfieldState {}
impl CompactHeightfieldState for NoRegions {}
impl CompactHeightfieldState for HasRegions {}

impl<TypeState: CompactHeightfieldState> CompactHeightfield<TypeState> {
  pub fn grid_width(&self) -> i32 {
    self.compact_heightfield.width
  }

  pub fn grid_height(&self) -> i32 {
    self.compact_heightfield.height
  }

  pub fn walkable_height(&self) -> i32 {
    self.compact_heightfield.walkableHeight
  }

  pub fn walkable_climb(&self) -> i32 {
    self.compact_heightfield.walkableClimb
  }

  pub fn min_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.compact_heightfield.bmin[0],
      self.compact_heightfield.bmin[1],
      self.compact_heightfield.bmin[2],
    )
  }

  pub fn max_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.compact_heightfield.bmax[0],
      self.compact_heightfield.bmax[1],
      self.compact_heightfield.bmax[2],
    )
  }

  pub fn cell_horizontal_size(&self) -> f32 {
    self.compact_heightfield.cs
  }

  pub fn cell_height(&self) -> f32 {
    self.compact_heightfield.ch
  }

  // Creates an Iterator of the cells. See `cell` for more on cells.
  pub fn cells_iter(&self) -> impl Iterator<Item = Range<usize>> + '_ {
    // SAFETY: `cells` is guaranteed to have `width` * `heights` cells, and be
    // well aligned. `cells` is also owned by `self` so the lifetime of the
    // iterator is tied to `cells`.
    let raw_cells = unsafe {
      std::slice::from_raw_parts(
        self.compact_heightfield.cells,
        self.grid_width() as usize * self.grid_height() as usize,
      )
    };

    raw_cells.iter().map(|cell| {
      (cell.index() as usize)..(cell.index() + cell.count()) as usize
    })
  }

  // Returns the range of spans for each cell. A cell is a column of spans. Each
  // grid cell contains a column (so there are `grid_width * grid_height`
  // cells), and cells are ordered by x followed by z.
  pub fn cell(&self, index: usize) -> Range<usize> {
    // SAFETY: `cells` is guaranteed to have `width` * `heights` cells, and be
    // well aligned.
    let raw_cells = unsafe {
      std::slice::from_raw_parts(
        self.compact_heightfield.cells,
        self.grid_width() as usize * self.grid_height() as usize,
      )
    };

    let cell = &raw_cells[index];
    (cell.index() as usize)..(cell.index() + cell.count()) as usize
  }

  pub fn spans_len(&self) -> usize {
    self.compact_heightfield.spanCount as usize
  }

  // Creates an Iterator of all spans.
  pub fn spans_iter(
    &self,
  ) -> impl Iterator<Item = CompactSpan<'_, TypeState>> + '_ {
    // SAFETY: `spans` is guaranteed to have `spanCount` cells, and be well
    // aligned.
    let raw_spans = unsafe {
      std::slice::from_raw_parts(
        self.compact_heightfield.spans,
        self.spans_len(),
      )
    };

    raw_spans.iter().map(|span| CompactSpan { compact_heightfield: self, span })
  }

  // Returns a slice of the area IDs of each span.
  pub fn span_areas(&self) -> &[u8] {
    // SAFETY: `areas` is guaranteed to have `spanCount` elements, and be well
    // aligned.
    unsafe {
      std::slice::from_raw_parts(
        self.compact_heightfield.areas,
        self.spans_len(),
      )
    }
  }
}

impl CompactHeightfield<NoRegions> {
  // Creates a CompactHeightfield from a `heightfield`. `walkable_height` and
  // `walkable_climb` are used to determine the areas where spans should be
  // generated.
  pub fn new(
    heightfield: &Heightfield,
    context: &mut Context,
    walkable_height: i32,
    walkable_climb: i32,
  ) -> Result<Self, ()> {
    let mut compact_heightfield = wrappers::RawCompactHeightfield::new()?;

    // SAFETY: rcBuildCompactHeightfield only mutates `context.context`,
    // `compact_heightfield`, which are mutably borrowed.
    let built_compact_heightfield = unsafe {
      rcBuildCompactHeightfield(
        context.context.deref_mut(),
        walkable_height,
        walkable_climb,
        heightfield.heightfield.deref(),
        compact_heightfield.deref_mut(),
      )
    };

    if built_compact_heightfield {
      Ok(Self { compact_heightfield, marker: std::marker::PhantomData })
    } else {
      Err(())
    }
  }

  // Erodes the walkable area by `radius` grid cells.
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

  // Marks all spans in the specified box with the area ID of `new_id`.
  pub fn mark_box_area_with_id(
    &mut self,
    context: &mut Context,
    min_bounds: Vec3<f32>,
    max_bounds: Vec3<f32>,
    new_id: u8,
  ) {
    // SAFETY: `rcMarkBoxArea` only mutates `context.context` and
    // `self.compact_heightfield`, which are exclusively borrowed. `min_bounds`
    // and `max_bounds` are only read.
    unsafe {
      rcMarkBoxArea(
        context.context.deref_mut(),
        &min_bounds.x,
        &max_bounds.x,
        new_id,
        self.compact_heightfield.deref_mut(),
      )
    };
  }

  // Marks all spans in the specified cylinder with the area ID of `new_id`.
  pub fn mark_cylinder_area_with_id(
    &mut self,
    context: &mut Context,
    position: Vec3<f32>,
    radius: f32,
    height: f32,
    new_id: u8,
  ) {
    // SAFETY: `rcMarkCylinderArea` only mutates `context.context` and
    // `self.compact_heightfield`, which are exclusively borrowed. `position`
    // are only read.
    unsafe {
      rcMarkCylinderArea(
        context.context.deref_mut(),
        &position.x,
        radius,
        height,
        new_id,
        self.compact_heightfield.deref_mut(),
      )
    };
  }

  // Marks all spans in the convex polygon defined by `vertices` with the area
  // ID of `new_id`. The convex polygon is extruded vertically based on
  // `base_height` and `top_height`. Note the Y component of `vertices` is
  // ignored.
  pub fn mark_convex_poly_area_with_id(
    &mut self,
    context: &mut Context,
    vertices: &[Vec3<f32>],
    base_height: f32,
    top_height: f32,
    new_id: u8,
  ) {
    // SAFETY: `rcMarkCylinderArea` only mutates `context.context` and
    // `self.compact_heightfield`, which are exclusively borrowed. `vertices`
    // are only read.
    unsafe {
      rcMarkConvexPolyArea(
        context.context.deref_mut(),
        vertices.as_ptr() as *const f32,
        vertices.len() as i32,
        base_height,
        top_height,
        new_id,
        self.compact_heightfield.deref_mut(),
      )
    };
  }

  // Performs a median filter on the area IDs of spans. This acts like a "blur"
  // which can remove noise from small unwalkable obstacles (e.g. a pebble
  // marked as unwalkable).
  pub fn median_filter_area_ids(
    &mut self,
    context: &mut Context,
  ) -> Result<(), ()> {
    // SAFETY: rcMedianFilterWalkableArea only mutates `context.context` and
    // `self.compact_heightfield`, which are both mutably borrowed.
    let success = unsafe {
      rcMedianFilterWalkableArea(
        context.context.deref_mut(),
        self.compact_heightfield.deref_mut(),
      )
    };

    if success {
      Ok(())
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

  // Builds regions using watershed partitioning. This makes the
  // CompactHeightfield immutable (as modifying the underlying data could make
  // the region data inconsistent). `border_size` is the size of the
  // non-navigable border around the heightfield. `min_region_area` is the
  // minimum number of cells allowed to form an isolated island. Any regions
  // with fewer span counts than `merge_region_area` will prefer to be merged
  // into a larger region.
  pub fn build_regions(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
    merge_region_area: i32,
  ) -> Result<CompactHeightfield<HasRegions>, ()> {
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
      Ok(CompactHeightfield::<HasRegions> {
        compact_heightfield: self.compact_heightfield,
        marker: std::marker::PhantomData,
      })
    } else {
      Err(())
    }
  }

  // Builds regions by partitioning the heightfield in non-overlapping layers.
  // This makes the CompactHeightfield immutable (as modifying the underlying
  // data could make the region data inconsistent). `border_size` is the size
  // of the non-navigable border around the heightfield. `min_region_area` is
  // the minimum number of cells allowed to form an isolated island.
  pub fn build_layer_regions(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
  ) -> Result<CompactHeightfield<HasRegions>, ()> {
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
      Ok(CompactHeightfield::<HasRegions> {
        compact_heightfield: self.compact_heightfield,
        marker: std::marker::PhantomData,
      })
    } else {
      Err(())
    }
  }

  // Builds regions using simple monotone partitioning. This makes the
  // CompactHeightfield immutable (as modifying the underlying data could make
  // the region data inconsistent). `border_size` is the size of the
  // non-navigable border around the heightfield. `min_region_area` is the
  // minimum number of cells allowed to form an isolated island. Any regions
  // with fewer span counts than `merge_region_area` will prefer to be merged
  // into a larger region.
  pub fn build_regions_monotone(
    mut self,
    context: &mut Context,
    border_size: i32,
    min_region_area: i32,
    merge_region_area: i32,
  ) -> Result<CompactHeightfield<HasRegions>, ()> {
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
      Ok(CompactHeightfield::<HasRegions> {
        compact_heightfield: self.compact_heightfield,
        marker: std::marker::PhantomData,
      })
    } else {
      Err(())
    }
  }
}

impl CompactHeightfield<HasRegions> {
  pub fn border_size(&self) -> i32 {
    self.compact_heightfield.borderSize
  }

  pub fn max_region_id(&self) -> u16 {
    self.compact_heightfield.maxRegions
  }

  pub fn max_distance(&self) -> u16 {
    self.compact_heightfield.maxDistance
  }
}

// A single span in a CompactHeightfield. A span represents a vertical column of
// unobstructed space.
pub struct CompactSpan<'compact_heightfield, TypeState>
where
  TypeState: CompactHeightfieldState,
{
  compact_heightfield: &'compact_heightfield CompactHeightfield<TypeState>,
  span: &'compact_heightfield rcCompactSpan,
}

impl<'compact_heightfield, TypeState>
  CompactSpan<'compact_heightfield, TypeState>
where
  TypeState: CompactHeightfieldState,
{
  pub fn y_start_u16(&self) -> u16 {
    self.span.y
  }

  pub fn y_size_u32(&self) -> u32 {
    self.span.h()
  }

  pub fn y_start_f32(&self) -> f32 {
    self.y_start_u16() as f32 * self.compact_heightfield.cell_height()
      + self.compact_heightfield.min_bounds().y
  }

  pub fn y_size_f32(&self) -> f32 {
    self.y_size_u32() as f32 * self.compact_heightfield.cell_height()
  }

  pub fn y_end_u32(&self) -> u32 {
    self.y_start_u16() as u32 + self.y_size_u32()
  }

  pub fn y_end_f32(&self) -> f32 {
    self.y_start_f32() + self.y_size_f32()
  }

  // Returns how/whether a span is connected to neighbouring spans.
  pub fn connection(&self, direction: Direction) -> u32 {
    let shift = match direction {
      Direction::NegX => 0,
      Direction::PosZ => 1,
      Direction::PosX => 2,
      Direction::NegZ => 3,
    } * 6;

    self.span.con() >> shift & 0x3f
  }
}

impl<'compact_heightfield> CompactSpan<'compact_heightfield, HasRegions> {
  pub fn region_id(&self) -> u16 {
    self.span.reg
  }
}

impl<'compact_heightfield> std::fmt::Debug
  for CompactSpan<'compact_heightfield, NoRegions>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CompactSpan")
      .field("y_start", &self.y_start_u16())
      .field("y_size", &self.y_size_u32())
      .field(
        "connections",
        &[
          (Direction::NegX, self.connection(Direction::NegX)),
          (Direction::PosZ, self.connection(Direction::PosZ)),
          (Direction::PosX, self.connection(Direction::PosX)),
          (Direction::NegZ, self.connection(Direction::NegZ)),
        ],
      )
      .finish()
  }
}

impl<'compact_heightfield> std::fmt::Debug
  for CompactSpan<'compact_heightfield, HasRegions>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CompactSpan")
      .field("y_start", &self.y_start_u16())
      .field("y_size", &self.y_size_u32())
      .field(
        "connections",
        &[
          (Direction::NegX, self.connection(Direction::NegX)),
          (Direction::PosZ, self.connection(Direction::PosZ)),
          (Direction::PosX, self.connection(Direction::PosX)),
          (Direction::NegZ, self.connection(Direction::NegZ)),
        ],
      )
      .field("region_id", &self.region_id())
      .finish()
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Direction {
  NegX,
  PosZ,
  PosX,
  NegZ,
}

#[cfg(test)]
mod tests {
  use crate::{
    CompactHeightfield, Context, HasRegions, Heightfield, NoRegions, Vec3,
    WALKABLE_AREA_ID,
  };

  macro_rules! assert_span_column_eq {
    ($column: expr, $expected_column: expr) => {{
      let column = $column;
      let expected_column = $expected_column;

      assert_eq!(
        column.len(),
        expected_column.len(),
        "\n\nactual_spans={:?}\nexpected_spans={:?}",
        column,
        expected_column
      );

      for (index, (actual_span, expected_span)) in
        column.iter().zip(expected_column.iter()).enumerate()
      {
        assert_eq!(
          (
            actual_span.y_start_u16(),
            actual_span.y_size_u32(),
            actual_span.y_end_u32(),
          ),
          *expected_span,
          "\n\nColumn differs at index {}\n\n\
          actual_span={:?}\n\
          expected_span=CompactSpan {{ \
            y_start_u16: {}, \
            y_size_u16: {}, \
           }}",
          index,
          actual_span,
          expected_span.0,
          expected_span.1,
        );
      }
    }};
  }

  #[test]
  fn create_compact_heightfield() {
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

    let compact_heightfield = CompactHeightfield::<NoRegions>::new(
      &heightfield,
      &mut context,
      /* walkable_height= */ 3,
      /* walkable_climb= */ 0,
    )
    .expect("creating CompactHeightfield succeeds");

    assert_eq!(compact_heightfield.grid_width(), 5);
    assert_eq!(compact_heightfield.grid_height(), 5);
    assert_eq!(compact_heightfield.walkable_height(), 3);
    assert_eq!(compact_heightfield.walkable_climb(), 0);
    assert_eq!(compact_heightfield.min_bounds(), min_bounds);
    assert_eq!(
      compact_heightfield.max_bounds(),
      // The bounds are expanded to allow the top of the heightfield to be
      // walked on.
      Vec3::new(max_bounds.x, max_bounds.y + 3.0, max_bounds.z)
    );

    // CompactSpans only store the "free space" rather than the blocked space
    // like in HeightFieldSpans. The top-most span is always expanded to 256.
    let expected_column = [(1, 255, 256)];

    let spans = compact_heightfield.spans_iter().collect::<Vec<_>>();
    for column in
      compact_heightfield.cells_iter().map(|cell_range| &spans[cell_range])
    {
      assert_span_column_eq!(column, &expected_column);
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
      CompactHeightfield::<NoRegions>::new(&heightfield, &mut context, 3, 0)
        .expect("creating CompactHeightfield succeeds");

    const W: u8 = WALKABLE_AREA_ID;
    assert_eq!(
      compact_heightfield.span_areas(),
      [
        W, W, W, W, W, //
        W, W, W, W, W, //
        W, W, W, W, W, //
        W, W, W, W, W, //
        W, W, W, W, W, //
      ]
    );

    compact_heightfield
      .erode_walkable_area(&mut context, 1)
      .expect("erosion succeeds");

    assert_eq!(
      compact_heightfield.span_areas(),
      [
        0, 0, 0, 0, 0, //
        0, W, W, W, 0, //
        0, W, W, W, 0, //
        0, W, W, W, 0, //
        0, 0, 0, 0, 0, //
      ]
    );
  }

  #[test]
  fn marks_areas() {
    let mut context = Context::new();

    let min_bounds = Vec3::new(0.0, 0.0, 0.0);
    let max_bounds = Vec3::new(10.0, 10.0, 10.0);

    let mut heightfield =
      Heightfield::new(&mut context, min_bounds, max_bounds, 1.0, 1.0)
        .expect("creation succeeds");

    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(10.0, 0.5, 0.0),
      Vec3::new(10.0, 0.5, 10.0),
      Vec3::new(0.0, 0.5, 10.0),
    ];

    let triangles = [Vec3::new(0, 2, 1), Vec3::new(2, 0, 3)];
    let area_ids = [WALKABLE_AREA_ID, WALKABLE_AREA_ID];

    // SAFETY: Triangles all have valid indices.
    unsafe {
      heightfield.rasterize_indexed_triangles_i32_unchecked(
        &mut context,
        &vertices,
        &triangles,
        &area_ids,
        1,
      )
    }
    .expect("rasterization succeeds");

    let mut compact_heightfield =
      CompactHeightfield::<NoRegions>::new(&heightfield, &mut context, 3, 0)
        .expect("creating CompactHeightfield succeeds");

    compact_heightfield.mark_box_area_with_id(
      &mut context,
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(2.99, 1.0, 2.99),
      1,
    );

    compact_heightfield.mark_cylinder_area_with_id(
      &mut context,
      Vec3::new(2.5, 1.0, 7.5),
      2.5,
      2.0,
      2,
    );

    compact_heightfield.mark_convex_poly_area_with_id(
      &mut context,
      &[
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 3.0),
        Vec3::new(8.0, 0.0, 8.0),
        Vec3::new(7.0, 0.0, 8.0),
        Vec3::new(5.0, 0.0, 3.0),
      ],
      0.0,
      2.0,
      4,
    );

    const W: u8 = WALKABLE_AREA_ID;
    assert_eq!(
      compact_heightfield.span_areas(),
      [
        1, 1, 1, W, W, 4, 4, 4, 4, 4, //
        1, 1, 1, W, W, 4, 4, 4, 4, 4, //
        1, 1, 1, W, W, 4, 4, 4, 4, 4, //
        W, W, W, W, W, 4, 4, 4, 4, 4, //
        W, W, W, W, W, W, 4, 4, 4, W, //
        W, 2, 2, 2, W, W, 4, 4, 4, W, //
        2, 2, 2, 2, 2, W, 4, 4, 4, W, //
        2, 2, 2, 2, 2, W, W, 4, W, W, //
        2, 2, 2, 2, 2, W, W, W, W, W, //
        W, 2, 2, 2, W, W, W, W, W, W, //
      ]
    );
  }

  #[test]
  fn median_filter_applied() {
    let mut context = Context::new();

    let min_bounds = Vec3::new(0.0, 0.0, 0.0);
    let max_bounds = Vec3::new(10.0, 10.0, 10.0);

    let mut heightfield =
      Heightfield::new(&mut context, min_bounds, max_bounds, 1.0, 1.0)
        .expect("creation succeeds");

    let vertices = [
      Vec3::new(0.0, 0.5, 0.0),
      Vec3::new(10.0, 0.5, 0.0),
      Vec3::new(10.0, 0.5, 10.0),
      Vec3::new(0.0, 0.5, 10.0),
    ];

    let triangles = [Vec3::new(0, 2, 1), Vec3::new(2, 0, 3)];
    let area_ids = [WALKABLE_AREA_ID, WALKABLE_AREA_ID];

    // SAFETY: Triangles all have valid indices.
    unsafe {
      heightfield.rasterize_indexed_triangles_i32_unchecked(
        &mut context,
        &vertices,
        &triangles,
        &area_ids,
        1,
      )
    }
    .expect("rasterization succeeds");

    let mut compact_heightfield =
      CompactHeightfield::<NoRegions>::new(&heightfield, &mut context, 3, 0)
        .expect("creating CompactHeightfield succeeds");

    compact_heightfield.mark_box_area_with_id(
      &mut context,
      Vec3::new(3.01, 0.0, 3.01),
      Vec3::new(6.99, 1.0, 6.99),
      1,
    );

    compact_heightfield.mark_box_area_with_id(
      &mut context,
      Vec3::new(5.01, 0.0, 5.01),
      Vec3::new(10.0, 1.0, 10.0),
      2,
    );

    // One cell that is filtered out as noise.
    compact_heightfield.mark_box_area_with_id(
      &mut context,
      Vec3::new(1.01, 0.0, 1.01),
      Vec3::new(1.99, 1.0, 1.99),
      4,
    );

    const W: u8 = WALKABLE_AREA_ID;
    assert_eq!(
      compact_heightfield.span_areas(),
      [
        W, W, W, W, W, W, W, W, W, W, //
        W, 4, W, W, W, W, W, W, W, W, //
        W, W, W, W, W, W, W, W, W, W, //
        W, W, W, 1, 1, 1, 1, W, W, W, //
        W, W, W, 1, 1, 1, 1, W, W, W, //
        W, W, W, 1, 1, 2, 2, 2, 2, 2, //
        W, W, W, 1, 1, 2, 2, 2, 2, 2, //
        W, W, W, W, W, 2, 2, 2, 2, 2, //
        W, W, W, W, W, 2, 2, 2, 2, 2, //
        W, W, W, W, W, 2, 2, 2, 2, 2, //
      ]
    );

    compact_heightfield
      .median_filter_area_ids(&mut context)
      .expect("median filter succeeded");
    assert_eq!(
      compact_heightfield.span_areas(),
      [
        W, W, W, W, W, W, W, W, W, W, //
        W, W, W, W, W, W, W, W, W, W, //
        W, W, W, W, W, W, W, W, W, W, //
        W, W, W, W, 1, 1, W, W, W, W, //
        W, W, W, 1, 1, 1, 2, 2, W, W, //
        W, W, W, 1, 1, 1, 2, 2, 2, 2, //
        W, W, W, W, 2, 2, 2, 2, 2, 2, //
        W, W, W, W, 2, 2, 2, 2, 2, 2, //
        W, W, W, W, W, 2, 2, 2, 2, 2, //
        W, W, W, W, W, 2, 2, 2, 2, 2, //
      ]
    );
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

    let mut compact_heightfield = CompactHeightfield::<NoRegions>::new(
      &heightfield,
      &mut context,
      /* walkable_height= */ 3,
      /* walkable_climb= */ 0,
    )
    .expect("creating CompactHeightfield succeeds");

    compact_heightfield
      .erode_walkable_area(&mut context, /* radius= */ 1)
      .expect("erosion successful");

    let compact_heightfield_with_regions =
      build_fn(compact_heightfield, &mut context)
        .expect("building regions succeeds");

    assert_eq!(compact_heightfield_with_regions.border_size(), 0);
    assert_eq!(compact_heightfield_with_regions.max_region_id(), 2);
    assert_eq!(compact_heightfield_with_regions.max_distance(), 2);

    assert_eq!(
      compact_heightfield_with_regions
        .spans_iter()
        .map(|span| span.region_id())
        .collect::<Vec<_>>(),
      [
        0, 0, 0, 0, 0, //
        0, 1, 1, 1, 0, //
        0, 1, 1, 1, 0, //
        0, 1, 1, 1, 0, //
        0, 0, 0, 0, 0, //
      ]
    );
  }

  #[test]
  fn build_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_regions(
        context, /* border_size= */ 0, /* min_region_area= */ 1,
        /* merge_region_area= */ 1,
      )
    }

    build_regions_base(build_fn);
  }

  #[test]
  fn build_layer_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_layer_regions(
        context, /* border_size= */ 0, /* min_region_area= */ 1,
      )
    }

    build_regions_base(build_fn);
  }

  #[test]
  fn build_monotone_regions() {
    fn build_fn(
      compact_heightfield: CompactHeightfield<NoRegions>,
      context: &mut Context,
    ) -> Result<CompactHeightfield<HasRegions>, ()> {
      compact_heightfield.build_regions_monotone(
        context, /* border_size= */ 0, /* min_region_area= */ 1,
        /* merge_region_area= */ 1,
      )
    }

    build_regions_base(build_fn);
  }
}
