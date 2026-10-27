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

P0 skeleton: control-channel types, host daemon config + `Hello` handling,
session wrapper that execs Moonlight, device listing. Session spawning (P1),
input fidelity (P2), USB forwarding (P3) and pairing (P4) follow the roadmap in
the project notes.

## Development

```sh
nix develop          # cargo/rustc/clippy/rustfmt shell
cargo build
nix build            # all four binaries in result/bin
nix fmt              # nixfmt + rustfmt + taplo + mdformat
```

CI packaging will be added later; for now the flake plus rebuild is the
verification.
