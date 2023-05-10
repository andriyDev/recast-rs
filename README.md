# recast-rs

Rust bindings for Recast from `recastnavigation`.

## Getting started

Recast is essentially a toolbox for generating navigation meshes from geometry.
A good starting point for generating navigation meshes is:

1. Create a `Heightfield`.
2. Rasterize triangles into the `Heightfield`. Consider marking triangles as
   walkable.
3. Create a `CompactHeightfield` from the `Heightfield`.
4. (Optional) Erode the walkable area.
5. (Optional) Mark areas using boxes, cylinders, or convex polygons.
6. Build regions for the `CompactHeightfield`.
7. Create a `ContourSet` from the `CompactHeightfield` (with regions).
8. Create a `PolyMesh` from the `ContourSet`.
9. Use the `PolyMesh` data as a navigation mesh.

As a simple example of this process:

```Rust
use recast_rs::*;

fn generate_nav_mesh(
  vertices: &[Vec3<f32>],
  triangles: &[Vec3<i32>],
) -> Result<Vec<Vec<Vec3<f32>>>, ()> {
  let mut context = Context::new();

  let (min_bounds, max_bounds) = util::calculate_bounds(vertices);

  let mut heightfield = Heightfield::new(
    &mut context,
    min_bounds,
    max_bounds,
    /* cell_horizontal_size */ 1.0,
    /* cell_height= */ 1.0,
  )?;

  let mut area_ids = vec![0; triangles.len()];
  util::mark_walkable_triangles(
    &mut context,
    /* walkable_slope_angle= */ 45.0,
    vertices,
    triangles,
    &mut area_ids,
  );

  heightfield.rasterize_indexed_triangles_i32(
    &mut context,
    vertices,
    triangles,
    &area_ids,
    /* flag_merge_threshold= */ 1,
  )?;

  let mut compact_heightfield = CompactHeightfield::<NoRegions>::new(
    &heightfield,
    &mut context,
    /* walkable_height= */ 3,
    /* walkable_climb= */ 1,
  )?;

  compact_heightfield
    .erode_walkable_area(&mut context, /* radius= */ 1)?;

  let compact_heightfield = compact_heightfield.build_regions(
    &mut context,
    /* border_size= */ 0,
    /* min_region_area= */ 0,
    /* merge_region_area= */ 0,
  )?;

  let contour_set = ContourSet::new(
    &compact_heightfield,
    &mut context,
    /* max_error= */ 1.0,
    /* max_edge_len= */ 10,
    ContourBuildFlags {
      tessellate_wall_edges: true,
      tessellate_area_edges: false,
    },
  )?;

  let poly_mesh = PolyMesh::new(
    &contour_set,
    &mut context,
    /* max_vertices_per_polygon= */ 8,
  )?;

  Ok(
    poly_mesh
      .polygons_iter()
      .map(|polygon| {
        polygon
          .valid_vertices()
          .iter()
          .map(|&vertex_index| poly_mesh.vertex(vertex_index as usize).as_f32())
          .collect()
      })
      .collect(),
  )
}
```

Note in this example we just create a Vec for each polygon filled with the
vertices that make up that polygon. In practice, you likely want to extract the
neighbour information for each polygon to use this as a navigation mesh.

## License

Licensed under the [MIT license](LICENSE).
