use std::path::Path;

#[must_use]
pub fn clean_filename(input: &str, fallback: &str, maximum_bytes: usize) -> String {
    let source = Path::new(input)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(fallback);
    let cleaned: String = source
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let normalized = cleaned.trim_matches(['.', '_']);
    if normalized.is_empty() {
        return fallback.to_owned();
    }
    if normalized.len() <= maximum_bytes {
        return normalized.to_owned();
    }
    let truncated = &normalized[..maximum_bytes];
    let truncated = truncated.trim_matches(['.', '_']);
    if truncated.is_empty() {
        fallback.to_owned()
    } else {
        truncated.to_owned()
    }
}

#[must_use]
pub fn title_from_filename(filename: &str) -> String {
    Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(filename)
        .replace(['_', '-'], " ")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::clean_filename;

    #[test]
    fn filename_cleanup_removes_paths_and_unsafe_characters() {
        assert_eq!(
            clean_filename("../../holiday clip.mp4", "upload", 240),
            "holiday_clip.mp4"
        );
        assert_eq!(clean_filename("..", "upload", 240), "upload");
        assert_eq!(clean_filename("movie.mp4", "upload", 5), "movie");
    }
}
