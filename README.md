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

- `airglow-hostd` — host supervisor: auth policy, session profiles (v1: sway
  headless + pinned `HEADLESS-*` output; GNOME/Mutter is v1.5+), Sunshine config
  generation and lifecycle, uinput templates, USB/IP attach.
- `airglow-session` — GDM session wrapper: sends the control-channel `Hello`
  (mode + video + device specs) and launches Moonlight kiosk-style.
- `airglow-capture` — evdev grab + side channel for touchpad gestures, pen
  pressure/tilt and mic (Moonlight's semantics are touchscreen-level only),
  mirroring them on the host as uinput devices.
- `airglowctl` — terminal pairing/status instead of any web panel; host and
  client behavior is fully driven by `host.toml`/`client.toml`.

Video modes and input device trees are declared by the client, so the host
renders natively at the terminal's resolution with matching device semantics.

Code policy: every crate carries `#![forbid(unsafe_code)]` (hard error, no local
overrides). If kernel interfaces (ioctl EVIOCGRAB/UI_DEV_CREATE, usbip) require
`unsafe`, they land in one dedicated exception crate that itself denies
`unsafe_code` and allows it only per-item with `// SAFETY:` comments and a
written justification in the docs; no exceptions anywhere else without that
record.

## Status

P1 core in place: see `TODOS.md` for the decision archive and current milestone
state. `airglow-hostd` accepts the `Hello` handshake, boots a headless desktop
session profile (`session.profile = "sway"` (v1) or `"gnome"`), starts Sunshine
with a generated `sunshine.toml` attached to the new Wayland display, and
records the session for `airglowctl status`. The session wrapper execs
Moonlight.

Verified: the GNOME path (`gnome-shell --headless --virtual-monitor`) works on
Shell 50 and Sunshine enumerates the virtual monitor (Meta-0). v1 then pivoted
to sway/wlroots: on sway, Sunshine's wlgrab selects the pinned headless output
natively (no portal consent blockers — see `docs/p1-capture-notes.md`) and VAAPI
H.264/HEVC/AV1 encoders are available.

Known next items: sway session profile in hostd and the pair + stream + input
E2E, PAM authentication replacing the allowlist stub, side-channel input
fidelity (P2), USB/IP (P3), pairing (P4).

## Development

```sh
nix develop          # cargo/rustc/clippy/rustfmt shell
cargo build
nix build            # all four binaries in result/bin
nix fmt              # nixfmt + rustfmt + taplo + mdformat
```

CI packaging will be added later; for now the flake plus rebuild is the
verification.
