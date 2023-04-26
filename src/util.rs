use recastnavigation_sys::rcCalcBounds;

use crate::Vec3;

pub fn calculate_bounds(vertices: &[Vec3<f32>]) -> (Vec3<f32>, Vec3<f32>) {
  let mut min_bounds = Vec3::<f32>::new(0.0, 0.0, 0.0);
  let mut max_bounds = Vec3::<f32>::new(0.0, 0.0, 0.0);

  // SAFETY: rcCalcBounds only reads the vertices slice and only writes to
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

#[cfg(test)]
mod tests {
  use crate::{util, Vec3};

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
}
