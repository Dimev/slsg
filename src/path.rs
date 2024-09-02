use std::path::PathBuf;

/// Resolves a program path to a path relative to the project root
pub fn resolve_path(path: &str) -> Option<PathBuf> {
    // backslashes means it's invalid
    if path.contains('\\') {
        return None;
    }

    // trim any initial /
    let path = path.trim_start_matches('/');

    // parts of the path
    let mut resolved = PathBuf::new();

    // go over all parts of the original path
    for component in path.split('/') {
        if component == ".." {
            // break if this path is not valid due to not being able to drop a component
            if !resolved.pop() {
                return None;
            }
        }
        // only advance if this is not the current directory
        else if component != "." {
            resolved.push(component);
        }
    }

    Some(resolved)
}

/// Concatenate a program path to a new program path
/// If the right path starts with a /, the entire right path is chosen instead
pub fn concat_path(left: &str, right: &str) -> Option<String> {
    // full concatenated path
    let path = if right.starts_with('/') {
        right.to_owned()
    } else {
        format!("{left}/{right}")
    };

    // backslashes means it's invalid
    if path.contains('\\') {
        return None;
    }

    // resolve path
    let mut resolved = Vec::new();
    for component in path.trim_start_matches('/').split('/') {
        if component == ".." {
            resolved.pop()?;
        } else if component != "." {
            resolved.push(component);
        }
    }

    Some(format!(
        "./{}",
        &resolved
            .into_iter()
            .map(|x| format!("/{x}"))
            .collect::<String>()[1..]
    ))
}

/// Get the file name
pub fn file_name(path: &str) -> Option<&str> {
    let last = path.rsplit('/').next()?;
    Some(last)
}

/// Get the file extention
pub fn file_extension(path: &str) -> Option<&str> {
    let name = path.rsplit('/').next()?;
    let after = path.rsplit('.').next()?;
    Some(after).filter(|_| after != name.trim_matches('.') && !name.starts_with('.'))
}

/// Get the file stem
pub fn file_stem(path: &str) -> Option<&str> {
    let last = path.rsplit('/').next()?;
    Some(last)
}
