pub struct Vec3 {
  pub x: f32,
  pub y: f32,
  pub z: f32,
}

impl Vec3 {
  pub fn new(x: f32, y: f32, z: f32) -> Self {
    Self { x, y, z }
  }
}

pub struct IVec3 {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

impl IVec3 {
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    Self { x, y, z }
  }
}
