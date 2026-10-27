//! airglowctl: host-side operational CLI. No web panel: pairing, status,
//! and device policy are driven from the terminal and config files.

use anyhow::bail;

const USAGE: &str = "\
airglowctl — manage airglow host sessions

subcommands:
  status            query airglow-hostd state (P1)
  pair              create/rotate pairing PIN (P4)
  devices           list forwarded uinput/USB devices (P2/P3)
  restart-session   tear down and rebuild the current session (P1)
";

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("status" | "pair" | "devices" | "restart-session") => {
            bail!("not implemented yet (planned for P1+)")
        }
        _ => {
            eprint!("{USAGE}");
            Ok(())
        }
    }
}
