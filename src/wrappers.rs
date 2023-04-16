use std::{
  ops::{Deref, DerefMut},
  ptr::NonNull,
};

use recastnavigation_sys::*;

pub struct RawContext(NonNull<rcContext>);

// SAFETY: The default rcContext implementation does not rely on thread-local
// state.
unsafe impl Send for RawContext {}

impl Drop for RawContext {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      DeleteContext(self.0.as_ptr());
    }
  }
}

impl RawContext {
  pub fn new() -> Self {
    // SAFETY: CreateContext always returns a valid, non-null pointer (else an
    // exception is thrown and everything crashes).
    Self(unsafe { NonNull::new_unchecked(CreateContext(false)) })
  }
}

impl Deref for RawContext {
  type Target = rcContext;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawContext {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawHeightfield(NonNull<rcHeightfield>);

// SAFETY: rcHeightfield does not use thread-local state.
unsafe impl Send for RawHeightfield {}

impl Drop for RawHeightfield {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreeHeightField(self.0.as_ptr());
    }
  }
}

impl RawHeightfield {
  // Creates a RecastHeightfield, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocHeightfield just allocates the rcHeightfield, or returns
    // null on failure.
    NonNull::new(unsafe { rcAllocHeightfield() }).map(|ptr| Self(ptr)).ok_or(())
  }
}

impl Deref for RawHeightfield {
  type Target = rcHeightfield;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawHeightfield {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawCompactHeightfield(NonNull<rcCompactHeightfield>);

// SAFETY: rcCompactHeightfield does not use thread-local state.
unsafe impl Send for RawCompactHeightfield {}

impl Drop for RawCompactHeightfield {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreeCompactHeightfield(self.0.as_ptr());
    }
  }
}

impl RawCompactHeightfield {
  // Creates a RecastCompactHeightfield, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocCompactHeightfield just allocates the
    // rcCompactHeightfield, or returns null on failure.
    NonNull::new(unsafe { rcAllocCompactHeightfield() })
      .map(|ptr| Self(ptr))
      .ok_or(())
  }
}

impl Deref for RawCompactHeightfield {
  type Target = rcCompactHeightfield;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawCompactHeightfield {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawHeightfieldLayerSet(NonNull<rcHeightfieldLayerSet>);

// SAFETY: rcHeightfieldLayerSet does not use thread-local state.
unsafe impl Send for RawHeightfieldLayerSet {}

impl Drop for RawHeightfieldLayerSet {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreeHeightfieldLayerSet(self.0.as_ptr());
    }
  }
}

impl RawHeightfieldLayerSet {
  // Creates a RecastHeightfieldLayerSet, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocHeightfieldLayerSet just allocates the
    // rcHeightfieldLayerSet, or returns null on failure.
    NonNull::new(unsafe { rcAllocHeightfieldLayerSet() })
      .map(|ptr| Self(ptr))
      .ok_or(())
  }
}

impl Deref for RawHeightfieldLayerSet {
  type Target = rcHeightfieldLayerSet;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawHeightfieldLayerSet {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawContourSet(NonNull<rcContourSet>);

// SAFETY: rcContourSet does not use thread-local state.
unsafe impl Send for RawContourSet {}

impl Drop for RawContourSet {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreeContourSet(self.0.as_ptr());
    }
  }
}

impl RawContourSet {
  // Creates a RecastContourSet, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocContourSet just allocates the rcContourSet, or returns
    // null on failure.
    NonNull::new(unsafe { rcAllocContourSet() }).map(|ptr| Self(ptr)).ok_or(())
  }
}

impl Deref for RawContourSet {
  type Target = rcContourSet;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawContourSet {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawPolyMesh(NonNull<rcPolyMesh>);

// SAFETY: rcPolyMesh does not use thread-local state.
unsafe impl Send for RawPolyMesh {}

impl Drop for RawPolyMesh {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreePolyMesh(self.0.as_ptr());
    }
  }
}

impl RawPolyMesh {
  // Creates a RecastPolyMesh, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocPolyMesh just allocates the rcPolyMesh, or returns
    // null on failure.
    NonNull::new(unsafe { rcAllocPolyMesh() }).map(|ptr| Self(ptr)).ok_or(())
  }
}

impl Deref for RawPolyMesh {
  type Target = rcPolyMesh;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawPolyMesh {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}

pub struct RawPolyMeshDetail(NonNull<rcPolyMeshDetail>);

// SAFETY: rcPolyMeshDetail does not use thread-local state.
unsafe impl Send for RawPolyMeshDetail {}

impl Drop for RawPolyMeshDetail {
  fn drop(&mut self) {
    // SAFETY: The pointer was allocated by Recast, has not been freed (since
    // the pointer is owned by self), and the pointer is not null.
    unsafe {
      rcFreePolyMeshDetail(self.0.as_ptr());
    }
  }
}

impl RawPolyMeshDetail {
  // Creates a RecastPolyMeshDetail, or returns None if allocation failed.
  pub fn new() -> Result<Self, ()> {
    // SAFETY: rcAllocPolyMeshDetail just allocates the rcPolyMeshDetail, or
    // returns null on failure.
    NonNull::new(unsafe { rcAllocPolyMeshDetail() })
      .map(|ptr| Self(ptr))
      .ok_or(())
  }
}

impl Deref for RawPolyMeshDetail {
  type Target = rcPolyMeshDetail;

  fn deref(&self) -> &Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed.
    unsafe { self.0.as_ref() }
  }
}

impl DerefMut for RawPolyMeshDetail {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The pointer is owned by self, so has not been freed and is
    // exclusive.
    unsafe { self.0.as_mut() }
  }
}
