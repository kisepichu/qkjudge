use actix_cors::Cors;
use actix_web::{http::header, HttpRequest};
use std::{
    io,
    path::{Component, Path, PathBuf},
};

pub const FRONTEND_ORIGIN: &str = "https://judge.tqk.blue";

pub fn cors() -> Cors {
    Cors::default()
        .allowed_origin(FRONTEND_ORIGIN)
        .allowed_methods(vec!["GET", "POST", "DELETE", "PUT", "OPTIONS"])
        .allowed_header(header::CONTENT_TYPE)
        .supports_credentials()
        .block_on_origin_mismatch(true)
}

pub fn trusted_origin(request: &HttpRequest) -> bool {
    request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        == Some(FRONTEND_ORIGIN)
}

pub fn reserved_username(username: &str) -> bool {
    username.eq_ignore_ascii_case("admin") || username.eq_ignore_ascii_case("tqk")
}

pub fn problem_directory(root: &str, path: &str) -> io::Result<PathBuf> {
    if path.is_empty()
        || !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid problem path",
        ));
    }
    let root = Path::new(root).canonicalize()?;
    let directory = root.join(path).canonicalize()?;
    if !directory.starts_with(&root) || !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "problem outside root",
        ));
    }
    Ok(directory)
}

pub fn problem_file(directory: &Path, relative: impl AsRef<Path>) -> io::Result<PathBuf> {
    let file = directory.join(relative).canonicalize()?;
    if !file.starts_with(directory) || !file.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid problem file",
        ));
    }
    Ok(file)
}

pub fn input_files(directory: &Path) -> io::Result<Vec<String>> {
    let inputs = directory.join("in").canonicalize()?;
    if !inputs.starts_with(directory) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "inputs outside problem",
        ));
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(inputs)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    files.sort();
    for filename in &files {
        problem_file(directory, Path::new("in").join(filename))?;
        problem_file(
            directory,
            Path::new("out").join(filename.replace(".in", ".out")),
        )?;
    }
    Ok(files)
}

pub fn truncate_io(mut value: String, maximum: usize) -> String {
    if value.len() > maximum {
        let mut boundary = maximum;
        while !value.is_char_boundary(boundary) {
            boundary -= 1;
        }
        value.truncate(boundary);
        value.push_str("...");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_preserves_unicode_at_the_byte_boundary() {
        let value = "x".repeat(1023) + "é";
        assert_eq!(truncate_io(value, 1024), "x".repeat(1023) + "...");
        assert_eq!(truncate_io("é".to_owned(), 2), "é");
        assert_eq!(truncate_io("é".to_owned(), 0), "...");
    }

    #[test]
    fn problem_paths_cannot_escape_their_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("p1")).unwrap();
        std::fs::write(root.path().join("p1/problem.yaml"), "title: fixture").unwrap();
        let directory = problem_directory(root.path().to_str().unwrap(), "p1").unwrap();
        assert_eq!(
            std::fs::read_to_string(problem_file(&directory, "problem.yaml").unwrap()).unwrap(),
            "title: fixture"
        );
        for path in ["../outside", "/etc", ""] {
            assert!(problem_directory(root.path().to_str().unwrap(), path).is_err());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            std::fs::write(outside.path().join("secret"), "not a testcase").unwrap();
            symlink(outside.path(), root.path().join("escape")).unwrap();
            symlink(outside.path().join("secret"), directory.join("expected")).unwrap();
            symlink(outside.path(), directory.join("in")).unwrap();
            assert!(problem_directory(root.path().to_str().unwrap(), "escape").is_err());
            assert!(problem_file(&directory, "expected").is_err());
            assert!(input_files(&directory).is_err());
        }
    }
}
