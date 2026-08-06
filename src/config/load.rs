use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};

use super::Config;

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read configuration file {}", path.display()))?;
        let expanded = interpolate_environment(&raw)?;
        let mut config: Self = toml::from_str(&expanded)
            .with_context(|| format!("parse configuration file {}", path.display()))?;
        config.server.public_base_url = config
            .server
            .public_base_url
            .trim_end_matches('/')
            .to_owned();
        config.web.repository_url = config.web.repository_url.trim_end_matches('/').to_owned();
        config.web.documentation_url = config
            .web
            .documentation_url
            .trim_end_matches('/')
            .to_owned();
        config.web.support_url = config
            .web
            .support_url
            .take()
            .map(|value| value.trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty());
        config.validate()?;
        Ok(config)
    }
}

pub fn configured_path() -> anyhow::Result<PathBuf> {
    let mut arguments = env::args_os().skip(1);
    let mut command_line_path = None;
    while let Some(argument) = arguments.next() {
        if argument.as_os_str() == OsStr::new("--config") {
            if command_line_path.is_some() {
                bail!("--config may only be specified once");
            }
            command_line_path = Some(
                arguments
                    .next()
                    .map(PathBuf::from)
                    .context("--config requires a path")?,
            );
        } else {
            bail!(
                "unsupported command-line argument: {}",
                PathBuf::from(argument).display()
            );
        }
    }
    command_line_path
        .or_else(|| env::var_os("GULFSTREAM_CONFIG").map(PathBuf::from))
        .context("configuration path is required via --config or GULFSTREAM_CONFIG")
}

fn interpolate_environment(input: &str) -> anyhow::Result<String> {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    let mut in_basic_string = false;
    let mut escaped = false;
    while cursor < input.len() {
        let remainder = &input[cursor..];
        if remainder.starts_with("${") {
            if !in_basic_string {
                bail!("environment placeholders must appear inside TOML basic strings");
            }
            let end = remainder
                .find('}')
                .context("unterminated environment placeholder")?;
            let name = &remainder[2..end];
            if name.is_empty()
                || !name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                bail!("invalid environment placeholder: ${{{name}}}");
            }
            let value = env::var(name)
                .with_context(|| format!("environment variable {name} is required"))?;
            output.push_str(&escape_toml_string_content(&value));
            cursor += end + 1;
            escaped = false;
            continue;
        }
        let character = remainder.chars().next().context("invalid UTF-8 cursor")?;
        output.push(character);
        cursor += character.len_utf8();
        if in_basic_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_basic_string = false;
            }
        } else if character == '"' {
            in_basic_string = true;
        }
    }
    Ok(output)
}

fn escape_toml_string_content(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => {
                escaped.push_str(&format!("\\u{:04X}", u32::from(value)))
            }
            value => escaped.push(value),
        }
    }
    escaped
}
