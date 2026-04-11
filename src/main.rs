mod conversion_functions;
mod geometry_functions;
mod translation_module;
mod write_functions;

use clap::Parser;
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::Path;

use ecitygml_core::model::core::CityObjectKind;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Input file path
    #[arg(short, long)]
    input: String,

    // Output directory
    #[arg(short, long)]
    output: String,

    // Option for invoking building-wise translation into local CRS
    #[arg(long, default_value_t = false)]
    tbw: bool,

    // Option for additionally writing out a json file containing metadata
    #[arg(long, default_value_t = false)]
    add_json: bool,

    // Option for adding the bounding box to the obj files
    #[arg(long, default_value_t = false)]
    add_bb: bool,

    // Option for importing a bounding box instead of creating a new one from the data
    #[arg(long, default_value_t = false)]
    import_bb: bool,

    // Option for grouping the polygons by semantic surfaces
    #[arg(long, default_value_t = false)]
    group_sc: bool,

    // Option for grouping the polygons by semantic surfaces
    #[arg(long, default_value_t = false)]
    group_scomp: bool,
}

/// Preprocess a GML file to strip namespace prefixes that the egml parser
/// doesn't handle (e.g. `<core:lod3Solid>` → `<lod3Solid>`).
/// Returns the original path if no changes needed, or a temp file path.
fn preprocess_gml(path: &Path) -> std::path::PathBuf {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return path.to_path_buf(),
    };

    // Strip namespace prefixes that the parser doesn't handle
    let prefixes_to_strip = ["core:", "gen:"];
    let needs_preprocessing = prefixes_to_strip.iter().any(|p| content.contains(&format!("<{}", p)));

    if !needs_preprocessing {
        return path.to_path_buf();
    }

    println!("  Preprocessing: stripping namespace prefixes for parser compatibility");

    let mut result = content;
    for prefix in &prefixes_to_strip {
        result = result.replace(&format!("<{}", prefix), "<");
        result = result.replace(&format!("</{}", prefix), "</");
    }

    // Fix StringAttributes that are missing a <value> element (IFC exports sometimes omit it)
    let re = Regex::new(r"(<StringAttribute>\s*<name>[^<]*</name>\s*)(</StringAttribute>)").unwrap();
    result = re.replace_all(&result, "${1}<value></value>\n${2}").to_string();

    // Convert gml:Ring (curveMember/LineString) to gml:LinearRing (the library doesn't support Ring yet)
    let ring_re = Regex::new(r"(?s)<gml:Ring>\s*((?:<gml:curveMember>\s*<gml:LineString[^>]*>\s*<gml:posList>([^<]*)</gml:posList>\s*</gml:LineString>\s*</gml:curveMember>\s*)+)</gml:Ring>").unwrap();
    let poslist_re = Regex::new(r"<gml:posList>([^<]*)</gml:posList>").unwrap();
    let mut new_result = String::with_capacity(result.len());
    let mut last_end = 0;
    for cap in ring_re.captures_iter(&result) {
        let m = cap.get(0).unwrap();
        new_result.push_str(&result[last_end..m.start()]);
        // Collect all posList coordinates
        let mut all_coords = String::new();
        for pos_cap in poslist_re.captures_iter(&cap[1]) {
            if !all_coords.is_empty() {
                all_coords.push(' ');
            }
            all_coords.push_str(pos_cap[1].trim());
        }
        new_result.push_str(&format!(
            "<gml:LinearRing><gml:posList>{}</gml:posList></gml:LinearRing>",
            all_coords
        ));
        last_end = m.end();
    }
    new_result.push_str(&result[last_end..]);
    result = new_result;

    // Write to system temp dir so we don't pollute the input directory
    // (and so the main loop doesn't pick it up as an input file)
    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("input");
    let temp_path = std::env::temp_dir().join(format!("{}.preprocessed.gml", file_stem));
    if let Ok(mut f) = fs::File::create(&temp_path) {
        if f.write_all(result.as_bytes()).is_ok() {
            return temp_path;
        }
    }

    path.to_path_buf()
}

fn main() {
    let args = Args::parse();
    println!("Input Directory: {}", args.input);
    println!("Output Directory: {}", args.output);
    println!("translate buildings into local crs: {}", args.tbw);
    println!("add bounding box: {}", args.add_json);
    println!("import bounding box: {}", args.import_bb);
    println!("group output by semantic class: {}", args.group_sc);
    println!("group output by semantic component: {}", args.group_scomp);

    let input_path = Path::new(&args.input);
    let entries = fs::read_dir(input_path).expect("Could not read input directory");

    for entry in entries.flatten() {
        let path = entry.path();

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase();
            if ext == "gml" || ext == "xml" {
                println!("Processing file: {}", path.display());

                let effective_path = preprocess_gml(&path);
                let reader_result = ecitygml_io::GmlReader::from_path(&effective_path);

                match reader_result.unwrap().finish() {
                    Ok(data) => {
                        // Extract buildings from the city model
                        for city_object in &data.city_objects {
                            if let CityObjectKind::Building(building) = city_object {
                                conversion_functions::collect_building_geometries(
                                    building,
                                    args.tbw,
                                    args.add_bb,
                                    args.add_json,
                                    args.import_bb,
                                    args.group_sc,
                                    args.group_scomp,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading file {}: {:?}", path.display(), e);
                    }
                }

                if effective_path != path {
                    let _ = fs::remove_file(&effective_path);
                }
            }
        }
    }
}
