# Airglow TODOS / decision archive

Snapshot 2026-10-27. This file archives settled decisions and open work so the
project state survives context resets. Keep it updated with every milestone
change.

## Settled decisions

- Goal: pick "Airglow Remote Session" at GDM on a thin terminal -> connect to a
  *newly created* desktop session on a remote host (works even if the host has
  no session). Full touch/touchpad/pen/keyboard/ mouse, audio, video, USB.
  Terminal only decodes; compositing stays on the host.
- Hybrid route: reuse Sunshine/Moonlight for video/audio/basic input ("Sunshine
  mode", no self-maintained compositor); own the orchestration: `airglow-hostd`
  supervisor, GDM session wrapper, side-channel input fidelity, USB/IP
  forwarding.
- v1 commitments: GNOME-first (`gnome-shell --headless --virtual-monitor`
  bootstrap accepted; consumes Mutter D-Bus but does NOT patch upstream;
  injection stays kernel-level uinput). wlroots (weston-headless +
  wlr-screencopy) and X11 (dummy + XShm) are v1.5+.
- Client decides specs: display mode/scale/fps/codec and the input device tree
  are declared by the client in the `Hello` handshake; the host pins the virtual
  monitor and builds uinput templates to match.
- Input fidelity: raw evdev from client (EVIOCGRAB isolates from Moonlight's own
  input) -> host uinput templates; server-side libinput does gesture/pen
  interpretation. No GameStream protocol forks; all extras ride our own side
  channel.
- USB: usbredir-style framing over our channel -> `usbip attach` / `vhci_hcd` on
  the host.
- Scope: LAN/VPN only for v1 (no STUN/TURN). HDR/10-bit out of v1.
  Multi-monitor: handshake fields are arrays, single monitor in v1.
- Client decode: hardware-first (VAAPI/NVDEC/VDPAU via moonlight-qt) with
  software fallback; Kiosk via moonlight-qt CLI
  (`moonlight stream --resolution ... --fps ...`).
- No web panel: config files (`host.toml`, `client.toml`) + `airglowctl`
  (pair/status/devices/restart-session). Sunshine web UI disabled
  (`origin_web_ui_allowed = "none"`); pairing via Sunshine stdin PIN (`-0`) +
  `moonlight pair --pin` (CLI design still to implement).
- Transport split: control channel + side channel are ours (QUIC or TLS-TCP,
  TBD); Moonlight/Sunshine RTSP handles the stream. Sunshine owns ports
  47984-48010; control port chosen outside that range (48990 in examples).
- Packaging: standalone repo `airglow`, nix flake dev/build now, CI later; NixOS
  module (`my.nixos.*` in nix-config + pkgs in nix-modules) at P6.
- License GPL-3.0-only (Sunshine/Moonlight are separate processes, no copyleft
  propagation). Commits GPG-signed; ask user to unlock before committing.

## Status

- [x] P0 skeleton: workspace, control protocol, kiosk wrapper, ctl (commit
  73ff0c6)
- [x] P1 core: hostd boots headless GNOME + virtual monitor at client mode,
  spawns Sunshine, state.json + `airglowctl status` (commit 466baa0);
  smoke-verified on Shell 50
- [x] P1b capture research: portal chain validated; consent dialog is the
  headless blocker; see `docs/p1-capture-notes.md` (24d8ceb)
- [ ] P1b end-to-end (pair + stream + consent automation)
- [ ] P2 input fidelity, P3 USB, P4 pairing/UX, P5 perf, v1.5 weston/X11, P6
  NixOS integration

## Open TODOs

- P1b consent automation, pick one (notes file has details):
  - uinput keyboard in session + `org.gnome.Mutter.RemoteDesktop` key injection
    to press Enter on the consent dialog (needs `/dev/uinput` access: udev rule
    via future NixOS module; dev machines with an interactive session grab
    devices, so test on a headless host).
  - systemd unit with `AmbientCapabilities=CAP_SYS_ADMIN` + KMS capture for
    Sunshine (avoids portal consent).
  - upstream request: pre-authorized RemoteDesktop consent.
- hostd: run `gnome-keyring-daemon --start --components=secrets` inside the
  session (portal stalls without secrets service).
- hostd: Sunshine state/pairing dir handling (`~/.config/sunshine` vs
  runtime_dir; `portal_token`, `sunshine_state.json`).
- PAM auth replacing `user_allowlist` stub; `airglowctl pair` wrapping Sunshine
  PIN pairing (`sunshine ... -0` + `moonlight pair --pin`).
- E2E topology decision pending: which headless host (candidate: ESN20-R9P21;
  verify GPU/encoder choice), which machine is the thin terminal.
- P2 details: evdev device descriptor capture (ABS_MT\_\*, tilt/pressure,
  INPUT_PROP quirks), uinput templates, EVIOCGRAB, side-channel transport choice
  (QUIC vs TLS-TCP), mic over side channel if Sunshine mic path is insufficient.
- P3: udev monitor + device policy + usbip userspace (Rust `usbip` client via
  `vhci_hcd` sysfs).
- Moonlight kiosk flags: suppress menu, no-quit-after, absolute mouse off for
  pen use; verify moonlight-qt `stream` CLI args matrix.
- Later: multi-monitor, HDR fields, reconnection semantics, watcher supervision
  of desktop/Sunshine children (currently only on replacement/exit; SIGTERM of
  hostd orphans children).

## Dev quick reference

- `nix develop` (or nixpkgs cargo/rustc/gcc/clippy/rustfmt shells); cargo needs
  `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc` outside nix shells.
- `nix flake check` = build + formatting (treefmt: nixfmt/rustfmt/
  taplo/mdformat). Rebuild is the verification; no CI yet.
- Manual headless smoke:
  `dbus-run-session -- gnome-shell --headless --virtual-monitor 1280x720@60`
  with `XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=GNOME` (and keyring) in a
  detached group (setsid; tool/terminal timeouts kill child groups otherwise).
