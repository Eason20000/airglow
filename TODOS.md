# Airglow TODOS / decision archive

Snapshot 2026-10-27. This file archives settled decisions and open work so the
project state survives context resets. Keep it updated with every milestone
change. Milestones are ordered risk-first; each carries a definition of done
(DoD).

## Settled decisions

Goal:

- Pick "Airglow Remote Session" at GDM on a thin terminal -> connect to a *newly
  created* desktop session on a remote host (works even if the host has no
  session). Full touch/touchpad/pen/keyboard/mouse, audio, video, USB. Terminal
  only decodes; compositing stays on the host.
- Hybrid route: reuse Sunshine/Moonlight for video/audio/basic input ("Sunshine
  mode", no self-maintained compositor); own the orchestration: `airglow-hostd`
  supervisor, GDM session wrapper, side-channel input fidelity, USB/IP
  forwarding.
- v1 commitments: GNOME-first (`gnome-shell --headless --virtual-monitor`
  bootstrap accepted; consumes Mutter D-Bus but does NOT patch upstream;
  injection stays kernel-level uinput). wlroots (weston-headless +
  wlr-screencopy) and X11 (dummy + XShm) are v1.5+.

Interfaces:

- Client decides specs: display mode/scale/fps/codec and the input device tree
  are declared by the client in the `Hello` handshake; the host pins the virtual
  monitor and builds uinput templates to match.
- Input fidelity: raw evdev from client (EVIOCGRAB isolates from Moonlight's own
  input) -> host uinput templates; server-side libinput does gesture/pen
  interpretation. No GameStream protocol forks; all extras ride our own side
  channel.
- USB: usbredir-style framing over our channel -> `usbip attach` / `vhci_hcd` on
  the host.
- No web panel: config files (`host.toml`, `client.toml`) + `airglowctl`
  (pair/status/devices/restart-session). Sunshine web UI disabled
  (`origin_web_ui_allowed = "none"`).

Scope & packaging:

- LAN/VPN only for v1 (no STUN/TURN). HDR/10-bit out of v1. Multi-monitor:
  handshake fields are arrays, single monitor in v1.
- Client decode: hardware-first (VAAPI/NVDEC/VDPAU via moonlight-qt) with
  software fallback; kiosk via moonlight-qt CLI
  (`moonlight stream --resolution ... --fps ...`).
- Packaging: standalone repo `airglow`, nix flake dev/build now, CI later; NixOS
  module (`my.nixos.*` in nix-config + pkgs in nix-modules) at P6.
- License GPL-3.0-only (Sunshine/Moonlight are separate processes, no copyleft
  propagation). Commits GPG-signed; ask user to unlock before committing.

## Pending decisions

- Consent automation route (pick as P1b task 1; details in
  `docs/p1-capture-notes.md`):
  - uinput keyboard in session + `org.gnome.Mutter.RemoteDesktop` key injection
    to press Enter on the consent dialog (needs `/dev/uinput` access: udev rule;
    test on headless host).
  - systemd unit with `AmbientCapabilities=CAP_SYS_ADMIN` + KMS capture for
    Sunshine (avoids portal consent).
  - upstream request: pre-authorized RemoteDesktop consent.
- Side/control transport: QUIC vs TLS-TCP. Decide before P2.
- E2E topology: which headless host (candidate ESN20-R9P21; verify GPU/encoder
  first), which machine is the thin terminal.
- Sunshine state/pairing data location (`~/.config/sunshine` vs hostd-managed
  runtime dir; `portal_token`, `sunshine_state.json`).

## Milestones

- [x] P0 skeleton: workspace, control protocol, kiosk wrapper, ctl (73ff0c6)
- [x] P1 core: hostd boots headless GNOME + virtual monitor at client mode,
  spawns Sunshine, state.json + `airglowctl status` (466baa0); smoke-verified on
  Shell 50
- [x] P1b research: portal chain validated; consent dialog is the headless
  blocker (24d8ceb)
- [ ] **P1b E2E** — DoD: a Moonlight client pairs through our tooling and
  `moonlight stream` shows the headless GNOME desktop with keyboard input
  landing in it.
  1. pick + implement consent route (pending decision)
  1. hostd hygiene (each blocks realistic E2E): run
     `gnome-keyring-daemon --start --components=secrets` in the session;
     Sunshine state/pairing dir handling; SIGTERM/child supervision (currently
     hostd exit orphans children)
  1. deployment path to the E2E host: `nix run` / small package (P6 minimal
     slice) beats copying binaries
  1. run pair + stream + input loopback/2-host test
- [ ] **P2 input fidelity** (needs transport decision) — DoD: a real client
  touchscreen/touchpad/pen appears as a matching uinput mirror on the host;
  libinput gestures/pen work in the remote session; no double injection with
  Moonlight's own input.
  - evdev descriptor capture (ABS_MT\_\*, tilt/pressure, INPUT_PROP quirks),
    uinput templates, EVIOCGRAB, side-channel framing
  - mic over side channel if Sunshine mic path is insufficient
  - Moonlight kiosk flags: suppress menu, no-quit-after, absolute mouse off for
    pen use; verify `stream` CLI args matrix
- [ ] **P3 USB** — DoD: allowlisted client USB devices appear on the host via
  usbip; plug/unplug handled. udev monitor + device policy + Rust usbip attach
  via `vhci_hcd` sysfs.
- [ ] **P4 auth & UX** — DoD: unauthenticated clients cannot open a session; GDM
  login leads to a working session with no manual pairing. PAM replacing
  `user_allowlist`; `airglowctl pair` wrapping Sunshine stdin PIN
  (`sunshine ... -0`) + `moonlight pair --pin`.
- [ ] **P5 perf** — DoD: measured end-to-end latency numbers per mode/encoder;
  tuned encoder/bitrate/fps defaults.
- [ ] **P6 NixOS integration** — DoD: `my.nixos.airglow` module + pkgs in
  nix-modules; declarative host.toml; udev rule for `/dev/uinput`.
- [ ] v1.5: weston-headless/wlroots and X11 profiles; multi-monitor; HDR fields;
  reconnection semantics.

## Dev quick reference

- `nix develop` (or nixpkgs cargo/rustc/gcc/clippy/rustfmt shells); cargo needs
  `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc` outside nix shells.
- `nix flake check` = build + formatting (treefmt: nixfmt/rustfmt/
  taplo/mdformat). Rebuild is the verification; no CI yet.
- Manual headless smoke:
  `dbus-run-session -- gnome-shell --headless --virtual-monitor 1280x720@60`
  with `XDG_SESSION_TYPE=wayland XDG_CURRENT_DESKTOP=GNOME` (and keyring) in a
  detached group (setsid; tool/terminal timeouts kill child groups otherwise).
- sudo without leaking the password:
  `tmux display-popup -E "sudo <cmd>; read -p done"` in the user's attached
  `SSH` session; the user types the password at the popup prompt (sudo reads
  tty, no echo; nothing via the agent). Run the privileged command itself inside
  the popup (sudo timestamps are per-tty).
