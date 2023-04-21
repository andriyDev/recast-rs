#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Vec3<T> {
  pub x: T,
  pub y: T,
  pub z: T,
}

impl<T> Vec3<T> {
  pub fn new(x: T, y: T, z: T) -> Self {
    Self { x, y, z }
  }
}
