//! Log de fallos de refresh, capado, sin dependencias nuevas.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::paths::app_config_dir;

const MAX_LINES: usize = 2000;

pub fn dir() -> PathBuf {
    app_config_dir().join("logs")
}

fn path() -> PathBuf {
    dir().join("app.log")
}

pub fn append(line: &str) {
    let _ = append_at(&path(), line);
}

fn append_at(file: &PathBuf, line: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    {
        let mut handle = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)?;
        writeln!(handle, "{timestamp} {line}")?;
    }
    trim_to_last(file, MAX_LINES)
}

fn trim_to_last(file: &PathBuf, max_lines: usize) -> Result<(), std::io::Error> {
    let content = fs::read_to_string(file)?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        return Ok(());
    }
    let kept = lines[lines.len() - max_lines..].join("\n") + "\n";
    fs::write(file, kept)
}

pub fn clear() {
    let _ = fs::write(path(), b"");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_file(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("iausagebar-log-test-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("app.log")
    }

    #[test]
    fn append_trims_to_last_2000_lines() {
        let file = scratch_file("trim");
        for i in 0..2100 {
            append_at(&file, &format!("line {i}")).unwrap();
        }
        let content = fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2000);
        assert!(
            lines[0].ends_with("line 100"),
            "first surviving line: {}",
            lines[0]
        );
        assert!(lines[1999].ends_with("line 2099"));
        let _ = fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn clear_empties_existing_file() {
        let file = scratch_file("clear");
        append_at(&file, "hello").unwrap();
        fs::write(&file, b"").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "");
        let _ = fs::remove_dir_all(file.parent().unwrap());
    }
}
