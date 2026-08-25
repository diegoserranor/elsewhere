//! The saved city list on disk. It is nothing but an ordered set of
//! geonameids, so plain text — one id per line — says everything a format
//! would, and stays readable and hand-editable. Anything the file cannot
//! offer, from a missing config dir to a garbled line, degrades to fewer
//! cities rather than an error: a clock is not worth refusing to start over.

use std::env;
use std::fs;
use std::path::PathBuf;

/// The comment the file opens with, so a curious reader knows what it holds.
const HEADER: &str = "# elsewhere saved cities, one geonameid per line, in list order";

/// Where the list lives, given the environment. Takes its variables as
/// arguments so it can be tested without touching the process environment.
/// `None` means there is nowhere sensible to write, and persistence is
/// silently skipped.
fn dir(xdg: Option<&str>, home: Option<&str>) -> Option<PathBuf> {
    // An empty variable counts as unset, per the XDG spec.
    let xdg = xdg.filter(|path| !path.is_empty());
    if let Some(xdg) = xdg {
        return Some(PathBuf::from(xdg).join("elsewhere"));
    }
    let home = home.filter(|path| !path.is_empty())?;
    Some(PathBuf::from(home).join(".config").join("elsewhere"))
}

fn path() -> Option<PathBuf> {
    let xdg = env::var("XDG_CONFIG_HOME").ok();
    let home = env::var("HOME").ok();
    dir(xdg.as_deref(), home.as_deref()).map(|dir| dir.join("saved"))
}

/// The geonameids of `text`, in order. Blank lines, comments and anything
/// unparseable are dropped, as are repeats of an id already seen.
fn parse(text: &str) -> Vec<u32> {
    let mut saved: Vec<u32> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Ok(geonameid) = line.parse()
            && !saved.contains(&geonameid)
        {
            saved.push(geonameid);
        }
    }
    saved
}

fn render(saved: &[u32]) -> String {
    let mut out = String::from(HEADER);
    for geonameid in saved {
        out.push('\n');
        out.push_str(&geonameid.to_string());
    }
    out.push('\n');
    out
}

pub fn load() -> Vec<u32> {
    let Some(path) = path() else {
        return Vec::new();
    };
    match fs::read_to_string(&path) {
        Ok(text) => parse(&text),
        Err(_) => Vec::new(),
    }
}

/// Rewrites the whole list, through a temporary file renamed into place so an
/// interrupted write cannot leave half a list behind. A failure is reported
/// and shrugged off: losing the order across restarts must never break the
/// running session.
pub fn save(saved: &[u32]) {
    let Some(path) = path() else {
        return;
    };
    let Some(dir) = path.parent() else {
        return;
    };
    let temp = path.with_extension("tmp");
    if let Err(e) = fs::create_dir_all(dir)
        .and_then(|()| fs::write(&temp, render(saved)))
        .and_then(|()| fs::rename(&temp, &path))
    {
        eprintln!("{}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_geonameid_per_line() {
        assert_eq!(
            parse("1850147\n2643743\n5128581\n"),
            [1850147, 2643743, 5128581]
        );
    }

    #[test]
    fn skips_comments_blanks_and_garbage_lines() {
        let text = "# elsewhere saved cities\n1850147\n\nTokyo\n-1\n2643743\n";
        assert_eq!(parse(text), [1850147, 2643743]);
    }

    #[test]
    fn drops_duplicates_keeping_the_first() {
        assert_eq!(parse("1850147\n2643743\n1850147\n"), [1850147, 2643743]);
    }

    #[test]
    fn render_round_trips_through_parse() {
        let saved = [1850147, 2643743, 5128581];
        assert_eq!(parse(&render(&saved)), saved);
    }

    #[test]
    fn resolves_the_dir_from_xdg_config_home() {
        assert_eq!(
            dir(Some("/xdg"), Some("/home/someone")),
            Some(PathBuf::from("/xdg/elsewhere"))
        );
    }

    #[test]
    fn an_empty_xdg_variable_falls_back_to_home() {
        assert_eq!(
            dir(Some(""), Some("/home/someone")),
            Some(PathBuf::from("/home/someone/.config/elsewhere"))
        );
    }

    #[test]
    fn yields_no_dir_without_home_or_xdg() {
        assert_eq!(dir(None, None), None);
    }
}
