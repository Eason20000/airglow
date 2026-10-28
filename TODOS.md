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
- Safety policy: `#![forbid(unsafe_code)]` in every crate (hard ban, not warn).
  Future unavoidable unsafe (evdev/uinput ioctls, usbip) goes into one dedicated
  exception crate: crate-level `deny` + per-item `#[allow(unsafe_code)]` +
  `// SAFETY:` comment + written justification in docs; any other exception
  needs explicit user sign-off recorded here. Does not cover dependency-internal
  unsafe or safe-Rust bugs (e.g. leaks).

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
- Accessing thin terminal: ESN20-R9P21 for now (SP4 is not at home). R9P21 is a
  RTX 3060 + Cezanne laptop whose user is not in `video`, so Moonlight decodes
  in software; that is fine for verification. Its headless sway has no input
  devices, so injected key events cannot reach a window there — the test rig
  runs Moonlight under Xvfb and types through XTest instead. A real thin
  terminal has a physical keyboard, so this only affects the test harness.

## Milestones

- [x] P0 skeleton: workspace, control protocol, kiosk wrapper, ctl (73ff0c6)
- [x] P1 core: hostd boots headless GNOME + virtual monitor at client mode,
  spawns Sunshine, state.json + `airglowctl status` (466baa0); smoke-verified on
  Shell 50
- [x] P1b research: portal chain validated; consent dialog is the headless
  blocker (24d8ceb)
- [ ] **P1b E2E (sway profile)** — DoD: a Moonlight client pairs through our
  tooling and `moonlight stream` shows the headless sway session with
  keyboard/mouse input landing in it. Sunshine has no wlr virtual-input
  protocol, so input is uinput only; the compositor must therefore have
  libinput.
  - [done] video loopback Z390 -> R9P21: wlgrab -> VAAPI HEVC -> RTSP ->
    Moonlight (software decode); red test wallpaper confirmed in a client
    screenshot.
  - [done] input loopback: XTest type in Moonlight under Xvfb -> Sunshine uinput
    -> sway libinput -> foot receives `hello-airglow`; pointer motion works too.
  - [done] headless sway needs libinput for input, and libinput needs an active
    seat: run seatd (`SEATD_VTBOUND=0`, socket group `wheel` here) and launch
    sway with `WLR_BACKENDS=headless,libinput` + `LIBSEAT_BACKEND=seatd`.
    Without those, sway starts but `swaymsg -t get_inputs` is empty and no
    injected event can arrive. seatd allows only one active client per seat, so
    a stray sway holding the seat makes the next session start fail with "no new
    wayland socket appeared".
  - [ ] P6 must install seatd as a system service (`services.seatd`,
    `SEATD_VTBOUND=0`) and add the hostd user to its socket group.
  1. [done] hostd session profiles: `profile = "sway" | "gnome"`; sway launch =
     generated config (pinned output) + `WLR_BACKENDS=headless,libinput` +
     `LIBSEAT_BACKEND=seatd`, no dbus-run-session/keyring; smoke-verified (Hello
     -> sway 1280x720 -> sunshine wlgrab + VAAPI)
  1. sunshine config: `system_tray = false` [done]; state redirect = launch
     Sunshine with `XDG_CONFIG_HOME=<rt>/sunshine-home` (the 2026.09 toml
     `state_file`/`credentials_file` keys are IGNORED; real state dir is
     `$XDG_CONFIG_HOME/sunshine/`) + first-run `--creds` bootstrap \[done,
     verified: bootstraps once, state survives session replacement\]
  1. sway wayland discovery \[done\]: sway runs with a private
     `XDG_RUNTIME_DIR=<rt>/sway-run` and discovery diffs inside it only
     (shared-dir names are unreliable: SIGKILL leaves stale locks, locks carry
     no pid, pinned WAYLAND_DISPLAY is skipped, stale locks force renumbering).
     sway output pinned via `resolution WxH` only (headless has no refresh
     control). 3x Hello-replacement rotation passes under the RAM wall.
  1. hostd hygiene: SIGTERM/child supervision (exit currently orphans children)
  1. pair + stream + input test: [done, by hand] Z390 \<-> R9P21 pairing via the
     Sunshine web API (Basic auth, confighttp port 47990: GET/POST/DELETE
     `/api/pin`; the stdin `--pin 0` trick is unreliable), stream + input
     verified. Remaining: drive it from `airglowctl pair` and repeat on SP4 as
     the thin terminal (user forwards SSH port).
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
  `user_allowlist`; `airglowctl pair` driving the Sunshine web API (Basic auth
  on confighttp 47990, `GET/POST/DELETE /api/pin`) instead of the stdin PIN
  (`sunshine -0`), which failed in testing.
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
- MEMORY INCIDENT 2026-10-28 (resolved): airglow-hostd grew to >10GB RSS,
  OOM-killed opencode, starved the local strata inference service; user killed
  it. Root cause: `random_hex` used `std::fs::read("/dev/urandom")` — reading a
  char device until EOF, i.e. forever. Fixed with `read_exact` into a fixed
  buffer. Never unbounded-read a device file.
- seatd for a headless test host (needs root; its socket group must include the
  session user). Start it with `SEATD_VTBOUND=0` so it never touches a VT, and
  wrap it in `sudo sh -c '...'` so the inner `</dev/null` redirect does not
  swallow sudo's own password prompt. Here it listens on `/run/seatd.sock` with
  group `wheel`.
- Pairing by hand (client side, no proxy): with the host's Sunshine web creds,
  `GET /api/pin` (Basic auth, `https://127.0.0.1:47990`) to obtain `pairing_id`,
  `POST /api/pin` with `{"pairing_id","pin","name"}`, and `DELETE /api/pin` to
  clear a stale pending pair.
- Client test rig on a headless machine: run Moonlight under
  `Xvfb :99 -screen 0 WxHx24` (DISPLAY=:99, QT_QPA_PLATFORM=xcb,
  SDL_AUDIODRIVER=dummy) and inject keys with `xdotool type`; a headless sway
  client has no seat keyboard, so wlrctl/wtype injects reach nothing there.
- RAM wall for ALL load tests: run the whole stack as a transient user unit so
  hostd + sway + sunshine share one cgroup and get auto-killed over the limit:
  `systemctl --user stop airglow-smoke; systemd-run --user --unit=airglow-smoke -p MemoryMax=1G -p TasksMax=256 ./target/debug/airglow-hostd <host.toml>`;
  logs: `journalctl --user -u airglow-smoke`. In the test host.toml use absolute
  sway/sunshine store paths (the unit PATH is minimal).
