use ecitygml_core::model::building::{Building, BuildingConstructiveElement};
use ecitygml_core::model::construction::{GroundSurface, RoofSurface, WallSurface};
use ecitygml_core::operations::FeatureWithGeometry;
use egml::model::geometry::{MultiSurface, Polygon};
use egml::operations::geometry::Geometry;

pub fn process_building_components_sequential(input_building: &Building) -> Vec<[f64; 3]> {
    let mut all_building_points: Vec<[f64; 3]> = Vec::new();
    let mut all_wall_points: Vec<[f64; 3]> = Vec::new();
    let mut all_roof_points: Vec<[f64; 3]> = Vec::new();
    let mut all_ground_points: Vec<[f64; 3]> = Vec::new();

    // Obtain the building id
    let all_wall_surface = &input_building.wall_surface;
    for wall_surface in all_wall_surface {
        // get the wall surface id
        let all_wall_points_tmp = process_wall_surface_sequential(wall_surface);
        all_wall_points.extend(all_wall_points_tmp);
    }

    // Take care of the Roof Surfaces
    let all_roof_surface = &input_building.roof_surface;
    for roof_surface in all_roof_surface {
        let all_roof_points_tmp = process_roof_surface_sequential(roof_surface);
        all_roof_points.extend(&all_roof_points_tmp);
    }

    // Take care of the Ground Surfaces
    let all_ground_surface = &input_building.ground_surface;
    for ground_surface in all_ground_surface {
        let all_ground_points_tmp = process_ground_surface_sequential(ground_surface);
        all_ground_points.extend(&all_ground_points_tmp);
    }

    // Take care of BuildingConstructiveElements (IFC-converted CityGML 3.0)
    for bce in &input_building.building_constructive_element {
        let bce_points = process_building_constructive_element_sequential(bce);
        all_building_points.extend(bce_points);
    }

    all_building_points.extend(&all_ground_points);
    all_building_points.extend(&all_roof_points);
    all_building_points.extend(&all_wall_points);
    return all_building_points;
}

pub fn process_building_constructive_element_sequential(
    input_bce: &BuildingConstructiveElement,
) -> Vec<[f64; 3]> {
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    let space = &input_bce.occupied_space.space;

    // Extract from lod3MultiSurface
    if let Some(multi_surface) = &space.lod3_multi_surface {
        all_points.extend(process_multi_surface_sequential(multi_surface));
    }
    // Extract from lod2MultiSurface
    if let Some(multi_surface) = &space.lod2_multi_surface {
        all_points.extend(process_multi_surface_sequential(multi_surface));
    }
    // Extract from lod3Solid
    if let Some(solid) = &space.lod3_solid {
        for point in solid.points() {
            all_points.push([point.x(), point.y(), point.z()]);
        }
    }
    // Extract from lod2Solid
    if let Some(solid) = &space.lod2_solid {
        for point in solid.points() {
            all_points.push([point.x(), point.y(), point.z()]);
        }
    }
    all_points
}

pub fn process_wall_surface_sequential(input_wall_surface: &WallSurface) -> Vec<[f64; 3]> {
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    let multi_surfaces = &input_wall_surface.thematic_surface.lod2_multi_surface;
    if let Some(multi_surface) = multi_surfaces {
        let all_points_tmp = process_multi_surface_sequential(&multi_surface);
        all_points.extend(&all_points_tmp);
    }
    return all_points;
}

pub fn process_roof_surface_sequential(input_roof_surface: &RoofSurface) -> Vec<[f64; 3]> {
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    let multi_surfaces = &input_roof_surface.thematic_surface.lod2_multi_surface;
    if let Some(multi_surface) = multi_surfaces {
        // get the id of the multi surface
        let all_points_tmp = process_multi_surface_sequential(&multi_surface);
        all_points.extend(&all_points_tmp);
    }
    return all_points;
}

pub fn process_ground_surface_sequential(input_ground_surface: &GroundSurface) -> Vec<[f64; 3]> {
    let multi_surfaces = &input_ground_surface.thematic_surface.lod2_multi_surface;
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    if let Some(multi_surface) = multi_surfaces {
        // get the id of the multi surface
        let all_points_tmp = process_multi_surface_sequential(&multi_surface);
        all_points.extend(&all_points_tmp);
    }
    return all_points;
}

pub fn process_multi_surface_sequential(input_multi_surface: &MultiSurface) -> Vec<[f64; 3]> {
    let mut all_points: Vec<[f64; 3]> = Vec::new();
    let surface_members = input_multi_surface.surface_member();
    for surface_member in surface_members {
        let all_points_tmp = process_surface_member_sequential(&surface_member);
        all_points.extend(&all_points_tmp);
    }
    return all_points;
}

pub fn process_surface_member_sequential(input_surface_member: &Polygon) -> Vec<[f64; 3]> {
    let mut all_points: Vec<[f64; 3]> = Vec::new();

    for point in input_surface_member.exterior.points() {
        all_points.push([point.x(), point.y(), point.z()]);
    }
    return all_points;
}
