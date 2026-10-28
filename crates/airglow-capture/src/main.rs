#![forbid(unsafe_code)]
//! airglow-capture: client-side input capture.
//!
//! Skeleton (P0): lists candidate evdev devices. P2 adds EVIOCGRAB,
//! DeviceSpec generation for the control channel, and forwarding raw
//! events on the side channel.

use anyhow::Context;
use std::path::Path;

fn describe(node: &Path) -> anyhow::Result<Option<String>> {
    // /sys/class/input/eventN only links to the real input device one
    // level up (…/inputN); name and capabilities live there.
    let resolved = node.canonicalize().unwrap_or_else(|_| node.to_path_buf());
    let parent = resolved.parent().unwrap_or(&resolved);
    let name = std::fs::read_to_string(parent.join("name")).unwrap_or_default();
    let name = name.trim();
    if name.is_empty() {
        return Ok(None);
    }
    let kinds = std::fs::read_dir(parent.join("capabilities"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default();
    Ok(Some(format!("{} [{kinds}]", name)))
}

fn main() -> anyhow::Result<()> {
    let dir = Path::new("/sys/class/input");
    let entries = std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    let mut nodes: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("event"))
        })
        .collect();
    nodes.sort();

    for node in nodes {
        if let Some(desc) = describe(&node)? {
            println!("{}: {desc}", node.display());
        }
    }
    Ok(())
}
