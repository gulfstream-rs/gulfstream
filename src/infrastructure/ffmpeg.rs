use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{Context, bail};
use serde::Deserialize;
use tokio::process::Command;

use crate::config::{SegmentFormat, TranscodingConfig, TranscodingProfile};

#[derive(Clone, Debug)]
pub struct MediaProbe {
    pub duration_ms: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GeneratedVariant {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub bandwidth_bps: u64,
    pub codecs: String,
    pub playlist_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ProbeDocument {
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

pub async fn probe(path: &Path, config: &TranscodingConfig) -> anyhow::Result<MediaProbe> {
    let mut command = Command::new(&config.ffprobe_path);
    command
        .arg("-v")
        .arg("error")
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-of")
        .arg("json")
        .arg(path);
    let output = run_command(&mut command, config.command_timeout_seconds)
        .await
        .context("run ffprobe")?;
    let document: ProbeDocument =
        serde_json::from_slice(&output.stdout).context("parse ffprobe JSON")?;
    let video = document
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"));
    let audio = document
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"));
    let duration_ms = document
        .format
        .and_then(|format| format.duration)
        .and_then(|duration| duration.parse::<f64>().ok())
        .filter(|duration| duration.is_finite() && *duration >= 0.0)
        .map(|duration| (duration * 1000.0).round() as i64);
    Ok(MediaProbe {
        duration_ms,
        width: video.and_then(|stream| stream.width),
        height: video.and_then(|stream| stream.height),
        video_codec: video.and_then(|stream| stream.codec_name.clone()),
        audio_codec: audio.and_then(|stream| stream.codec_name.clone()),
    })
}

pub async fn generate_hls(
    source: &Path,
    staging_root: &Path,
    probe: &MediaProbe,
    config: &TranscodingConfig,
) -> anyhow::Result<Vec<GeneratedVariant>> {
    let source_width = probe.width.context("source video width is unavailable")?;
    let source_height = probe.height.context("source video height is unavailable")?;
    if source_width == 0 || source_height == 0 {
        bail!("source video dimensions must be greater than zero");
    }
    let profiles = select_profiles(&config.profiles, source_height, config.allow_upscale);
    tokio::fs::create_dir_all(staging_root).await?;
    let mut variants = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let (width, height) = scaled_dimensions(
            source_width,
            source_height,
            profile.maximum_height,
            config.allow_upscale,
        );
        let directory = staging_root.join(&profile.name);
        tokio::fs::create_dir_all(&directory).await?;
        let playlist_path = directory.join(&config.variant_playlist_filename);
        transcode_profile(
            source,
            &directory,
            &playlist_path,
            profile,
            width,
            height,
            config,
        )
        .await?;
        variants.push(GeneratedVariant {
            name: profile.name.clone(),
            width,
            height,
            bandwidth_bps: (u64::from(profile.maximum_video_bitrate_kbps)
                + if probe.audio_codec.is_some() {
                    u64::from(profile.audio_bitrate_kbps)
                } else {
                    0
                })
            .saturating_mul(1000),
            codecs: if probe.audio_codec.is_some() {
                format!("{},{}", profile.hls_video_codec, profile.hls_audio_codec)
            } else {
                profile.hls_video_codec.clone()
            },
            playlist_path,
        });
    }
    write_master_playlist(staging_root, &variants, config).await?;
    Ok(variants)
}

fn select_profiles(
    profiles: &[TranscodingProfile],
    source_height: u32,
    allow_upscale: bool,
) -> Vec<&TranscodingProfile> {
    let mut ordered: Vec<_> = profiles.iter().collect();
    ordered.sort_by_key(|profile| profile.maximum_height);
    if allow_upscale {
        return ordered;
    }
    let selected: Vec<_> = ordered
        .iter()
        .copied()
        .filter(|profile| profile.maximum_height <= source_height)
        .collect();
    if selected.is_empty() {
        ordered.into_iter().take(1).collect()
    } else {
        selected
    }
}

fn scaled_dimensions(
    source_width: u32,
    source_height: u32,
    maximum_height: u32,
    allow_upscale: bool,
) -> (u32, u32) {
    let height = if allow_upscale {
        maximum_height
    } else {
        maximum_height.min(source_height)
    };
    let even_height = (height & !1).max(2);
    let raw_width = (u64::from(source_width) * u64::from(even_height)) / u64::from(source_height);
    let even_width = u32::try_from(raw_width).unwrap_or(u32::MAX) & !1;
    (even_width.max(2), even_height)
}

async fn transcode_profile(
    source: &Path,
    directory: &Path,
    playlist_path: &Path,
    profile: &TranscodingProfile,
    width: u32,
    height: u32,
    config: &TranscodingConfig,
) -> anyhow::Result<()> {
    let gop =
        u64::from(profile.frame_rate).saturating_mul(u64::from(config.segment_duration_seconds));
    let mut command = Command::new(&config.ffmpeg_path);
    command.arg("-hide_banner").arg("-nostdin").arg("-y");
    command.args(&config.extra_input_arguments);
    command.arg("-i").arg(source);
    command
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("0:a:0?")
        .arg("-vf")
        .arg(format!(
            "scale={width}:{height}:flags={}",
            config.scaling_flags
        ))
        .arg("-c:v")
        .arg(&profile.video_codec)
        .arg("-preset")
        .arg(&profile.preset)
        .arg("-profile:v")
        .arg(&profile.profile)
        .arg("-pix_fmt")
        .arg(&profile.pixel_format)
        .arg("-r")
        .arg(profile.frame_rate.to_string())
        .arg("-b:v")
        .arg(format!("{}k", profile.video_bitrate_kbps))
        .arg("-maxrate")
        .arg(format!("{}k", profile.maximum_video_bitrate_kbps))
        .arg("-bufsize")
        .arg(format!("{}k", profile.video_buffer_kbps))
        .arg("-g")
        .arg(gop.to_string())
        .arg("-keyint_min")
        .arg(gop.to_string())
        .arg("-sc_threshold")
        .arg("0")
        .arg("-force_key_frames")
        .arg(format!(
            "expr:gte(t,n_forced*{})",
            config.segment_duration_seconds
        ))
        .arg("-c:a")
        .arg(&profile.audio_codec)
        .arg("-b:a")
        .arg(format!("{}k", profile.audio_bitrate_kbps))
        .arg("-ar")
        .arg(config.audio_sample_rate_hz.to_string())
        .arg("-ac")
        .arg(config.audio_channels.to_string())
        .arg("-threads")
        .arg(config.ffmpeg_threads.to_string())
        .arg("-f")
        .arg("hls")
        .arg("-hls_time")
        .arg(config.segment_duration_seconds.to_string())
        .arg("-hls_playlist_type")
        .arg(&config.hls_playlist_type);
    if !config.hls_flags.is_empty() {
        command.arg("-hls_flags").arg(config.hls_flags.join("+"));
    }
    match config.segment_format {
        SegmentFormat::Fmp4 => {
            command
                .arg("-hls_segment_type")
                .arg("fmp4")
                .arg("-hls_fmp4_init_filename")
                .arg(&config.fmp4_init_filename)
                .arg("-hls_segment_filename")
                .arg(directory.join(&config.fmp4_segment_filename_pattern));
        }
        SegmentFormat::MpegTs => {
            command
                .arg("-hls_segment_filename")
                .arg(directory.join(&config.mpegts_segment_filename_pattern));
        }
    }
    command
        .args(&config.extra_output_arguments)
        .arg(playlist_path);
    run_command(&mut command, config.command_timeout_seconds)
        .await
        .with_context(|| format!("transcode profile {}", profile.name))?;
    Ok(())
}

async fn write_master_playlist(
    root: &Path,
    variants: &[GeneratedVariant],
    config: &TranscodingConfig,
) -> anyhow::Result<()> {
    let mut master = format!("#EXTM3U\n#EXT-X-VERSION:{}\n", config.hls_version);
    for variant in variants {
        master.push_str(&format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{},CODECS=\"{}\"\n{}/{}\n",
            variant.bandwidth_bps,
            variant.width,
            variant.height,
            variant.codecs,
            variant.name,
            config.variant_playlist_filename,
        ));
    }
    tokio::fs::write(root.join(&config.master_playlist_filename), master).await?;
    Ok(())
}

async fn run_command(
    command: &mut Command,
    timeout_seconds: u64,
) -> anyhow::Result<std::process::Output> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(timeout_seconds), command.output())
        .await
        .context("media command timed out")??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "media command failed with {}: {}",
            output.status,
            stderr.trim()
        );
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::scaled_dimensions;

    #[test]
    fn scaling_preserves_aspect_ratio_and_even_dimensions() {
        assert_eq!(scaled_dimensions(1920, 1080, 720, false), (1280, 720));
        assert_eq!(scaled_dimensions(1280, 720, 1080, false), (1280, 720));
        assert_eq!(scaled_dimensions(853, 480, 360, false), (638, 360));
    }
}
