//! Parses the GeoNames dumps in data/raw/ and reports stats about them.
//! Filtering and emitting the stripped dataset comes later.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::str::FromStr;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const RAW_DIR: &str = "data/raw";
const CITY_COLUMNS: usize = 19;
const COUNTRY_COLUMNS: usize = 19;
const ADMIN1_COLUMNS: usize = 4;

/// The columns of cities5000.txt we keep; the rest are dropped on parse.
#[allow(dead_code)] // most fields are only read once the dataset is emitted
#[derive(Debug)]
struct RawCity {
    geonameid: u32,
    name: String,
    asciiname: String,
    alternatenames: Vec<String>,
    latitude: f64,
    longitude: f64,
    feature_class: char,
    feature_code: String,
    country_code: String,
    admin1_code: String,
    population: u64,
    timezone: String,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let raw = Path::new(RAW_DIR);

    let cities = parse_cities(&read(&raw.join("cities5000.txt"))?)?;
    let countries = parse_countries(&read(&raw.join("countryInfo.txt"))?)?;
    let admin1 = parse_admin1(&read(&raw.join("admin1CodesASCII.txt"))?)?;

    let mut per_feature: HashMap<&str, usize> = HashMap::new();
    let mut timezones: HashSet<&str> = HashSet::new();
    let mut empty_admin1 = 0;
    let mut unknown_countries: HashSet<&str> = HashSet::new();
    for city in &cities {
        *per_feature.entry(&city.feature_code).or_default() += 1;
        timezones.insert(&city.timezone);
        if city.admin1_code.is_empty() {
            empty_admin1 += 1;
        }
        if !countries.contains_key(city.country_code.as_str()) {
            unknown_countries.insert(&city.country_code);
        }
    }

    let mut per_feature: Vec<_> = per_feature.into_iter().collect();
    per_feature.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    println!("rows                  {}", cities.len());
    println!("countries             {}", countries.len());
    println!("admin1 codes          {}", admin1.len());
    println!("distinct timezones    {}", timezones.len());
    println!("empty admin1 codes    {empty_admin1}");
    println!("unknown country codes {}", unknown_countries.len());
    println!("\nrows per feature code:");
    for (code, count) in per_feature {
        println!("  {code:<6} {count:>6}");
    }

    Ok(())
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| {
        format!(
            "{}: {e}\nrun scripts/fetch-geonames.sh from the repo root to download the dumps",
            path.display()
        )
        .into()
    })
}

fn parse_cities(text: &str) -> Result<Vec<RawCity>> {
    text.lines()
        .enumerate()
        .map(|(i, line)| parse_city(line, i + 1))
        .collect()
}

fn parse_city(line: &str, lineno: usize) -> Result<RawCity> {
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() != CITY_COLUMNS {
        return Err(columns_error(lineno, CITY_COLUMNS, cols.len()));
    }
    Ok(RawCity {
        geonameid: field(cols[0], lineno, "geonameid")?,
        name: cols[1].to_string(),
        asciiname: cols[2].to_string(),
        alternatenames: cols[3]
            .split(',')
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .collect(),
        latitude: field(cols[4], lineno, "latitude")?,
        longitude: field(cols[5], lineno, "longitude")?,
        feature_class: field(cols[6], lineno, "feature class")?,
        feature_code: cols[7].to_string(),
        country_code: cols[8].to_string(),
        admin1_code: cols[10].to_string(),
        population: field(cols[14], lineno, "population")?,
        timezone: cols[17].to_string(),
    })
}

/// country code -> country name
fn parse_countries(text: &str) -> Result<HashMap<String, String>> {
    let mut countries = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != COUNTRY_COLUMNS {
            return Err(columns_error(i + 1, COUNTRY_COLUMNS, cols.len()));
        }
        countries.insert(cols[0].to_string(), cols[4].to_string());
    }
    Ok(countries)
}

/// "CC.ADM1" -> admin1 name
fn parse_admin1(text: &str) -> Result<HashMap<String, String>> {
    let mut admin1 = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != ADMIN1_COLUMNS {
            return Err(columns_error(i + 1, ADMIN1_COLUMNS, cols.len()));
        }
        admin1.insert(cols[0].to_string(), cols[1].to_string());
    }
    Ok(admin1)
}

fn field<T: FromStr>(value: &str, lineno: usize, name: &str) -> Result<T>
where
    T::Err: Display,
{
    value
        .parse()
        .map_err(|e| format!("line {lineno}: bad {name} {value:?}: {e}").into())
}

fn columns_error(lineno: usize, expected: usize, got: usize) -> Box<dyn Error> {
    format!("line {lineno}: expected {expected} columns, got {got}").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITIES: &str = "\
1850147\tTokyo\tTokyo\tEdo,TYO,Tokio,Tokió,東京,도쿄\t35.6895\t139.69171\tP\tPPLC\tJP\t\t40\t\t\t\t9733276\t\t44\tAsia/Tokyo\t2025-07-22
3039163\tSant Julià de Lòria\tSant Julia de Loria\tSan Julià,Sant Julia de Loria\t42.46372\t1.49129\tP\tPPLA\tAD\t\t06\t\t\t\t8022\t\t921\tEurope/Andorra\t2018-10-14
9179507\tMalmok\tMalmok\t\t12.60087\t-70.05064\tP\tPPL\tAW\t\t\t\t\t\t5637\t\t9\tAmerica/Aruba\t2026-06-24";

    const COUNTRIES: &str = "\
# comment line
#ISO\tISO3\tISO-Numeric\tfips\tCountry\tCapital\tArea(in sq km)\tPopulation\tContinent\ttld\tCurrencyCode\tCurrencyName\tPhone\tPostal Code Format\tPostal Code Regex\tLanguages\tgeonameid\tneighbours\tEquivalentFipsCode
AD\tAND\t020\tAN\tAndorra\tAndorra la Vella\t468\t77006\tEU\t.ad\tEUR\tEuro\t376\tAD###\t^(?:AD)*(\\d{3})$\tca\t3041565\tES,FR\t
JP\tJPN\t392\tJA\tJapan\tTokyo\t377835\t126529100\tAS\t.jp\tJPY\tYen\t81\t###-####\t^\\d{3}-\\d{4}$\tja\t1861060\t\t";

    const ADMIN1: &str = "\
AD.06\tSant Julià de Lòria\tSant Julia de Loria\t3039162
JP.40\tTokyo\tTokyo\t1850144";

    #[test]
    fn parses_cities() {
        let cities = parse_cities(CITIES).unwrap();
        assert_eq!(cities.len(), 3);

        let tokyo = &cities[0];
        assert_eq!(tokyo.geonameid, 1850147);
        assert_eq!(tokyo.feature_class, 'P');
        assert_eq!(tokyo.feature_code, "PPLC");
        assert_eq!(tokyo.population, 9733276);
        assert_eq!(tokyo.timezone, "Asia/Tokyo");
        assert_eq!(tokyo.alternatenames.len(), 6);
        assert_eq!(tokyo.alternatenames[5], "도쿄");
        assert_eq!(tokyo.latitude, 35.6895);
        assert_eq!(tokyo.longitude, 139.69171);
    }

    #[test]
    fn keeps_non_ascii_names() {
        let city = &parse_cities(CITIES).unwrap()[1];
        assert_eq!(city.name, "Sant Julià de Lòria");
        assert_eq!(city.asciiname, "Sant Julia de Loria");
        assert_eq!(city.admin1_code, "06");
    }

    #[test]
    fn handles_empty_fields() {
        let city = &parse_cities(CITIES).unwrap()[2];
        assert!(city.admin1_code.is_empty());
        assert!(city.alternatenames.is_empty());
        assert_eq!(city.longitude, -70.05064);
    }

    #[test]
    fn rejects_wrong_column_count() {
        let err = parse_cities("1850147\tTokyo\tTokyo")
            .unwrap_err()
            .to_string();
        assert!(err.contains("line 1"), "{err}");
        assert!(err.contains("expected 19 columns, got 3"), "{err}");
    }

    #[test]
    fn rejects_unparseable_numbers() {
        let broken = CITIES.replace("9733276", "many");
        let err = parse_cities(&broken).unwrap_err().to_string();
        assert!(err.contains("line 1"), "{err}");
        assert!(err.contains("population"), "{err}");
    }

    #[test]
    fn parses_countries_skipping_comments() {
        let countries = parse_countries(COUNTRIES).unwrap();
        assert_eq!(countries.len(), 2);
        assert_eq!(countries["JP"], "Japan");
        assert_eq!(countries["AD"], "Andorra");
    }

    #[test]
    fn parses_admin1() {
        let admin1 = parse_admin1(ADMIN1).unwrap();
        assert_eq!(admin1["AD.06"], "Sant Julià de Lòria");
        assert_eq!(admin1["JP.40"], "Tokyo");
    }
}
