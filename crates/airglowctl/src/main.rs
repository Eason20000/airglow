#![forbid(unsafe_code)]
//! airglowctl: host-side operational CLI. No web panel: pairing, status,
//! and device policy are driven from the terminal and config files.

use std::path::PathBuf;

use anyhow::Context;

use airglow_common::HostState;

const USAGE: &str = "\
airglowctl — manage airglow host sessions

usage: airglowctl [--state-file PATH] <subcommand>

subcommands:
  status            show the active session as recorded by airglow-hostd
  pair              create/rotate pairing PIN (P4)
  devices           list forwarded uinput/USB devices (P2/P3)
  restart-session   tear down and rebuild the current session (P1+)
";

fn default_state_file() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    runtime.join("airglow").join("state.json")
}

fn status(path: &std::path::Path) -> anyhow::Result<()> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let state: HostState = serde_json::from_str(&raw).context("decoding state")?;
    println!("host_id: {}", state.host_id);
    println!("control: {}", state.listen);
    match state.session {
        Some(s) => println!(
            "session: user={} mode={}x{}@{} display={} started_utc={}",
            s.user, s.width, s.height, s.refresh, s.wayland_display, s.started_utc
        ),
        None => println!("session: none"),
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let mut state_file = default_state_file();
    let mut sub = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--state-file" => {
                state_file = PathBuf::from(args.next().context("--state-file needs a value")?);
            }
            other => {
                sub = Some(other.to_string());
                break;
            }
        }
    }

    match sub.as_deref() {
        Some("status") => status(&state_file),
        Some("pair" | "devices" | "restart-session") => {
            anyhow::bail!("not implemented yet (planned for P2+)")
        }
        _ => {
            eprint!("{USAGE}");
            Ok(())
        }
    }
}
