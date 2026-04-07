use crate::geometry_functions::{construct_buffered_bounding_box, triangulate};
use crate::write_functions;
use crate::write_functions::write_obj_file;
use ecitygml::operations::GeometryCollector;
use ecitygml_core::model::building::Building;
use ecitygml_core::model::common::{CityObjectClass, LevelOfDetail};
use ecitygml_core::operations::{FeatureWithGeometry, Visitable};
use egml::model::base::{Gml, Id};
use egml::model::geometry::{MultiSurface, Polygon};
use egml::operations::triangulate::Triangulate;
use egml::operations::geometry::Geometry;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Helper container that stores all vertices and triangles that belong to one
// semantic surface class (e.g. WallSurface, RoofSurface, …).
#[derive(Debug, Default)]
struct SurfaceGroup {
    vertices: Vec<[f64; 3]>,
    triangles: Vec<u32>,
    class_name: Option<String>,
}

pub fn collect_building_geometries(
    input_building: &mut Building,
    tbw: bool,
    add_bb: bool,
    add_json: bool,
    import_bb: bool,
    group_by_surface: bool,
    group_by_semantic_surface: bool,
) {
    // Initialize an empty bounding box
    let mut bbox = (Vec::new(), Vec::new());

    // Distinguish the different cases of the bounding box
    if add_bb {
        bbox = construct_buffered_bounding_box(input_building);
    } else if import_bb {
        // Import the bounding box from an external file
        // todo: Muss noch implementiert werden.
    }

    let building_id = &input_building.occupied_space.space.city_object.gml.id;

    // get the translation parameter into a local crs in case it is desired
    let mut dx: f64 = 0.0;
    let mut dy: f64 = 0.0;
    let mut dz: f64 = 0.0;
    if tbw {
        if let Some(envelope) = input_building.envelope() {
            let upper_corner = envelope.upper_corner();
            let lower_corner = envelope.lower_corner();
            dx = -((upper_corner.x() + lower_corner.x()) / 2.0);
            dy = -((upper_corner.y() + lower_corner.y()) / 2.0);
            dz = -((upper_corner.z() + lower_corner.z()) / 2.0);
        } else {
            println!("Envelope hat keine gültigen lower/upper corner Koordinaten.");
        }
    }

    let mut collector_1 = GeometryCollector::new();
    input_building.accept(&mut collector_1);

    // Prepare optional shared accumulators
    let groups_by_class: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>> = if group_by_surface {
        Some(Arc::new(Mutex::new(HashMap::new())))
    } else {
        None
    };

    let groups_by_semantic_surface: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>> =
        if group_by_semantic_surface {
            Some(Arc::new(Mutex::new(HashMap::new())))
        } else {
            None
        };

    collector_1
        .city_objects
        .par_iter()
        .for_each(|collected_geometry| {
            let gml_id = &collected_geometry.1.gml.id;
            let class = collected_geometry.1.class;
            let multi_surfaces = &collected_geometry.1.multi_surfaces;

            let class_key = city_object_class_to_str(class).to_owned();
            let solids = &collected_geometry.1.solids;

            for multi_surface in multi_surfaces {
                process_multi_surface(
                    &multi_surface,
                    building_id,
                    class,
                    dx,
                    dy,
                    dz,
                    &bbox,
                    gml_id,
                    groups_by_class.clone(),
                    groups_by_semantic_surface.clone(),
                    class_key.clone(),
                );
            }

            // Process solids (e.g. from BuildingConstructiveElement with lod3Solid)
            for solid in solids {
                let solid_obj = solid.1;
                for surface_property in solid_obj.members() {
                    if let Some(ring) = &surface_property.linear_ring {
                        if let Ok(polygon) = Polygon::new(
                            Gml::new(Id::from_hashed_string(&surface_property.href)),
                            ring.clone(),
                            Vec::new(),
                        ) {
                            process_surface_member(
                                &polygon,
                                building_id,
                                &solid_obj.gml.id,
                                class,
                                dx,
                                dy,
                                dz,
                                &bbox,
                                gml_id,
                                class_key.clone(),
                                groups_by_class.clone(),
                                groups_by_semantic_surface.clone(),
                            );
                        }
                    }
                }
            }
        });

    //  Write grouped OBJ files (semantic class level)
    if let Some(groups_arc) = groups_by_class {
        let map = Arc::try_unwrap(groups_arc)
            .expect("Unexpected Arc reference count")
            .into_inner()
            .unwrap();

        for (class_key, group) in map {
            let filename = format!("{}_{}.obj", building_id, class_key);

            write_obj_file(
                group.vertices,
                group.triangles,
                building_id,
                write_functions::SemanticSurfaceId::Str(&filename),
                &class_key,
                dx,
                dy,
                dz,
                &bbox,
                &Id::from_hashed_string("grouped"),
                &Id::from_hashed_string("grouped"),
            );
        }
    }

    // Write grouped OBJ files (semantic surface level)
    if let Some(groups_arc) = groups_by_semantic_surface {
        let map = Arc::try_unwrap(groups_arc)
            .expect("Unexpected Arc reference count")
            .into_inner()
            .unwrap();

        for (surface_id, group) in map {
            let class_name = group.class_name.as_deref().unwrap_or("UnknownSurface");
            let filename = format!("{}_{}_{}", building_id, class_name, surface_id);
            println!("filename {}", class_name);
            write_obj_file(
                group.vertices,
                group.triangles,
                building_id,
                write_functions::SemanticSurfaceId::Str(&filename),
                &class_name,
                dx,
                dy,
                dz,
                &bbox,
                &Id::from_hashed_string("grouped"),
                &Id::from_hashed_string(&filename),
            );
        }
    }

    if add_json {
        // TODO: muss noch implementiert werden
    }
}

pub fn process_multi_surface(
    input_multi_surface: &(&LevelOfDetail, &MultiSurface),
    building_id: &Id,
    class: CityObjectClass,
    dx: f64,
    dy: f64,
    dz: f64,
    bbox: &(Vec<[f64; 3]>, Vec<[u64; 3]>),
    gml_id: &Id,
    groups_by_class: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
    groups_by_semantic_surface: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
    class_key: String,
) {
    let stuffs = &input_multi_surface.1.surface_member();
    let stuff_gml_id = &input_multi_surface.1.gml.id;
    stuffs.par_iter().for_each(|surface_member| {
        process_surface_member(
            surface_member,
            building_id,
            stuff_gml_id,
            class,
            dx,
            dy,
            dz,
            bbox,
            gml_id,
            class_key.clone(),
            groups_by_class.clone(),
            groups_by_semantic_surface.clone(),
        );
    });
}

pub fn process_surface_member(
    input_surface_member: &Polygon,
    building_id: &Id,
    multi_surface_id: &Id,
    thematic_info: CityObjectClass,
    dx: f64,
    dy: f64,
    dz: f64,
    bbox: &(Vec<[f64; 3]>, Vec<[u64; 3]>),
    gml_id: &Id,
    class_key: String,
    groups_by_class: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
    groups_by_semantic_surface: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
) {
    let (triangles, all_points) = triangulate(input_surface_member);
    let surface_id = input_surface_member.gml.id.clone();

    // Semantic surface grouping
    if let Some(groups_arc) = groups_by_semantic_surface {
        let mut map = groups_arc.lock().unwrap();

        // Create or fetch the group for this semantic surface
        let bucket = map.entry(surface_id.to_string()).or_insert_with(|| {
            let mut g = SurfaceGroup::default();
            g.class_name = Some(class_key.clone());
            g
        });

        let vertex_offset = bucket.vertices.len() as u32;
        bucket.vertices.extend_from_slice(&all_points);

        let shifted: Vec<u32> = triangles
            .into_iter()
            .map(|idx| idx + vertex_offset)
            .collect();
        bucket.triangles.extend(shifted);
    }
    // Existing: semantic class grouping
    else if let Some(groups_arc) = groups_by_class {
        let mut map = groups_arc.lock().unwrap();
        let bucket = map.entry(class_key).or_default();

        let vertex_offset = bucket.vertices.len() as u32;
        bucket.vertices.extend_from_slice(&all_points);

        let shifted: Vec<u32> = triangles
            .into_iter()
            .map(|idx| idx + vertex_offset)
            .collect();
        bucket.triangles.extend(shifted);
    }
    // per-polygon output
    else {
        let thematic_info_string = city_object_class_to_str(thematic_info);
        write_obj_file(
            all_points,
            triangles,
            building_id,
            write_functions::SemanticSurfaceId::Id(&surface_id),
            &thematic_info_string,
            dx,
            dy,
            dz,
            bbox,
            gml_id,
            multi_surface_id,
        );
    }
}

pub fn city_object_class_to_str(class: CityObjectClass) -> &'static str {
    match class {
        CityObjectClass::AuxiliaryTrafficArea => "AuxiliaryTrafficArea",
        CityObjectClass::AuxiliaryTrafficSpace => "AuxiliaryTrafficSpace",
        CityObjectClass::Bridge => "Bridge",
        CityObjectClass::BridgeConstructiveElement => "BridgeConstructiveElement",
        CityObjectClass::BridgeFurniture => "BridgeFurniture",
        CityObjectClass::BridgeInstallation => "BridgeInstallation",
        CityObjectClass::BridgePart => "BridgePart",
        CityObjectClass::BridgeRoom => "BridgeRoom",
        CityObjectClass::Building => "Building",
        CityObjectClass::BuildingConstructiveElement => "BuildingConstructiveElement",
        CityObjectClass::BuildingFurniture => "BuildingFurniture",
        CityObjectClass::BuildingInstallation => "BuildingInstallation",
        CityObjectClass::BuildingPart => "BuildingPart",
        CityObjectClass::BuildingRoom => "BuildingRoom",
        CityObjectClass::BuildingUnit => "BuildingUnit",
        CityObjectClass::CeilingSurface => "CeilingSurface",
        CityObjectClass::CityFurniture => "CityFurniture",
        CityObjectClass::CityObjectGroup => "CityObjectGroup",
        CityObjectClass::ClearanceSpace => "ClearanceSpace",
        CityObjectClass::Door => "Door",
        CityObjectClass::DoorSurface => "DoorSurface",
        CityObjectClass::FloorSurface => "FloorSurface",
        CityObjectClass::GenericLogicalSpace => "GenericLogicalSpace",
        CityObjectClass::GenericOccupiedSpace => "GenericOccupiedSpace",
        CityObjectClass::GenericThematicSurface => "GenericThematicSurface",
        CityObjectClass::GenericUnoccupiedSpace => "GenericUnoccupiedSpace",
        CityObjectClass::GroundSurface => "GroundSurface",
        CityObjectClass::Hole => "Hole",
        CityObjectClass::HoleSurface => "HoleSurface",
        CityObjectClass::HollowSpace => "HollowSpace",
        CityObjectClass::InteriorWallSurface => "InteriorWallSurface",
        CityObjectClass::Intersection => "Intersection",
        CityObjectClass::Marking => "Marking",
        CityObjectClass::OtherConstruction => "OtherConstruction",
        CityObjectClass::OuterCeilingSurface => "OuterCeilingSurface",
        CityObjectClass::OuterFloorSurface => "OuterFloorSurface",
        CityObjectClass::PlantCover => "PlantCover",
        CityObjectClass::Railway => "Railway",
        CityObjectClass::Road => "Road",
        CityObjectClass::RoofSurface => "RoofSurface",
        CityObjectClass::Section => "Section",
        CityObjectClass::SolitaryVegetationObject => "SolitaryVegetationObject",
        CityObjectClass::Square => "Square",
        CityObjectClass::Story => "Story",
        CityObjectClass::Track => "Track",
        CityObjectClass::TrafficArea => "TrafficArea",
        CityObjectClass::TrafficSpace => "TrafficSpace",
        CityObjectClass::Tunnel => "Tunnel",
        CityObjectClass::TunnelConstructiveElement => "TunnelConstructiveElement",
        CityObjectClass::TunnelFurniture => "TunnelFurniture",
        CityObjectClass::TunnelInstallation => "TunnelInstallation",
        CityObjectClass::TunnelPart => "TunnelPart",
        CityObjectClass::WallSurface => "WallSurface",
        CityObjectClass::WaterBody => "WaterBody",
        CityObjectClass::WaterGroundSurface => "WaterGroundSurface",
        CityObjectClass::WaterSurface => "WaterSurface",
        CityObjectClass::Waterway => "Waterway",
        CityObjectClass::Window => "Window",
        CityObjectClass::WindowSurface => "WindowSurface",
    }
}
