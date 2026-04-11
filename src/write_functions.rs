use crate::conversion_functions::CityObjectAttributes;
use crate::Args;
use clap::Parser;
use egml::model::base::Id;
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug)]
pub enum SemanticSurfaceId<'a> {
    Id(&'a Id),
    Str(&'a str),
    None,
}

impl<'a> ToString for SemanticSurfaceId<'a> {
    fn to_string(&self) -> String {
        match self {
            SemanticSurfaceId::Id(id) => id.to_string(),
            SemanticSurfaceId::Str(s) => s.to_string(),
            SemanticSurfaceId::None => String::new(),
        }
    }
}

#[derive(Serialize)]
struct Metadata {
    building_id: String,
    class_gml_id: String,
    multi_surface_gml_id: String,
    polygon_gml_id: String,
    thematic_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    dx: String,
    dy: String,
    dz: String,
}

pub fn write_json_metadata(
    building_id: &Id,
    semantic_surface_id: SemanticSurfaceId,
    thematic_role: &str,
    output_dir: &str,
    dx: f64,
    dy: f64,
    dz: f64,
    gml_id: &Id,
    stuff_gml_id: &Id,
    attrs: &CityObjectAttributes,
) {
    let mut semantic_surface_string: String = "default".to_string();

    match semantic_surface_id {
        SemanticSurfaceId::Id(id) => {
            semantic_surface_string = semantic_surface_id.to_string();
        }
        SemanticSurfaceId::Str(s) => {
            semantic_surface_string = semantic_surface_id.to_string();
        }
        SemanticSurfaceId::None => {
            semantic_surface_string = "No_semantic_surface_sting_found".to_string();
        }
    }
    let metadata = Metadata {
        building_id: building_id.to_string(),
        class_gml_id: gml_id.to_string(),
        multi_surface_gml_id: stuff_gml_id.to_string(),
        polygon_gml_id: semantic_surface_string,
        thematic_role: thematic_role.to_string(),
        class: attrs.class.clone(),
        function: attrs.function.clone(),
        name: attrs.name.clone(),
        dx: dx.to_string(),
        dy: dy.to_string(),
        dz: dz.to_string(),
    };

    let filename = format!("{}.json", metadata.polygon_gml_id);

    let file_path = Path::new(output_dir).join(filename);

    let file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create JSON file: {}", e);
            return;
        }
    };

    let writer = BufWriter::new(file);
    if let Err(e) = serde_json::to_writer_pretty(writer, &metadata) {
        eprintln!("Failed to write JSON metadata: {}", e);
    }
}

pub fn import_bbox_from_file() {
    // todo: muss noch implementiert werden
}

pub fn write_obj_file(
    input_points: Vec<[f64; 3]>,
    triangles: Vec<u32>,
    building_id: &Id,
    semantic_surface_id: SemanticSurfaceId,
    thematic_role: &str,
    dx: f64,
    dy: f64,
    dz: f64,
    bbox: &(Vec<[f64; 3]>, Vec<[u64; 3]>),
    gml_id: &Id,
    stuff_gml_id: &Id,
    attrs: &CityObjectAttributes,
) {
    let args = Args::parse();
    let building_id_string = building_id.to_string();
    let mut semantic_surface_string: String = "default".to_string();

    match semantic_surface_id {
        SemanticSurfaceId::Id(id) => {
            semantic_surface_string = semantic_surface_id.to_string();
        }
        SemanticSurfaceId::Str(s) => {
            semantic_surface_string = semantic_surface_id.to_string();
        }
        SemanticSurfaceId::None => {
            semantic_surface_string = "No_semantic_surface_sting_found".to_string();
        }
    }

    let filename = format!("{}.obj", semantic_surface_string);
    let file_path = Path::new(&args.output).join(filename);

    let file = match File::create(&file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create file: {}", e);
            return;
        }
    };

    let mut writer = BufWriter::new(file);

    // Write main geometry vertices
    for point in &input_points {
        if let Err(e) = writeln!(writer, "v {} {} {}", point[0], point[1], point[2]) {
            eprintln!("Failed to write vertex: {}", e);
            return;
        }
    }

    let base_vertex_count = input_points.len();

    // Write triangle faces
    if triangles.len() % 3 != 0 {
        eprintln!("Triangle index list is not a multiple of 3.");
        return;
    }

    for face in triangles.chunks(3) {
        if let Err(e) = writeln!(writer, "f {} {} {}", face[0] + 1, face[1] + 1, face[2] + 1) {
            eprintln!("Failed to write face: {}", e);
            return;
        }
    }

    if args.add_bb {
        // Write bounding box vertices
        let (bbox_vertices, bbox_triangles) = bbox;
        for point in bbox_vertices {
            if let Err(e) = writeln!(writer, "v {} {} {}", point[0], point[1], point[2]) {
                eprintln!("Failed to write bbox vertex: {}", e);
                return;
            }
        }

        // Write bounding box edges
        let bbox_vertex_offset = base_vertex_count;
        // Write bounding box triangles
        for face in bbox_triangles {
            if let Err(e) = writeln!(
                writer,
                "f {} {} {}",
                bbox_vertex_offset + face[0] as usize + 1,
                bbox_vertex_offset + face[1] as usize + 1,
                bbox_vertex_offset + face[2] as usize + 1,
            ) {
                eprintln!("Failed to write bbox triangle: {}", e);
                return;
            }
        }
    }

    if args.add_json {
        write_json_metadata(
            building_id,
            semantic_surface_id,
            thematic_role,
            &args.output,
            dx,
            dy,
            dz,
            gml_id,
            stuff_gml_id,
            attrs,
        );
    }
}
