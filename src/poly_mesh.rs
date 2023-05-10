use std::ops::{Deref, DerefMut};

use recastnavigation_sys::{rcBuildPolyMesh, rcBuildPolyMeshDetail};

use crate::{
  wrappers, CompactHeightfield, CompactHeightfieldState, Context, ContourSet,
  Vec3,
};

pub use recastnavigation_sys::RC_MESH_NULL_IDX as NULL_INDEX;

// A Recast polygon mesh. This is essentially the completed navigation mesh.
pub struct PolyMesh {
  poly_mesh: wrappers::RawPolyMesh,
}

impl PolyMesh {
  // Creates a PolyMesh from a ContourSet. `max_vertices_per_polygon` determines
  // the maximum number of vertices each node/polygon can have.
  pub fn new(
    contour_set: &ContourSet,
    context: &mut Context,
    max_vertices_per_polygon: i32,
  ) -> Result<PolyMesh, ()> {
    let mut poly_mesh = wrappers::RawPolyMesh::new()?;

    // SAFETY: rcBuildPolyMesh only modifies `context.context` and `poly_mesh`,
    // both of which are taken by mutable borrows. `contour_set.contour_set` is
    // only read and is passed by immutable borrow.
    let build_succeeded = unsafe {
      rcBuildPolyMesh(
        context.context.deref_mut(),
        contour_set.contour_set.deref(),
        max_vertices_per_polygon,
        poly_mesh.deref_mut(),
      )
    };

    if build_succeeded {
      Ok(PolyMesh { poly_mesh })
    } else {
      Err(())
    }
  }

  fn raw_vertices(&self) -> &[Vec3<u16>] {
    // SAFETY: `self.poly_mesh.verts` has `self.poly_mesh.nverts` * 3 u16's
    // which lines up perfectly with `self.poly_mesh.nverts` Vec3<u16>'s. The
    // lifetime is also correct since `self` owns the verts memory (through
    // `self.poly_mesh`).
    unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.verts as *const Vec3<u16>,
        self.poly_mesh.nverts as usize,
      )
    }
  }

  pub fn vertices_len(&self) -> usize {
    self.poly_mesh.nverts as usize
  }

  // Gets a PolyMeshVertex by index.
  pub fn vertex(&self, index: usize) -> PolyMeshVertex {
    assert!(index < self.vertices_len());
    PolyMeshVertex { poly_mesh: self, index }
  }

  // Creates an Iterator of all vertices.
  pub fn vertices_iter(&self) -> impl Iterator<Item = PolyMeshVertex> + '_ {
    (0..self.vertices_len())
      .map(|index| PolyMeshVertex { poly_mesh: self, index })
  }

  pub fn polygons_len(&self) -> usize {
    self.poly_mesh.npolys as usize
  }

  // Gets a PolyMeshPolygon by index.
  pub fn polygon(&self, index: usize) -> PolyMeshPolygon {
    assert!(index < self.polygons_len());
    PolyMeshPolygon { poly_mesh: self, index }
  }

  // Creates an Iterator of all polygons.
  pub fn polygons_iter(&self) -> impl Iterator<Item = PolyMeshPolygon> + '_ {
    (0..self.polygons_len())
      .map(|index| PolyMeshPolygon { poly_mesh: self, index })
  }

  pub fn max_vertices_per_polygon(&self) -> i32 {
    self.poly_mesh.nvp
  }

  pub fn min_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.poly_mesh.bmin[0],
      self.poly_mesh.bmin[1],
      self.poly_mesh.bmin[2],
    )
  }

  pub fn max_bounds(&self) -> Vec3<f32> {
    Vec3::new(
      self.poly_mesh.bmax[0],
      self.poly_mesh.bmax[1],
      self.poly_mesh.bmax[2],
    )
  }

  pub fn cell_horizontal_size(&self) -> f32 {
    self.poly_mesh.cs
  }

  pub fn cell_height(&self) -> f32 {
    self.poly_mesh.ch
  }

  pub fn border_size(&self) -> i32 {
    self.poly_mesh.borderSize
  }

  pub fn max_edge_error(&self) -> f32 {
    self.poly_mesh.maxEdgeError
  }
}

// A single vertex in a PolyMesh. This is used to select how to represent the
// vertex (either in grid units or in "world" units).
pub struct PolyMeshVertex<'poly_mesh> {
  poly_mesh: &'poly_mesh PolyMesh,
  index: usize,
}

impl<'poly_mesh> PolyMeshVertex<'poly_mesh> {
  // Gets the vertex as its grid coordinates.
  pub fn as_u16(&self) -> Vec3<u16> {
    self.poly_mesh.raw_vertices()[self.index]
  }

  // Gets the vertex in "world space".
  pub fn as_f32(&self) -> Vec3<f32> {
    let raw_vector = self.as_u16();
    Vec3::<f32>::new(
      raw_vector.x as f32 * self.poly_mesh.poly_mesh.cs,
      raw_vector.y as f32 * self.poly_mesh.poly_mesh.ch,
      raw_vector.z as f32 * self.poly_mesh.poly_mesh.cs,
    )
  }
}

// A single polygon in a PolyMesh.
pub struct PolyMeshPolygon<'poly_mesh> {
  poly_mesh: &'poly_mesh PolyMesh,
  index: usize,
}

impl<'poly_mesh> PolyMeshPolygon<'poly_mesh> {
  // Gets the "raw" indices of each vertex in the polygon. This will always have
  // a length of `poly_mesh.max_vertices_per_polygon`. Polygons with fewer
  // vertices than the max will fill the remaining entries with `NULL_INDEX`.
  // Use `valid_vertices` to get the valid slice or use an iterator to filter
  // out `NULL_INDEX` values.
  pub fn vertices(&self) -> &'poly_mesh [u16] {
    let nvp = self.poly_mesh.poly_mesh.nvp as usize;

    // SAFETY: `polys` has a length of `maxpolys` * 2 * `nvp`. A lower-bound on
    // this is `npolys` in place of `maxpolys` since `npolys` <= `maxpolys`.
    // Therefore, the slice is fully covered by the allocated portion of
    // `polys`.
    let polys = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.poly_mesh.polys,
        self.poly_mesh.polygons_len() * 2 * nvp,
      )
    };

    let start_index = self.index * 2 * nvp;
    &polys[start_index..(start_index + nvp)]
  }

  // Gets the "raw" indices of the neighbours of each edge in the polygon. This
  // will always have a length of `poly_mesh.max_vertices_per_polygon`.
  // Polygons with fewer edges (equivalent to number of vertices) than the max
  // will fill the remaining entries with `NULL_INDEX`. Use `valid_neighbours`
  // to get the valid slice or use an iterator to filter out `NULL_INDEX`
  // values.
  pub fn neighbours(&self) -> &'poly_mesh [u16] {
    let nvp = self.poly_mesh.poly_mesh.nvp as usize;

    // SAFETY: `polys` has a length of `maxpolys` * 2 * `nvp`. A lower-bound on
    // this is `npolys` in place of `maxpolys` since `npolys` <= `maxpolys`.
    // Therefore, the slice is fully covered by the allocated portion of
    // `polys`.
    let polys = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.poly_mesh.polys,
        self.poly_mesh.polygons_len() * 2 * nvp,
      )
    };

    // Start one `nvp` over to start at the neighbour section of the polygon.
    let start_index = self.index * 2 * nvp + nvp;
    &polys[start_index..(start_index + nvp)]
  }

  // Same as `vertices`, but sliced to only include indices that aren't
  // `NULL_INDEX`. Note this involves a linear search to find the end of the
  // valid vertices, so it may be more expensive than simply filtering out
  // `NULL_INDEX`.
  pub fn valid_vertices(&self) -> &'poly_mesh [u16] {
    let vertices = self.vertices();
    let index = vertices
      .iter()
      .position(|vertex_index| {
        *vertex_index == recastnavigation_sys::RC_MESH_NULL_IDX
      })
      .unwrap_or(vertices.len());

    &vertices[..index]
  }

  // Same as `neighbours`, but sliced to only include indices that aren't
  // `NULL_INDEX`. Note this involves a linear search to find the end of the
  // valid neighbours, so it may be more expensive than simply filtering out
  // `NULL_INDEX`.
  pub fn valid_neighbours(&self) -> &'poly_mesh [u16] {
    let neighbours = self.neighbours();

    &neighbours[..self.valid_vertices().len()]
  }

  pub fn region_id(&self) -> u16 {
    // SAFETY: `regs` has a length of `maxpolys` which is >= `npolys`.
    // Therefore, the slice is fully covered by the allocated portion of `regs`.
    let regs = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.poly_mesh.regs,
        self.poly_mesh.polygons_len(),
      )
    };

    regs[self.index]
  }

  pub fn flags(&self) -> u16 {
    // SAFETY: `flags` has a length of `maxpolys` which is >= `npolys`.
    // Therefore, the slice is fully covered by the allocated portion of
    // `flags`.
    let flags = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.poly_mesh.flags,
        self.poly_mesh.polygons_len(),
      )
    };

    flags[self.index]
  }

  pub fn area_id(&self) -> u8 {
    // SAFETY: `areas` has a length of `maxpolys` which is >= `npolys`.
    // Therefore, the slice is fully covered by the allocated portion of
    // `areas`.
    let areas = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh.poly_mesh.areas,
        self.poly_mesh.polygons_len(),
      )
    };

    areas[self.index]
  }
}

// A Recast detailed polygon mesh. This is a triangle mesh that stores detailed
// height data for each polygon in its associated PolyMesh.
pub struct PolyMeshDetail {
  poly_mesh_detail: wrappers::RawPolyMeshDetail,
}

impl PolyMeshDetail {
  // Creates a PolyMeshDetail from a PolyMesh and a CompactHeightfield.
  // `sample_distance` is how frequently (in world units) to samples the height
  // at. `sample_max_error` is the max distance that the mesh surface should
  // deviate from the heightfield data.
  pub fn new(
    poly_mesh: &PolyMesh,
    context: &mut Context,
    compact_heightfield: &CompactHeightfield<impl CompactHeightfieldState>,
    sample_distance: f32,
    sample_max_error: f32,
  ) -> Result<PolyMeshDetail, ()> {
    let mut poly_mesh_detail = wrappers::RawPolyMeshDetail::new()?;

    // SAFETY: rcBuildPolyMeshDetail only modifies `context.context` and
    // `poly_mesh_detail`, both of which are taken by mutable borrows.
    // `poly_mesh.poly_mesh` and `compact_heightfield.compact_heightfield` are
    // only read and are passed by immutable borrows.
    let build_succeeded = unsafe {
      rcBuildPolyMeshDetail(
        context.context.deref_mut(),
        poly_mesh.poly_mesh.deref(),
        compact_heightfield.compact_heightfield.deref(),
        sample_distance,
        sample_max_error,
        poly_mesh_detail.deref_mut(),
      )
    };

    if build_succeeded {
      Ok(PolyMeshDetail { poly_mesh_detail })
    } else {
      Err(())
    }
  }

  // Vertices for all polygons in the detail mesh.
  pub fn vertices(&self) -> &[Vec3<f32>] {
    // SAFETY: `verts` has `nverts` * 3 f32's, so casting to `nverts`
    // Vec3<f32>'s is safe. The slice is well aligned and non-null since
    // creation was successful.
    unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh_detail.verts as *const Vec3<f32>,
        self.poly_mesh_detail.nverts as usize,
      )
    }
  }

  pub fn submeshes_len(&self) -> usize {
    self.poly_mesh_detail.nmeshes as usize
  }

  // Gets a single PolyMeshDetailSubmesh by its index.
  pub fn submesh(&self, index: usize) -> PolyMeshDetailSubmesh<'_> {
    assert!(index <= self.submeshes_len());
    PolyMeshDetailSubmesh { poly_mesh_detail: self, index }
  }

  // Creates an Iterator of all submeshes.
  pub fn submeshes_iter(
    &self,
  ) -> impl Iterator<Item = PolyMeshDetailSubmesh> + '_ {
    (0..self.submeshes_len())
      .map(|index| PolyMeshDetailSubmesh { poly_mesh_detail: self, index })
  }
}

// A single submesh of a PolyMeshDetail. A submesh generally represents a single
// node/polygon of a PolyMesh.
pub struct PolyMeshDetailSubmesh<'poly_mesh_detail> {
  poly_mesh_detail: &'poly_mesh_detail PolyMeshDetail,
  index: usize,
}

impl<'poly_mesh_detail> PolyMeshDetailSubmesh<'poly_mesh_detail> {
  fn raw_mesh_data(&self) -> &'poly_mesh_detail [u32] {
    // SAFETY: `meshes` is guaranteed to be non-null, well-aligned, and have 4 *
    // `nmeshes` u32's, since constructing the PolyMeshDetail succeeded.
    let raw_meshes = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh_detail.poly_mesh_detail.meshes,
        self.poly_mesh_detail.poly_mesh_detail.nmeshes as usize * 4,
      )
    };

    &raw_meshes[(self.index * 4)..(self.index * 4 + 4)]
  }

  // Gets a slice of the vertices that make up this submesh.
  pub fn vertices(&self) -> &'poly_mesh_detail [Vec3<f32>] {
    let raw_mesh_data = self.raw_mesh_data();
    let vert_start = raw_mesh_data[0] as usize;
    let vert_len = raw_mesh_data[1] as usize;
    &self.poly_mesh_detail.vertices()[vert_start..(vert_start + vert_len)]
  }

  pub fn triangles_len(&self) -> usize {
    let raw_mesh_data = self.raw_mesh_data();
    raw_mesh_data[3] as usize
  }

  // Creates an Iterator of triangles.
  pub fn triangles_iter(
    &self,
  ) -> impl Iterator<Item = PolyMeshDetailTriangle<'poly_mesh_detail>>
       + 'poly_mesh_detail {
    let raw_mesh_data = self.raw_mesh_data();
    let tri_start = raw_mesh_data[2] as usize;
    let tri_len = raw_mesh_data[3] as usize;

    // SAFETY: `tris` is guaranteed to be non-null, well-aligned, and have 4 *
    // `ntris` u8's, since constructing the PolyMeshDetail succeeded.
    let raw_triangles = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh_detail.poly_mesh_detail.tris,
        self.poly_mesh_detail.poly_mesh_detail.ntris as usize * 4,
      )
    };

    (tri_start..(tri_start + tri_len)).map(|tri_index| PolyMeshDetailTriangle {
      raw_triangle_data: &raw_triangles[(tri_index * 4)..(tri_index * 4 + 4)],
    })
  }

  // Gets a single PolyMeshDetailTriangle by its index.
  pub fn triangle(
    &self,
    index: usize,
  ) -> PolyMeshDetailTriangle<'poly_mesh_detail> {
    let raw_mesh_data = self.raw_mesh_data();
    let tri_start = raw_mesh_data[2] as usize;
    let tri_len = raw_mesh_data[3] as usize;
    assert!(tri_start <= index && index < tri_start + tri_len);

    // SAFETY: `tris` is guaranteed to be non-null, well-aligned, and have 4 *
    // `ntris` u8's, since constructing the PolyMeshDetail succeeded.
    let raw_triangles = unsafe {
      std::slice::from_raw_parts(
        self.poly_mesh_detail.poly_mesh_detail.tris,
        self.poly_mesh_detail.poly_mesh_detail.ntris as usize * 4,
      )
    };
    PolyMeshDetailTriangle {
      raw_triangle_data: &raw_triangles[(index * 4)..(index * 4 + 4)],
    }
  }
}

// A single triangle in a PolyMeshDetail.
pub struct PolyMeshDetailTriangle<'poly_mesh_detail> {
  raw_triangle_data: &'poly_mesh_detail [u8],
}

impl<'poly_mesh_detail> PolyMeshDetailTriangle<'poly_mesh_detail> {
  // Gets the indices of the vertices that make up the triangle as a triplet.
  pub fn vertex_indices(&self) -> (usize, usize, usize) {
    (
      self.raw_triangle_data[0] as usize,
      self.raw_triangle_data[1] as usize,
      self.raw_triangle_data[2] as usize,
    )
  }

  // Returns whether each edge of the triangle is on the boundary of the
  // PolyMesh. The boundaries are recorded as (index_0_to_index_1,
  // index_1_to_index_2, index_2_to_index_0).
  pub fn are_edges_on_mesh_boundary(&self) -> (bool, bool, bool) {
    let triangle_flags = self.raw_triangle_data[3];
    (
      triangle_flags & 0b000001 != 0,
      triangle_flags & 0b000100 != 0,
      triangle_flags & 0b010000 != 0,
    )
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    CompactHeightfield, Context, ContourBuildFlags, ContourSet, Heightfield,
    NoRegions, PolyMesh, PolyMeshDetail, Vec3, NULL_INDEX, WALKABLE_AREA_ID,
  };

  #[test]
  fn build_poly_mesh() {
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

    let compact_heightfield_with_regions = compact_heightfield
      .build_regions(
        &mut context,
        /* border_size= */ 0,
        /* min_region_area= */ 1,
        /* merge_region_area= */ 1,
      )
      .expect("regions built");

    let contour_set = ContourSet::new(
      &compact_heightfield_with_regions,
      &mut context,
      /* max_error= */ 1.0,
      /* max_edge_len= */ 10,
      ContourBuildFlags {
        tessellate_wall_edges: true,
        tessellate_area_edges: false,
      },
    )
    .expect("contours built");

    let poly_mesh = PolyMesh::new(
      &contour_set,
      &mut context,
      /* max_vertices_per_polygon= */ 5,
    )
    .expect("poly mesh built");

    assert_eq!(poly_mesh.max_vertices_per_polygon(), 5);
    assert_eq!(poly_mesh.min_bounds(), min_bounds);
    assert_eq!(
      poly_mesh.max_bounds(),
      Vec3::new(max_bounds.x, max_bounds.y + 3.0, max_bounds.z)
    );
    assert_eq!(poly_mesh.cell_horizontal_size(), 1.0);
    assert_eq!(poly_mesh.cell_height(), 1.0);
    assert_eq!(poly_mesh.border_size(), 0);
    assert_eq!(poly_mesh.max_edge_error(), 1.0);

    let raw_vertices = poly_mesh
      .vertices_iter()
      .map(|vertex| vertex.as_u16())
      .collect::<Vec<Vec3<u16>>>();

    assert_eq!(
      raw_vertices,
      [
        Vec3::<u16>::new(0, 1, 0),
        Vec3::<u16>::new(0, 1, 5),
        Vec3::<u16>::new(5, 1, 5),
        Vec3::<u16>::new(5, 1, 0),
      ]
    );

    let vertices = poly_mesh
      .vertices_iter()
      .map(|vertex| vertex.as_f32())
      .collect::<Vec<Vec3<f32>>>();

    assert_eq!(
      vertices,
      [
        Vec3::<f32>::new(0.0, 1.0, 0.0),
        Vec3::<f32>::new(0.0, 1.0, 5.0),
        Vec3::<f32>::new(5.0, 1.0, 5.0),
        Vec3::<f32>::new(5.0, 1.0, 0.0),
      ]
    );

    let polygon_vertices = poly_mesh
      .polygons_iter()
      .map(|polygon| polygon.valid_vertices())
      .collect::<Vec<_>>();

    assert_eq!(polygon_vertices, [&[0, 1, 2, 3]]);

    let polygon_neighbours = poly_mesh
      .polygons_iter()
      .map(|polygon| polygon.valid_neighbours())
      .collect::<Vec<_>>();

    assert_eq!(
      polygon_neighbours,
      [&[NULL_INDEX, NULL_INDEX, NULL_INDEX, NULL_INDEX]]
    );
  }

  #[test]
  fn build_poly_mesh_detail() {
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

    let compact_heightfield_with_regions = compact_heightfield
      .build_regions(
        &mut context,
        /* border_size= */ 0,
        /* min_region_area= */ 1,
        /* merge_region_area= */ 1,
      )
      .expect("regions built");

    let contour_set = ContourSet::new(
      &compact_heightfield_with_regions,
      &mut context,
      /* max_error= */ 1.0,
      /* max_edge_len= */ 10,
      ContourBuildFlags {
        tessellate_wall_edges: true,
        tessellate_area_edges: false,
      },
    )
    .expect("contours built");

    let poly_mesh = PolyMesh::new(
      &contour_set,
      &mut context,
      /* max_vertices_per_polygon= */ 5,
    )
    .expect("poly mesh built");

    let poly_mesh_detail = PolyMeshDetail::new(
      &poly_mesh,
      &mut context,
      &compact_heightfield_with_regions,
      /* sample_distance= */ 1.0,
      /* sample_max_error= */ 0.1,
    )
    .expect("poly mesh detail built");

    let vertices = poly_mesh_detail
      .submeshes_iter()
      .map(|submesh| submesh.vertices())
      .collect::<Vec<_>>();

    assert_eq!(
      vertices,
      [[
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(0.0, 2.0, 5.0),
        Vec3::new(5.0, 2.0, 5.0),
        Vec3::new(5.0, 2.0, 0.0),
      ]]
    );

    let triangle_indices = poly_mesh_detail
      .submeshes_iter()
      .map(|submesh| {
        submesh
          .triangles_iter()
          .map(|triangle| triangle.vertex_indices())
          .collect()
      })
      .collect::<Vec<Vec<_>>>();

    assert_eq!(triangle_indices, [[(3, 0, 2), (0, 1, 2)]]);

    let triangle_edge_boundaries = poly_mesh_detail
      .submeshes_iter()
      .map(|submesh| {
        submesh
          .triangles_iter()
          .map(|triangle| triangle.are_edges_on_mesh_boundary())
          .collect()
      })
      .collect::<Vec<Vec<_>>>();

    assert_eq!(
      triangle_edge_boundaries,
      [[(true, false, true), (true, true, false)]]
    );
  }
}
