mod conversion_functions;
mod geometry_functions;
mod translation_module;
mod write_functions;

use clap::Parser;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::Path;

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

    // Only preprocess if we detect prefixed lod/boundary tags
    if !content.contains("<core:lod") && !content.contains("<core:boundary") {
        return path.to_path_buf();
    }

    println!("  Preprocessing: stripping namespace prefixes for parser compatibility");

    let mut result = content;
    // Strip core: prefix from lod and boundary tags (opening and closing)
    for tag in &[
        "lod0MultiSurface", "lod1Solid", "lod2Solid", "lod3Solid",
        "lod0MultiSurface", "lod2MultiSurface", "lod3MultiSurface",
        "lod1ImplicitRepresentation", "lod2ImplicitRepresentation",
        "lod3ImplicitRepresentation", "boundary",
    ] {
        result = result.replace(
            &format!("<core:{}", tag),
            &format!("<{}", tag),
        );
        result = result.replace(
            &format!("</core:{}", tag),
            &format!("</{}", tag),
        );
    }

    // Write to a temp file next to the original
    let temp_path = path.with_extension("preprocessed.gml");
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

    // Read directory entries
    let input_path = Path::new(&args.input);
    let entries = fs::read_dir(input_path).expect("Could not read input directory");

    // Filter and process each matching file
    for entry in entries.flatten() {
        let path = entry.path();

        // Check if the file ends with a valid extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase();
            if ext == "gml" || ext == "xml" {
                println!("Processing file: {}", path.display());

                // Preprocess: strip namespace prefixes from CityGML 3.0 core/bldg/con
                // elements so the egml parser can match them (it expects unprefixed tags)
                let effective_path = preprocess_gml(&path);
                let reader_result = ecitygml_io::CitygmlReader::from_path(&effective_path);

                match reader_result.unwrap().finish() {
                    Ok(mut data) => {
                        let all_buildings = &mut data.building;

                        all_buildings.par_iter_mut().for_each(|building| {
                            conversion_functions::collect_building_geometries(
                                building,
                                args.tbw,
                                args.add_bb,
                                args.add_json,
                                args.import_bb,
                                args.group_sc,
                                args.group_scomp,
                            );
                        });
                    }
                    Err(e) => {
                        eprintln!("Error reading file {}: {:?}", path.display(), e);
                    }
                }

                // Clean up preprocessed temp file
                if effective_path != path {
                    let _ = fs::remove_file(&effective_path);
                }
            }
        }
    }
}
