use crate::geometry_functions::{construct_buffered_bounding_box, triangulate_surface_kind};
use crate::write_functions;
use crate::write_functions::write_obj_file;
use ecitygml_core::model::building::Building;
use ecitygml_core::model::common::CityObjectClass;
use ecitygml_core::model::core::{AsAbstractFeature, CityObjectRef};
use ecitygml_core::operations::CityObjectGeometry;
use egml::model::base::Id;
use egml::model::geometry::primitives::SurfaceKind;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Attributes extracted from CityGML city objects that complement the CityObjectClass.
/// These follow the CityGML 3.0 schema naming: `class`, `function`, and `name`.
#[derive(Debug, Clone, Default)]
pub struct CityObjectAttributes {
    /// The `class` attribute (e.g., "IfcWallStandardCase", "IfcSlab").
    /// Corresponds to `bldg:class` / `con:class` in CityGML.
    pub class: Option<String>,
    /// The `function` attribute(s), joined by "; " if multiple.
    /// Corresponds to `bldg:function` / `con:function` in CityGML.
    pub function: Option<String>,
    /// The `gml:name` value(s), joined by "; " if multiple.
    pub name: Option<String>,
}

/// Extract classification attributes from a CityObjectRef.
/// Works generically for all city object types that carry `class`, `function`,
/// or `gml:name` — including native CityGML 3.0 LOD3 and IFC-derived files.
pub fn extract_city_object_attributes(co_ref: &CityObjectRef) -> CityObjectAttributes {
    let names: &Vec<String> = co_ref.name();
    let name = if names.is_empty() {
        None
    } else {
        Some(names.join("; "))
    };

    let (class, function) = match co_ref {
        CityObjectRef::BuildingConstructiveElement(x) => (
            x.class().as_ref().map(|c| c.value.clone()),
            {
                let fns: Vec<String> = x.functions().iter().map(|c| c.value.clone()).collect();
                if fns.is_empty() { None } else { Some(fns.join("; ")) }
            },
        ),
        CityObjectRef::BuildingInstallation(x) => (
            x.class().as_ref().map(|c| c.value.clone()),
            {
                let fns: Vec<String> = x.functions().iter().map(|c| c.value.clone()).collect();
                if fns.is_empty() { None } else { Some(fns.join("; ")) }
            },
        ),
        CityObjectRef::BuildingRoom(x) => (
            x.class().as_ref().map(|c| c.value.clone()),
            {
                let fns: Vec<String> = x.functions().iter().map(|c| c.value.clone()).collect();
                if fns.is_empty() { None } else { Some(fns.join("; ")) }
            },
        ),
        _ => (None, None),
    };

    CityObjectAttributes { class, function, name }
}

#[derive(Debug, Default)]
struct SurfaceGroup {
    vertices: Vec<[f64; 3]>,
    triangles: Vec<u32>,
    class_name: Option<String>,
}

pub fn collect_building_geometries(
    input_building: &Building,
    tbw: bool,
    add_bb: bool,
    _add_json: bool,
    _import_bb: bool,
    group_by_surface: bool,
    group_by_semantic_surface: bool,
) {
    let mut bbox = (Vec::new(), Vec::new());

    if add_bb {
        bbox = construct_buffered_bounding_box(input_building);
    }

    let building_id = input_building.id();

    let mut dx: f64 = 0.0;
    let mut dy: f64 = 0.0;
    let mut dz: f64 = 0.0;
    if tbw {
        if let Some(envelope) = input_building.bounded_by() {
            let upper_corner = envelope.upper_corner();
            let lower_corner = envelope.lower_corner();
            dx = -((upper_corner.x() + lower_corner.x()) / 2.0);
            dy = -((upper_corner.y() + lower_corner.y()) / 2.0);
            dz = -((upper_corner.z() + lower_corner.z()) / 2.0);
        } else {
            let centroid = crate::translation_module::compute_building_centroid(input_building);
            if let Some((cx, cy, cz)) = centroid {
                dx = -cx;
                dy = -cy;
                dz = -cz;
            } else {
                println!("Envelope hat keine gültigen lower/upper corner Koordinaten.");
            }
        }
    }

    // Collect geometry and classification attributes from all city objects
    let geometry_map: Vec<(Id, CityObjectGeometry, CityObjectAttributes)> = input_building
        .iter_city_object()
        .map(|co_ref| {
            let id = co_ref.id().clone();
            let attrs = extract_city_object_attributes(&co_ref);
            let geom = CityObjectGeometry::from_city_object(co_ref);
            (id, geom, attrs)
        })
        .collect();

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

    geometry_map
        .par_iter()
        .for_each(|(gml_id, geom, attrs)| {
            let class = geom.class;
            let class_key = city_object_class_to_str(class).to_owned();

            // Process multi_surfaces
            for (_lod, multi_surface) in &geom.multi_surfaces {
                for surface_kind in multi_surface.surface_member() {
                    process_surface_kind(
                        surface_kind,
                        building_id,
                        class,
                        dx, dy, dz,
                        &bbox,
                        gml_id,
                        class_key.clone(),
                        groups_by_class.clone(),
                        groups_by_semantic_surface.clone(),
                        attrs,
                    );
                }
            }

            // Process solids
            for (_lod, solid) in &geom.solids {
                for surface_property in solid.members() {
                    process_surface_kind(
                        &surface_property.content,
                        building_id,
                        class,
                        dx, dy, dz,
                        &bbox,
                        gml_id,
                        class_key.clone(),
                        groups_by_class.clone(),
                        groups_by_semantic_surface.clone(),
                        attrs,
                    );
                }
            }
        });

    // Write grouped OBJ files (semantic class level)
    if let Some(groups_arc) = groups_by_class {
        let map = Arc::try_unwrap(groups_arc)
            .expect("Unexpected Arc reference count")
            .into_inner()
            .unwrap();

        for (class_key, group) in map {
            let filename = format!("{}_{}.obj", building_id, class_key);
            let empty_attrs = CityObjectAttributes::default();
            write_obj_file(
                group.vertices,
                group.triangles,
                building_id,
                write_functions::SemanticSurfaceId::Str(&filename),
                &class_key,
                dx, dy, dz,
                &bbox,
                &Id::from_hashed_string("grouped"),
                &Id::from_hashed_string("grouped"),
                &empty_attrs,
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
            let empty_attrs = CityObjectAttributes::default();
            write_obj_file(
                group.vertices,
                group.triangles,
                building_id,
                write_functions::SemanticSurfaceId::Str(&filename),
                class_name,
                dx, dy, dz,
                &bbox,
                &Id::from_hashed_string("grouped"),
                &Id::from_hashed_string(&filename),
                &empty_attrs,
            );
        }
    }
}

pub fn process_surface_kind(
    surface_kind: &SurfaceKind,
    building_id: &Id,
    thematic_info: CityObjectClass,
    dx: f64,
    dy: f64,
    dz: f64,
    bbox: &(Vec<[f64; 3]>, Vec<[u64; 3]>),
    gml_id: &Id,
    class_key: String,
    groups_by_class: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
    groups_by_semantic_surface: Option<Arc<Mutex<HashMap<String, SurfaceGroup>>>>,
    attrs: &CityObjectAttributes,
) {
    let (triangles, all_points, surface_id) = triangulate_surface_kind(surface_kind);

    if triangles.is_empty() {
        return;
    }

    // Semantic surface grouping
    if let Some(groups_arc) = groups_by_semantic_surface {
        let mut map = groups_arc.lock().unwrap();
        let bucket = map.entry(surface_id.to_string()).or_insert_with(|| {
            let mut g = SurfaceGroup::default();
            g.class_name = Some(class_key.clone());
            g
        });

        let vertex_offset = bucket.vertices.len() as u32;
        bucket.vertices.extend_from_slice(&all_points);
        let shifted: Vec<u32> = triangles.into_iter().map(|idx| idx + vertex_offset).collect();
        bucket.triangles.extend(shifted);
    }
    // Semantic class grouping
    else if let Some(groups_arc) = groups_by_class {
        let mut map = groups_arc.lock().unwrap();
        let bucket = map.entry(class_key).or_default();

        let vertex_offset = bucket.vertices.len() as u32;
        bucket.vertices.extend_from_slice(&all_points);
        let shifted: Vec<u32> = triangles.into_iter().map(|idx| idx + vertex_offset).collect();
        bucket.triangles.extend(shifted);
    }
    // Per-polygon output
    else {
        let thematic_info_string = city_object_class_to_str(thematic_info);
        write_obj_file(
            all_points,
            triangles,
            building_id,
            write_functions::SemanticSurfaceId::Id(&surface_id),
            thematic_info_string,
            dx, dy, dz,
            bbox,
            gml_id,
            &surface_id,
            attrs,
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
        CityObjectClass::Storey => "Storey",
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
        _ => "Unknown",
    }
}
