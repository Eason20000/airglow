//! Wire types shared by the airglow control plane.
//!
//! The control plane is a line-delimited JSON protocol spoken between the
//! client wrapper (`airglow-session`) and the host daemon (`airglow-hostd`)
//! before Moonlight/Sunshine streaming starts. The client declares the
//! display mode and its input devices; the host builds a virtual monitor and
//! uinput templates to match.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

/// Display mode requested by the client. The host creates the virtual
/// monitor with this geometry so the session renders natively at the
/// client resolution (no scaling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    /// Refresh rate in Hz.
    pub refresh: u32,
    /// Integer fraction scale, e.g. 100 for 1x, 200 for 2x.
    pub scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    H264,
    Hevc,
    Av1,
}

/// Video encoding preferences negotiated with the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSpec {
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub codec: Codec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Keyboard,
    Pointer,
    Touchscreen,
    Touchpad,
    Pen,
}

/// Declarative description of one client-side input device. The host
/// mirrors it as a uinput device so the server-side libinput stack sees a
/// device with the same properties as the physical one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSpec {
    pub kind: DeviceKind,
    pub name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub max_contacts: Option<u8>,
}

/// First message sent by the client after connecting the control channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    pub modes: Vec<Mode>,
    pub video: VideoSpec,
    pub devices: Vec<DeviceSpec>,
    /// Unix user to start the session for; host policy decides.
    pub user: String,
}

/// Host answer to `Hello`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum Ready {
    Ready {
        host_id: String,
        /// Sunshine/Moonlight endpoint the client should connect to.
        stream_addr: String,
        /// PAM authentication is required before the session starts.
        auth_required: bool,
    },
    Error {
        message: String,
    },
}

/// Snapshot the host writes for `airglowctl status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionState {
    pub user: String,
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub wayland_display: String,
    pub started_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostState {
    pub host_id: String,
    pub listen: String,
    pub session: Option<SessionState>,
}
