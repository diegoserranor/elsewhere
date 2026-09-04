//! Turns the Natural Earth 110m land GeoJSON in data/raw/ into the coarse
//! outlines the map draws: outer rings only, simplified, written to
//! data/land.tsv.

use std::error::Error;
use std::fs;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const RAW_PATH: &str = "data/raw/ne_110m_land.geojson";
const OUT_PATH: &str = "data/land.tsv";

/// Rings whose every point is below this are off the bottom of the map crop.
const SOUTH_CUTOFF: f64 = -56.0;

/// Douglas-Peucker tolerance in degrees; the map is about a pixel per degree.
const TOLERANCE: f64 = 0.15;

/// Fewer than this and the ring has no area left worth drawing.
const MIN_POINTS: usize = 4;

const OUT_HEADER: &str = "#lon,lat pairs, one ring per line";

#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    lon: f64,
    lat: f64,
}

/// A nested numeric array as it appears under "coordinates".
#[derive(Debug, PartialEq)]
enum Nested {
    Number(f64),
    List(Vec<Nested>),
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let text = read(Path::new(RAW_PATH))?;
    let polygons = parse_polygons(&text)?;
    let features = polygons.len();

    let outers: Vec<Vec<Point>> = polygons
        .into_iter()
        .map(|mut rings| rings.swap_remove(0))
        .filter(|ring| !is_far_south(ring))
        .collect();

    let before: usize = outers.iter().map(Vec::len).sum();
    let rings: Vec<Vec<Point>> = outers
        .into_iter()
        .map(|ring| simplify(&ring, TOLERANCE))
        .filter(|ring| ring.len() >= MIN_POINTS)
        .collect();
    let after: usize = rings.iter().map(Vec::len).sum();

    let out = render(&rings);
    fs::write(OUT_PATH, &out)?;

    println!("features          {features}");
    println!("rings kept        {}", rings.len());
    println!("points before     {before}");
    println!("points after      {after}");
    println!("bytes             {}", out.len());
    println!("\nemitted {} rings to {OUT_PATH}", rings.len());

    Ok(())
}

/// One line per ring, points separated by spaces. Rings are left open: the
/// closing point a GeoJSON ring repeats is dropped on parse.
fn render(rings: &[Vec<Point>]) -> String {
    let mut out = String::from(OUT_HEADER);
    for ring in rings {
        out.push('\n');
        for (i, point) in ring.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&format!("{:.2},{:.2}", point.lon, point.lat));
        }
    }
    out.push('\n');
    out
}

fn is_far_south(ring: &[Point]) -> bool {
    ring.iter().all(|point| point.lat < SOUTH_CUTOFF)
}

/// Douglas-Peucker over lon/lat as if they were planar; the map projection is
/// equirectangular, so degrees are the units the error is judged in anyway.
fn simplify(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;

    let mut spans = vec![(0, last)];
    while let Some((start, end)) = spans.pop() {
        let mut worst = start;
        let mut worst_distance = 0.0;
        for (i, point) in points.iter().enumerate().take(end).skip(start + 1) {
            let distance = distance_to_segment(*point, points[start], points[end]);
            if distance > worst_distance {
                worst = i;
                worst_distance = distance;
            }
        }
        if worst_distance > tolerance {
            keep[worst] = true;
            spans.push((start, worst));
            spans.push((worst, end));
        }
    }

    points
        .iter()
        .zip(keep)
        .filter(|(_, keep)| *keep)
        .map(|(point, _)| *point)
        .collect()
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let (dx, dy) = (end.lon - start.lon, end.lat - start.lat);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((point.lon - start.lon) * dx + (point.lat - start.lat) * dy) / length_squared)
            .clamp(0.0, 1.0)
    };
    let (nx, ny) = (start.lon + t * dx, start.lat + t * dy);
    ((point.lon - nx).powi(2) + (point.lat - ny).powi(2)).sqrt()
}

/// Every feature's rings, outer first, as GeoJSON orders them. The file is a
/// flat list of Polygon features, so rather than pull in a JSON parser this
/// walks to each "coordinates" key and reads the nested array behind it.
fn parse_polygons(text: &str) -> Result<Vec<Vec<Vec<Point>>>> {
    let geometries = geometry_types(text)?;
    let unexpected: Vec<&str> = geometries
        .iter()
        .copied()
        .filter(|kind| *kind != "Polygon")
        .collect();
    if !unexpected.is_empty() {
        return Err(format!(
            "expected only Polygon geometries, got {}",
            unexpected.join(", ")
        )
        .into());
    }

    let bytes = text.as_bytes();
    let mut polygons = Vec::new();
    for (start, _) in text.match_indices("\"coordinates\"") {
        let mut i = start + "\"coordinates\"".len();
        skip_to_value(bytes, &mut i)?;
        polygons.push(rings(&parse_nested(bytes, &mut i)?, polygons.len() + 1)?);
    }

    if polygons.len() != geometries.len() {
        return Err(format!(
            "{} geometries but {} coordinate arrays",
            geometries.len(),
            polygons.len()
        )
        .into());
    }
    Ok(polygons)
}

/// The "type" of every geometry object, in file order.
fn geometry_types(text: &str) -> Result<Vec<&str>> {
    let mut types = Vec::new();
    for (start, _) in text.match_indices("\"geometry\"") {
        let rest = &text[start..];
        let key = rest
            .find("\"type\"")
            .ok_or_else(|| -> Box<dyn Error> { "geometry without a type".into() })?;
        let value = rest[key + "\"type\"".len()..]
            .split('"')
            .nth(1)
            .ok_or_else(|| -> Box<dyn Error> { "geometry type is not a string".into() })?;
        types.push(value);
    }
    if types.is_empty() {
        return Err("no geometries found".into());
    }
    Ok(types)
}

/// A polygon's rings. GeoJSON repeats the first point last; the repeat is
/// dropped here so the rest of the pipeline never sees it.
fn rings(value: &Nested, feature: usize) -> Result<Vec<Vec<Point>>> {
    let Nested::List(rings) = value else {
        return Err(format!("feature {feature}: coordinates is a number").into());
    };
    if rings.is_empty() {
        return Err(format!("feature {feature}: no rings").into());
    }

    rings
        .iter()
        .map(|ring| {
            let Nested::List(points) = ring else {
                return Err(format!("feature {feature}: ring is a number").into());
            };
            let mut points: Vec<Point> = points
                .iter()
                .map(|point| self::point(point, feature))
                .collect::<Result<_>>()?;
            match (points.first(), points.last()) {
                (Some(first), Some(last)) if first == last => {
                    points.pop();
                    Ok(points)
                }
                _ => Err(format!("feature {feature}: ring is not closed").into()),
            }
        })
        .collect()
}

fn point(value: &Nested, feature: usize) -> Result<Point> {
    match value {
        Nested::List(pair) => match pair.as_slice() {
            [Nested::Number(lon), Nested::Number(lat)] => Ok(Point {
                lon: *lon,
                lat: *lat,
            }),
            _ => Err(format!("feature {feature}: point is not a pair of numbers").into()),
        },
        Nested::Number(n) => Err(format!("feature {feature}: point is the bare number {n}").into()),
    }
}

fn parse_nested(bytes: &[u8], i: &mut usize) -> Result<Nested> {
    skip_whitespace(bytes, i);
    match bytes.get(*i) {
        Some(b'[') => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                skip_whitespace(bytes, i);
                match bytes.get(*i) {
                    Some(b']') => {
                        *i += 1;
                        return Ok(Nested::List(items));
                    }
                    Some(b',') => *i += 1,
                    Some(_) => items.push(parse_nested(bytes, i)?),
                    None => return Err(unterminated(*i)),
                }
            }
        }
        Some(_) => parse_number(bytes, i),
        None => Err(unterminated(*i)),
    }
}

fn parse_number(bytes: &[u8], i: &mut usize) -> Result<Nested> {
    let start = *i;
    while matches!(bytes.get(*i), Some(c) if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E'))
    {
        *i += 1;
    }
    let text = std::str::from_utf8(&bytes[start..*i])?;
    text.parse()
        .map(Nested::Number)
        .map_err(|e| format!("byte {start}: bad number {text:?}: {e}").into())
}

/// Walks past the ":" separating a key from its value.
fn skip_to_value(bytes: &[u8], i: &mut usize) -> Result<()> {
    skip_whitespace(bytes, i);
    if bytes.get(*i) != Some(&b':') {
        return Err(format!("byte {i}: expected \":\" after a key").into());
    }
    *i += 1;
    Ok(())
}

fn skip_whitespace(bytes: &[u8], i: &mut usize) {
    while matches!(bytes.get(*i), Some(c) if c.is_ascii_whitespace()) {
        *i += 1;
    }
}

fn unterminated(i: usize) -> Box<dyn Error> {
    format!("byte {i}: array ends mid-value").into()
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| {
        format!(
            "{}: {e}\nrun scripts/fetch-naturalearth.sh from the repo root to download it",
            path.display()
        )
        .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEATURE: &str = r#"{"type":"FeatureCollection","features":[
{"type":"Feature","properties":{"scalerank":1},"geometry":{"type":"Polygon","coordinates":[
[[0.0,0.0],[2.0,0.0],[2.0,2.0],[0.0,2.0],[0.0,0.0]],
[[0.5,0.5],[1.0,0.5],[1.0,1.0],[0.5,0.5]]]}}]}"#;

    fn point(lon: f64, lat: f64) -> Point {
        Point { lon, lat }
    }

    #[test]
    fn parses_a_polygon_with_a_hole() {
        let polygons = parse_polygons(FEATURE).unwrap();
        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 2);
        assert_eq!(
            polygons[0][0],
            [
                point(0.0, 0.0),
                point(2.0, 0.0),
                point(2.0, 2.0),
                point(0.0, 2.0)
            ]
        );
        assert_eq!(polygons[0][1].len(), 3);
    }

    #[test]
    fn parses_negative_and_exponent_numbers() {
        let text = r#"{"geometry":{"type":"Polygon","coordinates":[[[-66.29,-8.1e1],[1,2],[3,4],[-66.29,-81]]]}}"#;
        let ring = &parse_polygons(text).unwrap()[0][0];
        assert_eq!(ring[0], point(-66.29, -81.0));
        assert_eq!(ring[2], point(3.0, 4.0));
    }

    #[test]
    fn rejects_geometries_that_are_not_polygons() {
        let text =
            r#"{"geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]]]}}"#;
        let err = parse_polygons(text).unwrap_err().to_string();
        assert!(err.contains("got MultiPolygon"), "{err}");
    }

    #[test]
    fn rejects_an_unclosed_ring() {
        let text = r#"{"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1]]]}}"#;
        let err = parse_polygons(text).unwrap_err().to_string();
        assert!(err.contains("ring is not closed"), "{err}");
    }

    #[test]
    fn rejects_points_that_are_not_pairs() {
        let text =
            r#"{"geometry":{"type":"Polygon","coordinates":[[[0,0,7],[1,0],[1,1],[0,0,7]]]}}"#;
        let err = parse_polygons(text).unwrap_err().to_string();
        assert!(err.contains("not a pair of numbers"), "{err}");
    }

    #[test]
    fn rejects_a_truncated_file() {
        let text = r#"{"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0]"#;
        let err = parse_polygons(text).unwrap_err().to_string();
        assert!(err.contains("ends mid-value"), "{err}");
    }

    #[test]
    fn simplify_drops_points_within_the_tolerance() {
        let points = [
            point(0.0, 0.0),
            point(1.0, 0.1),
            point(2.0, -0.1),
            point(3.0, 0.05),
            point(4.0, 0.0),
        ];
        assert_eq!(
            simplify(&points, TOLERANCE),
            [point(0.0, 0.0), point(4.0, 0.0)]
        );
    }

    #[test]
    fn simplify_keeps_a_point_past_the_tolerance() {
        let points = [point(0.0, 0.0), point(1.0, 0.2), point(2.0, 0.0)];
        assert_eq!(simplify(&points, TOLERANCE), points);
    }

    #[test]
    fn simplify_keeps_the_endpoints_of_a_short_ring() {
        let points = [point(0.0, 0.0), point(1.0, 0.0)];
        assert_eq!(simplify(&points, TOLERANCE), points);
    }

    #[test]
    fn simplify_measures_distance_to_the_segment_not_its_line() {
        // The middle point is far along the line through the ends but close to
        // the segment itself, which a line-distance test would keep.
        let points = [point(0.0, 0.0), point(10.0, 0.05), point(0.1, 0.0)];
        assert_eq!(simplify(&points, TOLERANCE), points);
    }

    #[test]
    fn drops_rings_wholly_south_of_the_cutoff() {
        assert!(is_far_south(&[point(0.0, -80.0), point(1.0, -60.0)]));
        assert!(!is_far_south(&[point(0.0, -80.0), point(1.0, -50.0)]));
    }

    #[test]
    fn renders_a_header_and_one_line_per_ring() {
        let rings = vec![
            vec![point(-0.1275, 51.5072), point(2.3522, 48.8566)],
            vec![point(139.6917, 35.6895)],
        ];
        assert_eq!(
            render(&rings),
            "#lon,lat pairs, one ring per line\n-0.13,51.51 2.35,48.86\n139.69,35.69\n"
        );
    }
}
