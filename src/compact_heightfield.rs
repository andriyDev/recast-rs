use std::ops::{Deref, DerefMut};

use recastnavigation_sys::{
  rcBuildCompactHeightfield, rcBuildDistanceField, rcBuildLayerRegions,
  rcBuildRegions, rcBuildRegionsMonotone, rcErodeWalkableArea,
};

use crate::{wrappers, Context, Heightfield};

pub struct CompactHeightfield<TypeState: CompactHeightfieldState> {
  pub(crate) compact_heightfield: wrappers::RawCompactHeightfield,
  marker: std::marker::PhantomData<TypeState>,
}

pub enum NoRegions {}
pub enum HasRegions {}

pub trait CompactHeightfieldState {}
impl CompactHeightfieldState for NoRegions {}
impl CompactHeightfieldState for HasRegions {}

impl<TypeState: CompactHeightfieldState> CompactHeightfield<TypeState> {}

impl CompactHeightfield<NoRegions> {
  pub fn create_from_heightfield(
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
