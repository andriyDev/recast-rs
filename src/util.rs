use std::ops::DerefMut;

use recastnavigation_sys::{
  rcCalcBounds, rcClearUnwalkableTriangles, rcMarkWalkableTriangles,
};

use crate::{Context, Vec3};

// Computes the bounds of the provided `vertices`. The returned tuple is
// `(min_bounds, max_bounds)`.
pub fn calculate_bounds(vertices: &[Vec3<f32>]) -> (Vec3<f32>, Vec3<f32>) {
  let mut min_bounds = Vec3::<f32>::new(0.0, 0.0, 0.0);
  let mut max_bounds = Vec3::<f32>::new(0.0, 0.0, 0.0);

  // SAFETY: `rcCalcBounds` only reads the vertices slice and only writes to
  // (owned) min_bounds and max_bounds. All f32 pointers are read as 3 floats,
  // which is the same as Vec3<f32>.
  unsafe {
    rcCalcBounds(
      vertices.as_ptr() as *const f32,
      vertices.len() as i32,
      &mut min_bounds as *mut Vec3<f32> as *mut f32,
      &mut max_bounds as *mut Vec3<f32> as *mut f32,
    )
  };

  (min_bounds, max_bounds)
}

// Marks triangles as walkable if their slope is less than
// `walkable_slope_angle`. Each triangle contains 3 indices that index into
// `vertices`. `WALKABLE_AREA_ID` will be stored in `triangle_area_ids` in the
// corresponding index for each triangle if walkable (so we must have
// `triangle_area_ids.len() == triangles.len()`).
// SAFETY: This function is only safe if all indices in `triangles` are less
// than the length of `vertices`.
pub unsafe fn mark_walkable_triangles_unchecked(
  context: &mut Context,
  walkable_slope_angle: f32,
  vertices: &[Vec3<f32>],
  triangles: &[Vec3<i32>],
  triangle_area_ids: &mut [u8],
) {
  assert_eq!(
    triangles.len(),
    triangle_area_ids.len(),
    "Each triangle should have one area id."
  );

  // SAFETY: `rcMarkWalkableTriangles` only mutates `context.context`, and
  // `triangle_area_ids` for the number of triangles. `vertices` and `triangles`
  // are only read in the valid portion (due to passing in the triangles len,
  // and the safety of the function).
  unsafe {
    rcMarkWalkableTriangles(
      context.context.deref_mut(),
      walkable_slope_angle,
      vertices.as_ptr() as *const f32,
      vertices.len() as i32,
      triangles.as_ptr() as *const i32,
      triangles.len() as i32,
      triangle_area_ids.as_mut_ptr(),
    )
  }
}

// Same as `mark_walkable_triangles_unchecked`, but checks that each triangle
// indexes a valid vertex first (panics otherwise).
pub fn mark_walkable_triangles(
  context: &mut Context,
  walkable_slope_angle: f32,
  vertices: &[Vec3<f32>],
  triangles: &[Vec3<i32>],
  triangle_area_ids: &mut [u8],
) {
  for triangle in triangles {
    assert!(
      0 <= triangle.x
        && triangle.x <= vertices.len() as i32
        && 0 <= triangle.y
        && triangle.y <= vertices.len() as i32
        && 0 <= triangle.z
        && triangle.z <= vertices.len() as i32,
      "Triangle indexes out-of-bounds vertex. Triangle={:?}, vertices_len={}",
      *triangle,
      vertices.len()
    );
  }

  // SAFETY: We have checked that all indices in `triangles` are valid.
  // Therefore, the function is guaranteed to be safe.
  unsafe {
    mark_walkable_triangles_unchecked(
      context,
      walkable_slope_angle,
      vertices,
      triangles,
      triangle_area_ids,
    )
  };
}

// Same as `mark_walkable_triangles_unchecked`, except it marks triangles
// unwalkable (`INVALID_AREA_ID`) if they are steeper than
// `walkable_slope_angle`.
// SAFETY: This function is only safe if all indices in `triangles` are less
// than the length of `vertices`.
pub unsafe fn clear_unwalkable_triangles_unchecked(
  context: &mut Context,
  walkable_slope_angle: f32,
  vertices: &[Vec3<f32>],
  triangles: &[Vec3<i32>],
  triangle_area_ids: &mut [u8],
) {
  assert_eq!(
    triangles.len(),
    triangle_area_ids.len(),
    "Each triangle should have one area id."
  );

  // SAFETY: `rcClearUnwalkableTriangles` only mutates `context.context`, and
  // `triangle_area_ids` for the number of triangles. `vertices` and `triangles`
  // are only read in the valid portion (due to passing in the triangles len,
  // and the safety of the function).
  unsafe {
    rcClearUnwalkableTriangles(
      context.context.deref_mut(),
      walkable_slope_angle,
      vertices.as_ptr() as *const f32,
      vertices.len() as i32,
      triangles.as_ptr() as *const i32,
      triangles.len() as i32,
      triangle_area_ids.as_mut_ptr(),
    )
  }
}

// Same as `clear_unwalkable_triangles_unchecked`, but checks that each triangle
// indexes a valid vertex first (panics otherwise).
pub fn clear_unwalkable_triangles(
  context: &mut Context,
  walkable_slope_angle: f32,
  vertices: &[Vec3<f32>],
  triangles: &[Vec3<i32>],
  triangle_area_ids: &mut [u8],
) {
  for triangle in triangles {
    assert!(
      0 <= triangle.x
        && triangle.x <= vertices.len() as i32
        && 0 <= triangle.y
        && triangle.y <= vertices.len() as i32
        && 0 <= triangle.z
        && triangle.z <= vertices.len() as i32,
      "Triangle indexes out-of-bounds vertex. Triangle={:?}, vertices_len={}",
      *triangle,
      vertices.len()
    );
  }

  // SAFETY: We have checked that all indices in `triangles` are valid.
  // Therefore, the function is guaranteed to be safe.
  unsafe {
    clear_unwalkable_triangles_unchecked(
      context,
      walkable_slope_angle,
      vertices,
      triangles,
      triangle_area_ids,
    )
  };
}

#[cfg(test)]
mod tests {
  use std::panic::AssertUnwindSafe;

  use crate::{util, Context, Vec3};

  #[test]
  fn calculates_bounds() {
    assert_eq!(
      util::calculate_bounds(&[
        Vec3::new(-1.0, 3.0, 0.0),
        Vec3::new(10.0, -2.0, 1.0),
        Vec3::new(5.0, 6.0, 2.0),
        Vec3::new(2.0, 4.0, 3.0),
      ]),
      (Vec3::new(-1.0, -2.0, 0.0), Vec3::new(10.0, 6.0, 3.0))
    );
  }

  #[test]
  fn marks_walkable_triangles() {
    let mut context = Context::new();

    let vertices = [
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(1.0, 0.0, 0.0),
      Vec3::new(1.0, 0.0, 1.0),
      Vec3::new(0.0, 0.0, 1.0),
      // First slope: < 45 degrees.
      Vec3::new(3.0, 1.0, 0.0),
      Vec3::new(3.0, 1.0, 1.0),
      // Second slope: > 45 degrees.
      Vec3::new(-1.0, 2.0, 0.0),
      Vec3::new(-1.0, 2.0, 1.0),
    ];

    let triangles = [
      Vec3::new(0, 2, 1),
      Vec3::new(2, 0, 3),
      // First slope.
      Vec3::new(1, 5, 4),
      Vec3::new(5, 1, 2),
      // Second slope.
      Vec3::new(6, 3, 0),
      Vec3::new(3, 6, 7),
    ];

    let mut triangle_area_ids = [0, 0, 0, 0, 0, 0];

    util::mark_walkable_triangles(
      &mut context,
      45.0,
      &vertices,
      &triangles,
      &mut triangle_area_ids,
    );

    const W: u8 = crate::WALKABLE_AREA_ID;
    assert_eq!(triangle_area_ids, [W, W, W, W, 0, 0]);
  }

  #[test]
  fn checks_for_invalid_triangles_before_marking_triangles_walkable() {
    let mut context = Context::new();

    let vertices = [
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(1.0, 0.0, 1.0),
      Vec3::new(1.0, 0.0, 0.0),
    ];

    let triangles = [Vec3::new(0, 1, 2)];

    let mut triangle_area_ids = [0];

    util::mark_walkable_triangles(
      &mut context,
      45.0,
      &vertices,
      &triangles,
      &mut triangle_area_ids,
    );

    const W: u8 = crate::WALKABLE_AREA_ID;
    assert_eq!(triangle_area_ids, [W]);

    let invalid_triangles = [
      Vec3::new(-1, 1, 2),
      Vec3::new(10, 1, 2),
      Vec3::new(0, -2, 2),
      Vec3::new(0, 20, 2),
      Vec3::new(0, 1, -3),
      Vec3::new(0, 1, 30),
    ];

    for invalid_triangle_slice in invalid_triangles.chunks(1) {
      let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        util::mark_walkable_triangles(
          &mut context,
          45.0,
          &vertices,
          invalid_triangle_slice,
          &mut triangle_area_ids,
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
  fn clears_unwalkable_triangles() {
    let mut context = Context::new();

    let vertices = [
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(1.0, 0.0, 0.0),
      Vec3::new(1.0, 0.0, 1.0),
      Vec3::new(0.0, 0.0, 1.0),
      // First slope: < 45 degrees.
      Vec3::new(3.0, 1.0, 0.0),
      Vec3::new(3.0, 1.0, 1.0),
      // Second slope: > 45 degrees.
      Vec3::new(-1.0, 2.0, 0.0),
      Vec3::new(-1.0, 2.0, 1.0),
    ];

    let triangles = [
      Vec3::new(0, 2, 1),
      Vec3::new(2, 0, 3),
      // First slope.
      Vec3::new(1, 5, 4),
      Vec3::new(5, 1, 2),
      // Second slope.
      Vec3::new(6, 3, 0),
      Vec3::new(3, 6, 7),
    ];

    const W: u8 = crate::WALKABLE_AREA_ID;
    let mut triangle_area_ids = [W, W, 1, 2, W, W];

    util::clear_unwalkable_triangles(
      &mut context,
      45.0,
      &vertices,
      &triangles,
      &mut triangle_area_ids,
    );

    assert_eq!(triangle_area_ids, [W, W, 1, 2, 0, 0]);
  }

  #[test]
  fn checks_for_invalid_triangles_before_clearing_triangles_unwalkable() {
    let mut context = Context::new();

    let vertices = [
      Vec3::new(0.0, 0.0, 0.0),
      Vec3::new(1.0, 10.0, 1.0),
      Vec3::new(1.0, 10.0, 0.0),
    ];

    let triangles = [Vec3::new(0, 1, 2)];

    let mut triangle_area_ids = [W];

    util::clear_unwalkable_triangles(
      &mut context,
      45.0,
      &vertices,
      &triangles,
      &mut triangle_area_ids,
    );

    const W: u8 = crate::WALKABLE_AREA_ID;
    assert_eq!(triangle_area_ids, [0]);

    let invalid_triangles = [
      Vec3::new(-1, 1, 2),
      Vec3::new(10, 1, 2),
      Vec3::new(0, -2, 2),
      Vec3::new(0, 20, 2),
      Vec3::new(0, 1, -3),
      Vec3::new(0, 1, 30),
    ];

    for invalid_triangle_slice in invalid_triangles.chunks(1) {
      let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        util::clear_unwalkable_triangles(
          &mut context,
          45.0,
          &vertices,
          invalid_triangle_slice,
          &mut triangle_area_ids,
        );
      }));
      assert!(
        result.is_err(),
        "Expected invalid triangle to break an assert, but succeeded. Triangle={:?}",
        invalid_triangle_slice[0]
      );
    }
  }
}
