use earcut::Earcut;
use earcut::utils3d::project3d_to_2d;
use ecitygml_core::model::building::Building;
use ecitygml_core::model::core::AsAbstractFeature;
use ecitygml_core::operations::CityObjectGeometry;
use egml::model::base::Id;
use egml::model::geometry::primitives::{Polygon, Solid, SurfaceKind};

/// Extract all 3D points from a SurfaceKind.
/// Avoids the library's `.points()` which is unimplemented for Surface/CompositeSurface.
pub fn collect_points_from_surface_kind(surface_kind: &SurfaceKind) -> Vec<[f64; 3]> {
    match surface_kind {
        SurfaceKind::Polygon(polygon) => {
            let mut out = Vec::new();
            if let Some(exterior) = &polygon.exterior {
                for p in exterior.points() {
                    out.push([p.x(), p.y(), p.z()]);
                }
            }
            for interior in &polygon.interior {
                for p in interior.points() {
                    out.push([p.x(), p.y(), p.z()]);
                }
            }
            out
        }
        SurfaceKind::Surface(_) | SurfaceKind::CompositeSurface(_) => {
            // Triangulate and extract triangle vertices
            let mut out = Vec::new();
            if let Ok(tri_surface) = surface_kind.triangulate() {
                for tri in tri_surface.triangles() {
                    out.push([tri.a.x(), tri.a.y(), tri.a.z()]);
                    out.push([tri.b.x(), tri.b.y(), tri.b.z()]);
                    out.push([tri.c.x(), tri.c.y(), tri.c.z()]);
                }
            }
            out
        }
    }
}

/// Extract all 3D points from a Solid's surface members.
pub fn collect_points_from_solid(solid: &Solid) -> Vec<[f64; 3]> {
    let mut out = Vec::new();
    for surface_property in solid.members() {
        out.extend(collect_points_from_surface_kind(&surface_property.content));
    }
    out
}

/// Triangulate a SurfaceKind (Polygon, Surface, or CompositeSurface).
/// Returns (triangle_indices, all_3d_points, surface_id).
pub fn triangulate_surface_kind(surface_kind: &SurfaceKind) -> (Vec<u32>, Vec<[f64; 3]>, Id) {
    match surface_kind {
        SurfaceKind::Polygon(polygon) => {
            let (triangles, points) = triangulate_polygon(polygon);
            let id = Id::from_hashed_string(&format!("{:?}", polygon.exterior));
            (triangles, points, id)
        }
        SurfaceKind::Surface(_) | SurfaceKind::CompositeSurface(_) => {
            let id = Id::from_hashed_string(&format!("{:p}", surface_kind as *const _));

            // Use the library's triangulation to get Triangle objects directly
            if let Ok(tri_surface) = surface_kind.triangulate() {
                let lib_triangles = tri_surface.triangles();
                let mut points: Vec<[f64; 3]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();

                for tri in lib_triangles {
                    let base = points.len() as u32;
                    points.push([tri.a.x(), tri.a.y(), tri.a.z()]);
                    points.push([tri.b.x(), tri.b.y(), tri.b.z()]);
                    points.push([tri.c.x(), tri.c.y(), tri.c.z()]);
                    indices.push(base);
                    indices.push(base + 1);
                    indices.push(base + 2);
                }
                (indices, points, id)
            } else {
                (Vec::new(), Vec::new(), id)
            }
        }
    }
}

/// Triangulate a single Polygon using earcut.
fn triangulate_polygon(polygon: &Polygon) -> (Vec<u32>, Vec<[f64; 3]>) {
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    let mut exterior_len = 0usize;

    // Exterior ring
    if let Some(exterior) = &polygon.exterior {
        let pts = exterior.points();
        exterior_len = pts.len();
        for point in pts {
            all_points.push([point.x(), point.y(), point.z()]);
        }
    } else {
        return (Vec::new(), Vec::new());
    }

    // Interior rings
    let mut hole_indices: Vec<u32> = Vec::new();
    for interior_ring in &polygon.interior {
        hole_indices.push(all_points.len() as u32);
        let pts = interior_ring.points();
        for point in pts {
            all_points.push([point.x(), point.y(), point.z()]);
        }
    }

    // Project to 2D
    let mut all_points_projected: Vec<[f64; 2]> = Vec::new();
    let _ = project3d_to_2d(&all_points, exterior_len, &mut all_points_projected);

    // Perform triangulation
    let mut triangles = vec![];
    let mut earcut = Earcut::new();
    earcut.earcut(
        all_points_projected.iter().copied(),
        &hole_indices,
        &mut triangles,
    );

    (triangles, all_points)
}

pub fn construct_buffered_bounding_box(
    input_building: &Building,
) -> (Vec<[f64; 3]>, Vec<[u64; 3]>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Try envelope first, fall back to computing from geometry
    let bounds = if let Some(envelope) = input_building.bounded_by() {
        let lower = envelope.lower_corner();
        let upper = envelope.upper_corner();
        Some((lower.x(), lower.y(), lower.z(), upper.x(), upper.y(), upper.z()))
    } else {
        compute_bbox_from_geometry(input_building)
    };

    if let Some((x_min_raw, y_min_raw, z_min_raw, x_max_raw, y_max_raw, z_max_raw)) = bounds {
        // Apply 2m buffer
        let x_min = x_min_raw - 2.0;
        let y_min = y_min_raw - 2.0;
        let z_min = z_min_raw - 2.0;
        let x_max = x_max_raw + 2.0;
        let y_max = y_max_raw + 2.0;
        let z_max = z_max_raw + 2.0;

        // Define the 8 corners of the buffered box
        let corners = vec![
            [x_min, y_min, z_min],
            [x_max, y_min, z_min],
            [x_max, y_max, z_min],
            [x_min, y_max, z_min],
            [x_min, y_min, z_max],
            [x_max, y_min, z_max],
            [x_max, y_max, z_max],
            [x_min, y_max, z_max],
        ];

        vertices.extend_from_slice(&corners);

        let base_index = vertices.len() as u64;
        let mut next_index = base_index;
        let offset = 0.5;

        for &corner in &corners {
            let tip = [
                corner[0] + offset * (if corner[0] == x_min { 1.0 } else { -1.0 }),
                corner[1] + offset * (if corner[1] == y_min { 1.0 } else { -1.0 }),
                corner[2] + offset * (if corner[2] == z_min { 1.0 } else { -1.0 }),
            ];

            vertices.push(tip);
            let tip_idx = next_index;
            next_index += 1;

            let corner_idx = vertices.iter().position(|&v| v == corner).unwrap() as u64;

            let neighbors = [
                [corner[0], corner[1], tip[2]],
                [corner[0], tip[1], corner[2]],
                [tip[0], corner[1], corner[2]],
            ];

            let mut neighbor_indices = Vec::new();
            for neighbor in neighbors {
                let idx = vertices.iter().position(|&v| v == neighbor);
                let n_idx = if let Some(i) = idx {
                    i as u64
                } else {
                    vertices.push(neighbor);
                    let i = next_index;
                    next_index += 1;
                    i
                };
                neighbor_indices.push(n_idx);
            }

            for &n_idx in &neighbor_indices {
                indices.push([tip_idx, corner_idx, n_idx]);
            }
        }

        return (vertices, indices);
    }

    println!("Envelope hat keine gültigen lower/upper corner Koordinaten.");
    (vertices, indices)
}

fn compute_bbox_from_geometry(building: &Building) -> Option<(f64, f64, f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut found = false;

    for co_ref in building.iter_city_object() {
        let geom = CityObjectGeometry::from_city_object(co_ref);

        for (_lod, ms) in &geom.multi_surfaces {
            for surface_kind in ms.surface_member() {
                for point in collect_points_from_surface_kind(surface_kind) {
                    found = true;
                    min_x = min_x.min(point[0]);
                    min_y = min_y.min(point[1]);
                    min_z = min_z.min(point[2]);
                    max_x = max_x.max(point[0]);
                    max_y = max_y.max(point[1]);
                    max_z = max_z.max(point[2]);
                }
            }
        }

        for (_lod, solid) in &geom.solids {
            for point in collect_points_from_solid(solid) {
                found = true;
                min_x = min_x.min(point[0]);
                min_y = min_y.min(point[1]);
                min_z = min_z.min(point[2]);
                max_x = max_x.max(point[0]);
                max_y = max_y.max(point[1]);
                max_z = max_z.max(point[2]);
            }
        }
    }

    if found {
        Some((min_x, min_y, min_z, max_x, max_y, max_z))
    } else {
        None
    }
}

pub fn import_bounding_box(_path_to_bounding_box: &str) {
    // todo: muss noch implementiert werden
}
