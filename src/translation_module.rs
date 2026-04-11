use crate::geometry_functions::{collect_points_from_solid, collect_points_from_surface_kind};
use ecitygml_core::model::building::Building;
use ecitygml_core::operations::CityObjectGeometry;

/// Compute the centroid of all geometry in a building by iterating over
/// all city objects (including BuildingConstructiveElements, rooms, etc.)
pub fn compute_building_centroid(building: &Building) -> Option<(f64, f64, f64)> {
    let mut sum_x = 0.0f64;
    let mut sum_y = 0.0f64;
    let mut sum_z = 0.0f64;
    let mut count = 0usize;

    for co_ref in building.iter_city_object() {
        let geom = CityObjectGeometry::from_city_object(co_ref);

        // Collect points from multi_surfaces
        for (_lod, ms) in &geom.multi_surfaces {
            for surface_kind in ms.surface_member() {
                for point in collect_points_from_surface_kind(surface_kind) {
                    sum_x += point[0];
                    sum_y += point[1];
                    sum_z += point[2];
                    count += 1;
                }
            }
        }

        // Collect points from solids
        for (_lod, solid) in &geom.solids {
            for point in collect_points_from_solid(solid) {
                sum_x += point[0];
                sum_y += point[1];
                sum_z += point[2];
                count += 1;
            }
        }
    }

    if count == 0 {
        return None;
    }

    let n = count as f64;
    Some((sum_x / n, sum_y / n, sum_z / n))
}
