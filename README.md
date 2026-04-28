# RustCityGML2OBJ

Command-line converter for CityGML (`.gml` / `.xml`) to OBJ (`.obj`) with optional JSON metadata.

Current recommended version is **v3.2** (`atsaad/rustcitygml2obj:v3.2`).

## What It Does

- Converts CityGML 3.0 building geometry to triangulated OBJ output.
- Supports default per-polygon export and grouped export modes.
- Supports IFC-derived CityGML 3.0 files (including TUM `TUM-0507-GML3.gml`) in v3.1+.
- In v3.2, writes CityGML attributes (`class`, `function`, `name`) into JSON metadata when present.

## Usage

```bash
RustCityGML2OBJ \
  --input /absolute/path/to/input \
  --output /absolute/path/to/output
```

Use absolute paths for input and output directories.

## Optional Flags

| Flag | Description |
|------|-------------|
| `--tbw` | Translate each building into a local CRS before triangulation. |
| `--add-bb` | Add small triangle markers for a buffered axis-aligned bounding box. |
| `--add-json` | Write one JSON metadata file per OBJ output file. |
| `--import-bb` | Import external bounding box (currently TODO / not implemented). |
| `--group-sc` | Group output by semantic class (one OBJ per `CityObjectClass`). |
| `--group-scomp` | Group output by semantic component (one OBJ per semantic surface id). |

## Output Modes

- Default (no grouping flags): one OBJ per polygon.
- `--group-sc`: one OBJ per semantic class (for example, `WallSurface`, `RoofSurface`, `BuildingConstructiveElement`).
- `--group-scomp`: one OBJ per semantic component id.

## JSON Metadata (with `--add-json`)

Core metadata includes building id, thematic role, geometry/translation data, and related export context.

Starting with v3.2, the following optional fields are included when available in source CityGML:

- `class`
- `function`
- `name`

These are especially useful for IFC-derived CityGML where `class` carries IFC type values like `IfcWall`, `IfcSlab`, etc.

## Version Matrix

| Tag | Stack | IFC support status | Notes |
|-----|-------|--------------------|-------|
| `v1` | `ecitygml 0.0.1-alpha.8` | No | Original baseline. |
| `v2` | `ecitygml 0.0.1-alpha.8` | Partial (older IFC exports) | Added namespace preprocessing, solid processing, wall overwrite fix. |
| `v3` | `ecitygml 0.0.2-alpha.1` | No (for TUM IFC) | Full library/API migration. |
| `v3.1` | `ecitygml 0.0.2-alpha.1` | Yes | Adds Ring->LinearRing conversion, StringAttribute fix, Surface handling, temp preprocessing file fix. |
| `v3.2` | `ecitygml 0.0.2-alpha.1` | Yes | Adds extraction of `class`, `function`, `name` to JSON metadata. |

## Detailed Comparison: v3.2 vs v1

This section summarizes what changed from the original baseline (`v1`) to the
current recommended release (`v3.2`).

### 1) Library stack and internal API

| Topic | v1 | v3.2 |
|------|----|------|
| Core parser/model stack | `ecitygml/egml 0.0.1-alpha.8` | `ecitygml/egml 0.0.2-alpha.1` |
| Geometry traversal model | Older typed-surface traversal (`WallSurface`, `RoofSurface`, `GroundSurface`) | Generic `iter_city_object()` + `CityObjectGeometry::from_city_object()` |
| Surface representation | `linear_ring` / `href` style structures | `SurfaceKind` enum (`Polygon`, `Surface`, `CompositeSurface`) |

Impact:

- v3.2 can process broader CityGML 3.0 geometry layouts, especially IFC-derived structures.
- v1 is tightly coupled to a narrower representation pattern and misses many real IFC exports.

### 2) IFC-derived CityGML handling

| Problem area | v1 behavior | v3.2 behavior |
|-------------|-------------|---------------|
| Namespace prefixes (`core:`, `gen:`) | Not handled robustly | Preprocessing strips prefixes before parse |
| Missing `<StringAttribute><value>` | Parse error | Empty `<value></value>` injected during preprocessing |
| `gml:Ring` in place of `gml:LinearRing` | Parser panic / unsupported | Rewritten to `gml:LinearRing` before parse |
| `Surface` / `CompositeSurface` point extraction | Can hit library `todo!()` paths | Uses triangulation-based extraction, avoiding unsupported `.points()` paths |
| TUM IFC file (`TUM-0507-GML3.gml`) | Not usable end-to-end | Fully supported in v3.2 |

Impact:

- v3.2 is production-usable for large IFC-converted files where v1 typically produces no useful output.

### 3) Geometry conversion quality and output behavior

| Topic | v1 | v3.2 |
|------|----|------|
| Default output mode | Per-polygon OBJ | Per-polygon OBJ |
| Group by semantic class (`--group-sc`) | Limited / earlier implementation stage | Supported with `CityObjectClass` mapping |
| Group by semantic component (`--group-scomp`) | Limited / earlier implementation stage | Supported and stable |
| Building-wise translation (`--tbw`) | Can fail on IFC files without expected surfaces/envelope | Works with centroid fallback over iterated geometry |
| Buffered bounding box (`--add-bb`) | Older fallback behavior | Uses updated geometry helpers compatible with new surface types |

Impact:

- v3.2 retains the original per-polygon workflow while adding reliable grouped outputs and improved robustness for translation/bbox on complex datasets.

### 4) Metadata and semantics (major new value in v3.2)

| Metadata field | v1 | v3.2 |
|---------------|----|------|
| `thematic_role` | Basic thematic role | Present |
| `class` | Not exported | Exported when available |
| `function` | Not exported | Exported when available |
| `name` | Not exported | Exported when available |

Why this matters:

- IFC-converted models often store useful subtype semantics in `class` (for example: `IfcWall`, `IfcSlab`, `IfcBeam`).
- With v3.2, downstream tools can use meaningful labels instead of treating most surfaces as generic `BuildingConstructiveElement`.

### 5) Scalability and real-data validation

| Validation point | v1 | v3.2 |
|------------------|----|------|
| Large TUM IFC-derived file processing | Fails / incomplete | Successfully processed end-to-end |
| High-output-volume workflows | Not reliable for current IFC export style | Validated in large fan-out runs (OBJ + JSON outputs) |

### 6) Remaining limitations in v3.2

- `--import-bb` is still TODO.
- Some deep interior object extraction depends on upstream `ecitygml` library support.
- Focus remains building-centric (not a full CityGML coverage tool across all domains).

### 7) Practical recommendation

- Use `v3.2` for current CityGML 3.0 and IFC-derived workflows.
- Keep `v1` only for historical reproducibility or baseline comparison.

## CityGML Input Requirements

- CityGML 3.0.
- File extension: `.gml`, `.GML`, `.xml`, or `.XML`.
- Input XML should be valid (for example, pre-check with CityDoctor).

## Current Limitations

- Focus is currently on Buildings and building-related geometry.
- CityGML 1.0 and 2.0 are not supported.
- `--import-bb` is still TODO.
- Some `ecitygml` library limitations remain for deep interior object extraction in certain datasets.

## Tech Stack

- [`ecitygml`](https://docs.rs/ecitygml/latest/ecitygml/) / `ecitygml-io` / `ecitygml-transform`
- [`earcut`](https://github.com/ciscorn/earcut-rs) for triangulation
- `rayon` for parallel processing

## Detailed Change History

For full implementation notes and test results across versions, see:

- `rustcitymlversions/RUSTCITYGML2OBJ_VERSIONS.md`
- `rustcitymlversions/RUSTCITYGML2OBJ_V2_CHANGES.md`

## Contact

For issues and feedback, use GitHub Issues/discussions.
