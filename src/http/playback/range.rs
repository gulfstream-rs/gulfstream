use axum::http::HeaderValue;

use crate::error::AppError;

pub(super) fn parse_range(
    value: Option<&HeaderValue>,
    total: u64,
) -> Result<Option<(u64, u64)>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if total == 0 {
        return Err(AppError::RangeNotSatisfiable);
    }
    let text = value.to_str().map_err(|_| AppError::RangeNotSatisfiable)?;
    let range = text
        .strip_prefix("bytes=")
        .ok_or(AppError::RangeNotSatisfiable)?;
    if range.contains(',') {
        return Err(AppError::RangeNotSatisfiable);
    }
    let (start_text, end_text) = range.split_once('-').ok_or(AppError::RangeNotSatisfiable)?;
    let (start, end) = if start_text.is_empty() {
        let suffix = end_text
            .parse::<u64>()
            .map_err(|_| AppError::RangeNotSatisfiable)?;
        if suffix == 0 {
            return Err(AppError::RangeNotSatisfiable);
        }
        (total.saturating_sub(suffix.min(total)), total - 1)
    } else {
        let start = start_text
            .parse::<u64>()
            .map_err(|_| AppError::RangeNotSatisfiable)?;
        let end = if end_text.is_empty() {
            total - 1
        } else {
            end_text
                .parse::<u64>()
                .map_err(|_| AppError::RangeNotSatisfiable)?
                .min(total - 1)
        };
        (start, end)
    };
    if start >= total || start > end {
        return Err(AppError::RangeNotSatisfiable);
    }
    Ok(Some((start, end)))
}

#[cfg(test)]
mod tests {
    use super::parse_range;
    use axum::http::HeaderValue;

    #[test]
    fn range_parser_supports_standard_and_suffix_ranges() {
        let standard = HeaderValue::from_static("bytes=10-19");
        assert_eq!(
            parse_range(Some(&standard), 100).expect("valid range"),
            Some((10, 19))
        );
        let open = HeaderValue::from_static("bytes=90-");
        assert_eq!(
            parse_range(Some(&open), 100).expect("valid range"),
            Some((90, 99))
        );
        let suffix = HeaderValue::from_static("bytes=-10");
        assert_eq!(
            parse_range(Some(&suffix), 100).expect("valid range"),
            Some((90, 99))
        );
    }

    #[test]
    fn range_parser_rejects_multiple_or_out_of_bounds_ranges() {
        let multiple = HeaderValue::from_static("bytes=0-1,4-5");
        assert!(parse_range(Some(&multiple), 100).is_err());
        let outside = HeaderValue::from_static("bytes=100-101");
        assert!(parse_range(Some(&outside), 100).is_err());
    }
}
