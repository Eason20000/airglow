//! airglow-hostd: host-side supervisor.
//!
//! Skeleton (P0): loads configuration, serves the line-delimited-JSON
//! control channel, and records the client `Hello`. Session spawning
//! (virtual monitor + headless GNOME + Sunshine) lands in P1.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use anyhow::Context;
use serde::Deserialize;

use airglow_common::{Hello, PROTOCOL_VERSION, Ready};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    listen: String,
    host_id: String,
    session: SessionConfig,
    #[serde(default)]
    sunshine: SunshineConfig,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionConfig {
    /// Users allowed to open remote sessions.
    user_allowlist: Vec<String>,
    /// Session profile command, e.g. `gnome-session`. Consumed in P1.
    #[allow(dead_code)]
    session_command: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SunshineConfig {
    // Consumed by the P1 session spawner when generating sunshine.toml.
    #[allow(dead_code)]
    #[serde(default = "default_sunshine_binary")]
    binary: String,
    #[allow(dead_code)]
    #[serde(default = "default_encoder")]
    encoder: String,
    #[serde(default = "default_stream_port")]
    stream_port: u16,
}

impl Default for SunshineConfig {
    fn default() -> Self {
        Self {
            binary: default_sunshine_binary(),
            encoder: default_encoder(),
            stream_port: default_stream_port(),
        }
    }
}

fn default_sunshine_binary() -> String {
    "sunshine".to_string()
}

fn default_encoder() -> String {
    "auto".to_string()
}

fn default_stream_port() -> u16 {
    47989
}

fn load_config(path: &std::path::Path) -> anyhow::Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parsing config {}", path.display()))
}

fn handle_conn(mut stream: TcpStream, cfg: &Config) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line).context("reading Hello")?;

    let hello: Hello = serde_json::from_str(&line).context("decoding Hello")?;
    if hello.protocol != PROTOCOL_VERSION {
        reject(&mut stream, "unsupported protocol version")?;
        return Ok(());
    }
    if !cfg.session.user_allowlist.contains(&hello.user) {
        reject(&mut stream, "user not allowed")?;
        return Ok(());
    }

    // P1: create virtual monitor at hello.modes[0], spawn the session
    // profile, then start Sunshine tuned to hello.video.
    tracing_stub(&hello);

    let reply = Ready::Ready {
        host_id: cfg.host_id.clone(),
        stream_addr: format!("{}:{}", host_of(&cfg.listen), cfg.sunshine.stream_port),
        auth_required: true,
    };
    serde_json::to_writer(&mut stream, &reply)?;
    Ok(())
}

fn reject(stream: &mut TcpStream, message: &str) -> anyhow::Result<()> {
    let reply = Ready::Error {
        message: message.to_string(),
    };
    serde_json::to_writer(stream, &reply)?;
    Ok(())
}

fn host_of(listen: &str) -> String {
    listen
        .rsplit_once(':')
        .map(|(host, _)| host.to_string())
        .unwrap_or_else(|| "0.0.0.0".to_string())
}

fn tracing_stub(hello: &Hello) {
    // Replace with structured logging once the session manager exists.
    eprintln!(
        "hello: {} mode(s), {} devices, codec {:?} @ {}fps",
        hello.modes.len(),
        hello.devices.len(),
        hello.video.codec,
        hello.video.fps
    );
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/etc/airglow/host.toml".to_string());
    let cfg = load_config(PathBuf::from(path).as_path())?;

    let listener = TcpListener::bind(&cfg.listen)
        .with_context(|| format!("binding control channel {}", cfg.listen))?;
    eprintln!(
        "airglow-hostd: host_id={} listening on {}",
        cfg.host_id, cfg.listen
    );
    for stream in listener.incoming() {
        let stream = stream.context("accept")?;
        if let Err(err) = handle_conn(stream, &cfg) {
            eprintln!("airglow-hostd: connection error: {err:#}");
        }
    }
    Ok(())
}
