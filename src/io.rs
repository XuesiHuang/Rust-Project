//! I/O module: parses CSV rows with WKT geometries into road segments, skipping invalid or unnamed segments.

use csv::Reader;
use geo_types::Geometry;
use std::{error::Error, path::Path};
use wkt::Wkt;

/// Represents a single road segment defined by its endpoints and street name.
///
/// This struct holds the street name (guaranteed non-empty) and the start/end coordinates parsed from WKT.
pub struct Segment {
    /// Street name for the segment
    pub name:  String,
    /// Start point of the segment (longitude, latitude)
    pub start: (f64, f64),
    /// End point of the segment (longitude, latitude)
    pub end:   (f64, f64),
}

/// Load all valid `Segment`s from a CSV file located at `path`.
///
/// Inputs:
/// `path`: Path to the CSV file with `shape_wkt` and `STREETNAME` columns.
///
/// Outputs:
/// `Ok(Vec<Segment>)` on success
/// `Err` if file I/O fails or required columns are missing
///
/// High-level logic:
/// 1. Read headers and find indices for geometry and street name.
/// 2. Iterate through each record:
///    - Skip if the street name is empty after trimming.
///    - Parse the WKT string into a WKT object, then convert to a geo_types geometry.
///    - Only handle `MultiLineString` geometries: for each linestring, create a segment from its first to last coordinate.
///    - Collect these segments into a vector.
pub fn load_segments(path: &Path) -> Result<Vec<Segment>, Box<dyn Error>> {
    // Initialize CSV reader
    let mut rdr = Reader::from_path(path)?;
    // Clone headers to locate columns
    let headers = rdr.headers()?.clone();
    // Index of the WKT geometry column
    let wkt_idx = headers
        .iter()
        .position(|h| h == "shape_wkt")
        .ok_or("missing shape_wkt column")?;
    // Index of the street name column
    let name_idx = headers
        .iter()
        .position(|h| h == "STREETNAME")
        .ok_or("missing STREETNAME column")?;

    let mut segments = Vec::new();
    // Process each CSV record
    for record in rdr.records() {
        let rec = record?;
        // Extract street name and skip if blank
        let raw = rec.get(name_idx).unwrap().trim();
        if raw.is_empty() {
            continue;
        }
        let street = raw.to_string();

        // If there's a geometry string, attempt to parse it
        if let Some(wkt_str) = rec.get(wkt_idx) {
            if let Ok(wkt_obj) = wkt_str.parse::<Wkt<f64>>() {
                if let Ok(geo) = Geometry::try_from(wkt_obj) {
                    // Only process MultiLineStrings
                    if let Geometry::MultiLineString(mls) = geo {
                        // Each LineString becomes one segment
                        for ls in mls.0 {
                            let coords = ls.0;
                            // Require at least two points to form a segment
                            if coords.len() >= 2 {
                                // Safe unwrap: coords.len() >= 2
                                let a = coords.first().unwrap();
                                let b = coords.last().unwrap();
                                segments.push(Segment {
                                    name:  street.clone(),
                                    start: (a.x, a.y),
                                    end:   (b.x, b.y),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that a MULTILINESTRING WKT is parsed into two LineStrings within the geometry.
    #[test]
    fn test_parse_named_wkt() {
        let wkt_str = "MULTILINESTRING ((0 0,1 1),(2 2,3 3))";
        // Parse into WKT structure
        let wkt = wkt_str.parse::<Wkt<f64>>().unwrap();
        // Convert to geo_types Geometry
        let geo = Geometry::try_from(wkt).unwrap();
        // Assert that we got a MultiLineString with exactly 2 components
        if let Geometry::MultiLineString(mls) = geo {
            assert_eq!(mls.0.len(), 2);
        } else {
            panic!("Expected MultiLineString variant");
        }
    }
}
