use std::fs;

use gulfstream::Config;

fn complete_example() -> anyhow::Result<String> {
    let mut content = fs::read_to_string("config/gulfstream.example.toml")?;
    for variable in [
        "GULFSTREAM_ADMIN_TOKEN",
        "GULFSTREAM_PASSWORD_PEPPER",
        "GULFSTREAM_SESSION_SIGNING_KEY",
        "GULFSTREAM_API_KEY_PEPPER",
        "GULFSTREAM_PLAYBACK_SIGNING_KEY",
        "GULFSTREAM_VIEWER_HASH_KEY",
    ] {
        content = content.replace(&format!("${{{variable}}}"), &"x".repeat(48));
    }
    Ok(content)
}

#[test]
fn complete_example_configuration_loads() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("gulfstream.toml");
    fs::write(&path, complete_example()?)?;
    Config::load(&path)?;
    Ok(())
}

#[test]
fn unknown_configuration_fields_are_rejected() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("gulfstream.toml");
    let content = complete_example()?.replace(
        "max_in_flight_requests = 512",
        "max_in_flight_requests = 512\nunknown_setting = true",
    );
    fs::write(&path, content)?;
    let error = Config::load(&path).expect_err("unknown configuration field must fail");
    assert!(error.to_string().contains("parse configuration file"));
    Ok(())
}
