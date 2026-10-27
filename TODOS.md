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
- v1 commitments: wlroots-first using an existing compositor (sway
  `WLR_BACKENDS=headless` + headless output; Sunshine wlgrab capture — no
  portal, no consent dialogs, no Mutter). GNOME/Mutter (
  `gnome-shell --headless --virtual-monitor`, portal consent automation) is
  deferred to v1.5+ because of portal/Mutter pitfalls. X11 (dummy + XShm) also
  v1.5+.

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

- Side/control transport: QUIC vs TLS-TCP. Decide before P2.
- Sunshine state/pairing data location (`~/.config/sunshine` vs hostd-managed
  runtime dir; creds setup via `sunshine --creds user pass`,
  `sunshine_state.json`).

## Verified (2026-10-28, dev machine as headless host)

- sway 1.12 headless: `WLR_BACKENDS=headless sway --config gen` creates
  `HEADLESS-1`; output syntax `output HEADLESS-1 resolution WxH@R` (headless
  refresh reports 0 Hz — frame pacing is client-driven).
- Sunshine 2026.09 on sway: **wlgrab enumerates and selects HEADLESS-1** (native
  wlr-export-dmabuf; portal errors are harmless fallbacks). No consent dialog
  anywhere. Encoders: h264/hevc/av1 VAAPI on RX 7800 XT.
- Sunshine CLI: `-0` reads the pairing PIN from stdin at startup; needs
  `system_tray = false` headless (else "Main loop has exited"); first run
  demands Web creds → use `sunshine --creds <user> <pass>`.
- moonlight-qt CLI (QT_QPA_PLATFORM=offscreen): `pair <host> --pin NNNN`,
  `stream <host> "<app>" --resolution WxH --fps N --bitrate K`.

## E2E topology (settled 2026-10-28)

- Visited (headless) host: this dev machine (ESN20-Z390, RX 7800 XT VAAPI;
  user's GNOME session stopped).
- Accessing thin terminal: ESN20-SP4; user reverse-forwards its SSH port to the
  dev machine on request.

## Milestones

- [x] P0 skeleton: workspace, control protocol, kiosk wrapper, ctl (73ff0c6)
- [x] P1 core: hostd boots headless GNOME + virtual monitor at client mode,
  spawns Sunshine, state.json + `airglowctl status` (466baa0); smoke-verified on
  Shell 50
- [x] P1b research: portal chain validated; consent dialog is the headless
  blocker (24d8ceb)
- [ ] **P1b E2E (sway profile)** — DoD: a Moonlight client pairs through our
  tooling and `moonlight stream` shows the headless sway session with
  keyboard/mouse input landing in it (via Sunshine's wlr-virtual-pointer path —
  verify on sway).
  1. [done] hostd session profiles: `profile = "sway" | "gnome"`; sway launch =
     generated config (pinned output) + `WLR_BACKENDS=headless`, no
     dbus-run-session/keyring; smoke-verified (Hello -> sway 1280x720 ->
     sunshine wlgrab + VAAPI)
  1. sunshine config: `system_tray = false` [done]; state redirect = launch
     Sunshine with `XDG_CONFIG_HOME=<rt>/sunshine-home` (the 2026.09 toml
     `state_file`/`credentials_file` keys are IGNORED) + first-run `--creds`
     bootstrap with a random secret \[code done; re-verify after the memory
     incident\]
  1. hostd hygiene: SIGTERM/child supervision (exit currently orphans children)
  1. pair + stream + input test: loopback on dev machine first, then SP4 as thin
     terminal (user forwards SSH port)
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
- MEMORY INCIDENT 2026-10-28: the headless sway+Sunshine (wlgrab) stack grew to
  \>10GB RSS and starved the local strata inference service (~50GB RSS); user
  killed it. Leak source not yet identified. Before rerunning capture/stream
  tests: watch per-PID RSS (e.g.
  `while :; do ps -o pid,rss,comm -p <pids>; sleep 5; done`), cap run time, and
  prefer the SP4 thin terminal for decode-side work; never run long E2E loops on
  the dev machine while inference is up.
