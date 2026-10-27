# P1 capture notes (GNOME Wayland, Sunshine 2026.09, Shell 50)

Validated chain, in order:

1. `dbus-run-session -- gnome-shell --headless --virtual-monitor WxH@R` boots a
   full GNOME session (portal, portals' PermissionStore, autostart) on a fresh
   `wayland-N` socket.
1. `XDG_CURRENT_DESKTOP=GNOME` must be set for the session, otherwise
   xdg-desktop-portal resolves to gtk fallbacks and ScreenCast/ RemoteDesktop
   interfaces are missing (sunshine: "Could not create RemoteDesktop session").
1. `gnome-keyring-daemon --start --components=secrets` is needed inside the
   session bus; without `org.freedesktop.secrets` the portal stack stalls on
   activation timeouts.
1. Sunshine capture selection on GNOME: wlgrab unavailable (no
   `wlr-export-dmabuf`), KMS needs `CAP_SYS_ADMIN`, portal is the intended path.
1. PermissionStore pre-authorization works for the *source* selection:
   `SetPermission screen true screencast <app-id> ['1']` (and the `screencast`
   table) suppresses source dialogs; sunshine persists a `portal_token`
   (`PERSIST_UNTIL_REVOKED`) for reuse.
1. Remaining blocker: `RemoteDesktop.Start` always shows a GNOME consent dialog
   ("远程桌面", xdg-desktop-portal-gnome) on the virtual monitor; sunshine's
   `xdg_worker` then waits (probe watchdog: 10 s). A headless host has nobody to
   click "Enable".

Candidate automations (hostd / P6 module):

- Start a uinput keyboard with the session (udev/NixOS rule for `/dev/uinput`,
  currently root-only `0600`), then press Enter on the dialog via
  `org.gnome.Mutter.RemoteDesktop` input injection.
- Run sunshine via a systemd unit with `AmbientCapabilities=CAP_SYS_ADMIN` and
  use KMS capture instead.
- Revisit upstream/GNOME for pre-authorized consent.

Dev-machine caveat: on a machine with an interactive session, the real GNOME
grabs the physical input devices and `/dev/uinput` is root-only, so consent
automation and full E2E must be exercised on a headless host (target scenario).
