use std::ops::{Deref, DerefMut};

use recastnavigation_sys::rcBuildHeightfieldLayers;

use crate::{
  wrappers, CompactHeightfield, CompactHeightfieldState, Context, Vec3,
};

pub struct HeightfieldLayerSet {
  pub(crate) layer_set: wrappers::RawHeightfieldLayerSet,
}

impl HeightfieldLayerSet {
  pub fn new(
    compact_heightfield: &CompactHeightfield<impl CompactHeightfieldState>,
    context: &mut Context,
    border_size: i32,
    walkable_height: i32,
  ) -> Result<HeightfieldLayerSet, ()> {
    let mut layer_set = wrappers::RawHeightfieldLayerSet::new()?;

    // SAFETY: rcBuildHeightfieldLayers only mutates `context.context` and
    // `layer_set`. It also only reads from
    // `compact_heightfield.compact_heightfield`.
    let build_succeeded = unsafe {
      rcBuildHeightfieldLayers(
        context.context.deref_mut(),
        compact_heightfield.compact_heightfield.deref(),
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
  pub fn min_bounds(&self) -> Vec3<f32> {
    Vec3::new(self.layer.bmin[0], self.layer.bmin[1], self.layer.bmin[2])
  }

  pub fn max_bounds(&self) -> Vec3<f32> {
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

  pub fn grid_min_bounds(&self) -> Vec3<i32> {
    Vec3::new(self.layer.minx, self.layer.hmin, self.layer.miny)
  }

  pub fn grid_max_bounds(&self) -> Vec3<i32> {
    Vec3::new(self.layer.maxx, self.layer.hmax, self.layer.maxy)
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

#[cfg(test)]
mod tests {
  use crate::{
    CompactHeightfield, Context, Heightfield, HeightfieldLayerSet, NoRegions,
    Vec3, WALKABLE_AREA_ID,
  };

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
}
