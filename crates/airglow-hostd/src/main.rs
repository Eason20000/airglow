//! airglow-hostd: host-side supervisor.
//!
//! P1: loads configuration, serves the line-delimited-JSON control
//! channel, and on each accepted `Hello` tears down any previous session
//! then starts a headless desktop session (virtual monitor at the
//! client's mode) plus a Sunshine instance serving the video stream.
//! PAM authentication replaces the allowlist stub in a later step.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use serde::Deserialize;

use airglow_common::{Hello, HostState, PROTOCOL_VERSION, Ready, SessionState};

mod session;
mod sunshine;

use session::ActiveSession;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    listen: String,
    host_id: String,
    session: SessionConfig,
    #[serde(default)]
    sunshine: SunshineConfig,
    #[serde(default)]
    runtime_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    /// Users allowed to open remote sessions.
    user_allowlist: Vec<String>,
    /// Desktop session binary for the active profile.
    pub session_command: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SunshineConfig {
    #[serde(default = "default_sunshine_binary")]
    pub binary: String,
    #[serde(default = "default_encoder")]
    pub encoder: String,
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

struct Server {
    cfg: Config,
    active: Mutex<Option<ActiveSession>>,
}

impl Server {
    fn write_state(&self, session: Option<&SessionState>) -> anyhow::Result<()> {
        let dir = sunshine::state_dir(&self.cfg)?;
        std::fs::create_dir_all(&dir).ok();
        let state = HostState {
            host_id: self.cfg.host_id.clone(),
            listen: self.cfg.listen.clone(),
            session: session.cloned(),
        };
        let path = dir.join("state.json");
        let tmp = path.with_extension("json.tmp");
        let mut tmp_file =
            std::fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        serde_json::to_writer_pretty(&mut tmp_file, &state).context("writing state tmp")?;
        std::fs::rename(&tmp, &path).context("replacing state file")?;
        Ok(())
    }

    fn handle_conn(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line).context("reading Hello")?;

        let hello: Hello = serde_json::from_str(&line).context("decoding Hello")?;
        if hello.protocol != PROTOCOL_VERSION {
            return reject(&mut stream, "unsupported protocol version");
        }
        if !self.cfg.session.user_allowlist.contains(&hello.user) {
            return reject(&mut stream, "user not allowed");
        }
        let mode = hello
            .modes
            .first()
            .copied()
            .context("client sent no display mode")?;

        // Replace any previous session before starting the new one.
        if let Some(old) = self.active.lock().unwrap().take() {
            eprintln!("airglow-hostd: tearing down previous session");
            old.stop();
        }

        let started = session::ActiveSession::start(&self.cfg, &hello.user, &mode)?;
        let snap = SessionState {
            user: started.user.clone(),
            width: started.width,
            height: started.height,
            refresh: started.refresh,
            wayland_display: started.wayland_display.clone(),
            started_utc: utc_now(),
        };
        self.write_state(Some(&snap))?;
        *self.active.lock().unwrap() = Some(started);

        let reply = Ready::Ready {
            host_id: self.cfg.host_id.clone(),
            stream_addr: format!(
                "{}:{}",
                host_of(&self.cfg.listen),
                self.cfg.sunshine.stream_port
            ),
            auth_required: false,
        };
        serde_json::to_writer(&mut stream, &reply)?;
        Ok(())
    }
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

fn utc_now() -> String {
    // No chrono dependency for the skeleton; system clock is enough.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/etc/airglow/host.toml".to_string());
    let cfg = load_config(PathBuf::from(path).as_path())?;

    let server = Arc::new(Server {
        cfg,
        active: Mutex::new(None),
    });

    let listener = TcpListener::bind(&server.cfg.listen)
        .with_context(|| format!("binding control channel {}", server.cfg.listen))?;
    eprintln!(
        "airglow-hostd: host_id={} listening on {}",
        server.cfg.host_id, server.cfg.listen
    );
    server.write_state(None).ok();

    let mut conns = Vec::new();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let srv = Arc::clone(&server);
                let handle = std::thread::spawn(move || {
                    if let Err(err) = srv.handle_conn(stream) {
                        eprintln!("airglow-hostd: connection error: {err:#}");
                    }
                });
                conns.retain(|h: &std::thread::JoinHandle<()>| !h.is_finished());
                conns.push(handle);
            }
            Err(err) => eprintln!("airglow-hostd: accept error: {err:#}"),
        }
    }
    Ok(())
}
