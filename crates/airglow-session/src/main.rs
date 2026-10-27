//! airglow-session: GDM session wrapper.
//!
//! Started from a custom wayland session .desktop file. Skeleton (P0):
//! sends the client `Hello` (mode + video + devices) to airglow-hostd and
//! execs the Moonlight client. Device capture (airglow-capture) and PAM
//! authentication land in P2.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::Command;

use anyhow::Context;
use serde::Deserialize;

use airglow_common::{Codec, Hello, Mode, PROTOCOL_VERSION, Ready, VideoSpec};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    host: String,
    #[serde(default)]
    video: VideoConfig,
    #[serde(default)]
    session: SessionConfig,
    // Read by airglow-capture once P2 wires the side channel up.
    #[allow(dead_code)]
    #[serde(default)]
    capture: CaptureConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoConfig {
    #[serde(default = "default_fps")]
    fps: u32,
    #[serde(default = "default_bitrate")]
    bitrate_kbps: u32,
    #[serde(default = "default_codec")]
    codec: String,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            fps: default_fps(),
            bitrate_kbps: default_bitrate(),
            codec: default_codec(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionConfig {
    #[serde(default = "default_moonlight_binary")]
    moonlight_binary: String,
    /// Extra arguments passed to moonlight-qt.
    #[serde(default)]
    moonlight_args: Vec<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            moonlight_binary: default_moonlight_binary(),
            moonlight_args: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureConfig {
    /// Device kinds grabbed via evdev and forwarded on the side channel.
    /// Consumed by airglow-capture in P2.
    #[allow(dead_code)]
    #[serde(default)]
    grab: Vec<String>,
}

fn default_fps() -> u32 {
    60
}

fn default_bitrate() -> u32 {
    20000
}

fn default_codec() -> String {
    "h264".to_string()
}

fn default_moonlight_binary() -> String {
    "moonlight".to_string()
}

fn parse_codec(name: &str) -> anyhow::Result<Codec> {
    match name {
        "h264" => Ok(Codec::H264),
        "hevc" => Ok(Codec::Hevc),
        "av1" => Ok(Codec::Av1),
        other => anyhow::bail!("unknown codec {other}"),
    }
}

fn client_mode() -> anyhow::Result<Mode> {
    // P0: use the modes reported by the local output. For the skeleton we
    // trust the environment override, else a sane default.
    if let Ok(spec) = std::env::var("AIRGLOW_MODE") {
        let mut it = spec.split(['x', '@']);
        let width = it
            .next()
            .and_then(|v| v.parse().ok())
            .context("bad AIRGLOW_MODE")?;
        let height = it
            .next()
            .and_then(|v| v.parse().ok())
            .context("bad AIRGLOW_MODE")?;
        let refresh = it.next().and_then(|v| v.parse().ok()).unwrap_or(60);
        return Ok(Mode {
            width,
            height,
            refresh,
            scale: 100,
        });
    }
    Ok(Mode {
        width: 1920,
        height: 1080,
        refresh: 60,
        scale: 100,
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/etc/airglow/client.toml".to_string());
    let raw = std::fs::read_to_string(&path).with_context(|| format!("reading config {path}"))?;
    let cfg: Config = toml::from_str(&raw).with_context(|| format!("parsing {path}"))?;

    let hello = Hello {
        protocol: PROTOCOL_VERSION,
        modes: vec![client_mode()?],
        video: VideoSpec {
            fps: cfg.video.fps,
            bitrate_kbps: cfg.video.bitrate_kbps,
            codec: parse_codec(&cfg.video.codec)?,
        },
        // P2: enumerate real devices through airglow-capture.
        devices: Vec::new(),
        user: std::env::var("USER").unwrap_or_else(|_| "airglow".to_string()),
    };

    let mut stream = TcpStream::connect(&cfg.host)
        .with_context(|| format!("connecting to host {}", cfg.host))?;
    serde_json::to_writer(&mut stream, &hello).context("sending Hello")?;
    stream.write_all(b"\n")?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).context("reading Ready")?;
    let ready: Ready = serde_json::from_str(&line).context("decoding Ready")?;

    let (stream_addr, host_id) = match ready {
        Ready::Ready {
            host_id,
            stream_addr,
            ..
        } => (stream_addr, host_id),
        Ready::Error { message } => anyhow::bail!("host refused session: {message}"),
    };

    eprintln!("airglow-session: host {host_id} ready, streaming from {stream_addr}");

    // P0: exec moonlight-qt directly; kiosk flags and the capture side
    // process arrive with P2.
    let status = Command::new(&cfg.session.moonlight_binary)
        .args(&cfg.session.moonlight_args)
        .status()
        .with_context(|| format!("executing {}", cfg.session.moonlight_binary))?;
    std::process::exit(status.code().unwrap_or(1));
}
