//! Session bootstrap: spawns the headless desktop session and Sunshine,
//! tracks the running children, and tears them down on replacement or
//! exit.
//!
//! P1 model: one active session per host. A new `Hello` replaces the
//! current session. The sway profile (v1) pins a headless wlroots output
//! at the client's mode; the GNOME profile (v1.5+) uses `gnome-shell
//! --headless --virtual-monitor`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};

use airglow_common::Mode;

use crate::Config;
use crate::sunshine;

pub struct ActiveSession {
    pub user: String,
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub wayland_display: String,
    desktop: Child,
    sunshine: Child,
}

fn runtime_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .context("XDG_RUNTIME_DIR not set")
}

fn list_wayland_sockets(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if let Some(sock) = name.strip_prefix("wayland-") {
                if sock.parse::<u32>().is_ok() {
                    out.insert(sock.to_string());
                }
            }
        }
    }
    out
}

/// Discover the newest wayland socket that was not there before.
fn discover_wayland_display(dir: &Path, before: &BTreeSet<String>) -> anyhow::Result<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let now = list_wayland_sockets(dir);
        let mut fresh: Vec<_> = now.difference(before).collect();
        if !fresh.is_empty() {
            fresh.sort();
            return Ok(format!("wayland-{}", fresh.last().unwrap()));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    bail!("no new wayland socket appeared in {}", dir.display());
}

/// Render a sway config pinning the headless output to the client mode.
fn sway_config(cfg: &Config, mode: &Mode) -> anyhow::Result<PathBuf> {
    let dir = sunshine::state_dir(cfg)?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join("sway-config");
    let mut content = String::from("xwayland disable\n");
    content.push_str(&format!(
        "output HEADLESS-1 resolution {}x{}@{}\n",
        mode.width, mode.height, mode.refresh
    ));
    if mode.scale != 100 {
        content.push_str(&format!(
            "output HEADLESS-1 scale {}\n",
            mode.scale as f64 / 100.0
        ));
    }
    std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

fn spawn_sway(cfg: &Config, mode: &Mode) -> anyhow::Result<Child> {
    let path = sway_config(cfg, mode)?;
    Command::new(&cfg.session.sway_binary)
        .arg("--config")
        .arg(&path)
        .env("WLR_BACKENDS", "headless")
        .env("XDG_SESSION_TYPE", "wayland")
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("spawning {}", cfg.session.sway_binary))
}

fn spawn_gnome(cfg: &Config, mode: &Mode) -> anyhow::Result<Child> {
    let monitor_arg = format!("{}x{}@{}", mode.width, mode.height, mode.refresh);
    Command::new("dbus-run-session")
        .arg("--")
        .arg(&cfg.session.gnome_session_command)
        .arg("--headless")
        .arg("--virtual-monitor")
        .arg(&monitor_arg)
        .env("XDG_SESSION_TYPE", "wayland")
        // Portal backends (ScreenCast/RemoteDesktop) are selected by
        // XDG_CURRENT_DESKTOP; without this the session falls back to
        // the gtk portal which lacks screen capture support.
        .env("XDG_CURRENT_DESKTOP", "GNOME")
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("spawning {}", cfg.session.gnome_session_command))
}

impl ActiveSession {
    pub fn start(cfg: &Config, user: &str, mode: &Mode) -> anyhow::Result<Self> {
        let dir = runtime_dir()?;
        let before = list_wayland_sockets(&dir);

        let mut desktop = match cfg.session.profile.as_str() {
            "sway" => spawn_sway(cfg, mode)?,
            "gnome" => spawn_gnome(cfg, mode)?,
            other => bail!("unknown session profile {other}"),
        };

        let wayland_display = match discover_wayland_display(&dir, &before) {
            Ok(d) => d,
            Err(err) => {
                let _ = desktop.kill();
                let _ = desktop.wait();
                return Err(err);
            }
        };
        eprintln!(
            "airglow-hostd: desktop session up on {wayland_display} ({}x{}@{})",
            mode.width, mode.height, mode.refresh
        );

        let sun_path = sunshine::write_config(cfg)?;
        let sun = sunshine::spawn(cfg, &sun_path, &wayland_display).context("spawning sunshine")?;

        Ok(Self {
            user: user.to_string(),
            width: mode.width,
            height: mode.height,
            refresh: mode.refresh,
            wayland_display,
            desktop,
            sunshine: sun,
        })
    }

    pub fn stop(mut self) {
        let _ = self.sunshine.kill();
        let _ = self.sunshine.wait();
        let _ = self.desktop.kill();
        let _ = self.desktop.wait();
    }
}
