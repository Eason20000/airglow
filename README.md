# Airglow

Remote desktop sessions where the thin terminal only glows — everything is
computed on the host.

Selecting **Airglow Remote Session** at GDM logs into a *local* session that
immediately connects to a freshly created desktop session on another machine,
even when that host has no desktop running. Video, keyboard, mouse, touchpad,
touchscreen, pen, audio and USB are remoted end to end; the terminal machine
only needs video decode capacity, compositing stays on the host.

## Design (hybrid)

Airglow reuses the Sunshine/Moonlight streaming stack for video, basic input and
audio, and adds the missing orchestration:

- `airglow-hostd` — host supervisor: PAM-style policy, virtual monitor creation
  via `org.gnome.Mutter.RemoteDesktop` D-Bus (client-decided resolution/scale),
  spawning the headless session profile, Sunshine config generation and
  lifecycle, uinput templates, USB/IP attach.
- `airglow-session` — GDM session wrapper: sends the control-channel `Hello`
  (mode + video + device specs) and launches Moonlight kiosk-style.
- `airglow-capture` — evdev grab + side channel for touchpad gestures, pen
  pressure/tilt and mic (Moonlight's semantics are touchscreen-level only),
  mirroring them on the host as uinput devices.
- `airglowctl` — terminal pairing/status instead of any web panel; host and
  client behavior is fully driven by `host.toml`/`client.toml`.

Video modes and input device trees are declared by the client, so the host
renders natively at the terminal's resolution with matching device semantics.

## Status

P1 core in place: `airglow-hostd` accepts the `Hello` handshake, boots a
headless GNOME session with
`gnome-shell --headless --virtual-monitor <client mode>`, starts Sunshine with a
generated `sunshine.toml` attached to the new Wayland display, and records the
session for `airglowctl status`. The session wrapper execs Moonlight.

Verified end to end on GNOME Shell 50: virtual monitor (Meta-0) appears in the
headless session and Sunshine enumerates it at the client mode.

Known next items: headless consent automation for Sunshine's RemoteDesktop
portal flow and capture path selection (see `docs/p1-capture-notes.md`), PAM
authentication replacing the allowlist stub, side-channel input fidelity (P2),
USB/IP (P3), pairing (P4).

## Development

```sh
nix develop          # cargo/rustc/clippy/rustfmt shell
cargo build
nix build            # all four binaries in result/bin
nix fmt              # nixfmt + rustfmt + taplo + mdformat
```

CI packaging will be added later; for now the flake plus rebuild is the
verification.
