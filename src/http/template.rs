//! Helpers shared by the HTML renderers (web shell and player page).

use serde::Serialize;

use crate::error::AppError;

/// Escapes text for use in HTML element content and quoted attributes.
#[must_use]
pub fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Serializes a value to JSON that is safe to embed in an inline `<script>`.
///
/// `<`, `>` and `&` are written as the JSON escapes `\u003c`, `\u003e` and
/// `\u0026`, which are equivalent inside JSON strings but cannot close the
/// script element or open an HTML comment.
///
/// # Errors
///
/// Returns an internal error when serialization fails.
pub fn script_json<T: Serialize + ?Sized>(value: &T) -> Result<String, AppError> {
    let json = serde_json::to_string(value).map_err(AppError::internal)?;
    Ok(json
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026"))
}

/// Replaces `{{MARKER}}` placeholders in a single pass.
///
/// Substituted values are never rescanned, so user-controlled text containing
/// `{{...}}` cannot inject into other placeholders. Unknown markers are left
/// untouched.
#[must_use]
pub fn substitute(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(index) = rest.find("{{") {
        output.push_str(&rest[..index]);
        let candidate = &rest[index..];
        if let Some((marker, value)) = replacements
            .iter()
            .find(|(marker, _)| candidate.starts_with(marker))
        {
            output.push_str(value);
            rest = &candidate[marker.len()..];
        } else {
            output.push_str("{{");
            rest = &candidate[2..];
        }
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::{html_escape, script_json, substitute};

    #[test]
    fn html_escape_covers_markup_and_quotes() {
        assert_eq!(
            html_escape(r#"<a href="x">Tom & 'Jerry'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/a&gt;"
        );
        assert_eq!(html_escape("plain"), "plain");
    }

    #[test]
    fn script_json_escapes_html_significant_characters() {
        let json = script_json("</script><!-- & -->").expect("serializes");
        assert!(!json.contains('<') && !json.contains('>') && !json.contains('&'));
        assert_eq!(json, r#""\u003c/script\u003e\u003c!-- \u0026 --\u003e""#);
        let round_trip: String = serde_json::from_str(&json).expect("valid json");
        assert_eq!(round_trip, "</script><!-- & -->");
    }

    #[test]
    fn script_json_serializes_null_and_unsized_values() {
        assert_eq!(script_json(&Option::<String>::None).expect("ok"), "null");
        assert_eq!(script_json("a").expect("ok"), "\"a\"");
    }

    #[test]
    fn substitute_does_not_rescan_inserted_values() {
        let output = substitute("{{A}}|{{B}}|{{C}}", &[("{{A}}", "{{B}}"), ("{{B}}", "b")]);
        assert_eq!(output, "{{B}}|b|{{C}}");
    }
}
