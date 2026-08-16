//! Finding the collector this build ships with.
//!
//! athar used to install a copy of the collector and register it with `launchd`,
//! so scans happened whether or not the window was open. That bought a guarantee
//! and cost a scheduler: an install step, a copy that could fall behind the app,
//! and a mechanism that exists only on macOS.
//!
//! The window now runs the collector itself, while it is open. Nothing is
//! installed, nothing can go stale, and the only platform-specific thing left is
//! where a bundle puts its sidecar.

use std::path::{Path, PathBuf};

/// The collector shipped beside `current`, for a caller that is not itself one.
///
/// The bundled name differs from the CLI's on purpose. A macOS bundle puts the
/// app's executable in the same directory as its sidecars, and the app is called
/// `athar` — so a sidecar with that name could not exist there, and a lookup by
/// that name would find the window instead. That is the bug this whole path
/// exists to prevent, so the fallback refuses to return `current` itself.
pub fn beside(current: &Path) -> Option<PathBuf> {
    let dir = current.parent()?;

    let bundled = dir.join(SIDECAR_BIN);
    if bundled.is_file() {
        return Some(bundled);
    }
    // A development tree, where the CLI is built next to the window under target/.
    let sibling = dir.join(COLLECTOR_BIN);
    if sibling.is_file() && sibling != current {
        return Some(sibling);
    }
    None
}

/// What the CLI is called, and what it is called inside the app bundle.
const COLLECTOR_BIN: &str = "athar";
const SIDECAR_BIN: &str = "athar-collector";

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a macOS bundle: the app's executable and its sidecars share
    /// one directory, and the app here is called `athar`, exactly like the CLI.
    #[test]
    fn an_app_named_like_the_collector_never_resolves_to_itself() {
        let dir = std::env::temp_dir().join(format!("athar-beside-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let app = dir.join("athar");
        std::fs::write(&app, b"the window").unwrap();

        // Only the app is present: this is the state that scheduled a window as
        // the collector, and it must resolve to nothing at all.
        assert_eq!(beside(&app), None, "the app must never be its own collector");

        let sidecar = dir.join("athar-collector");
        std::fs::write(&sidecar, b"the collector").unwrap();
        assert_eq!(beside(&app), Some(sidecar));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A development tree: the window is `athar-desktop` and the CLI is `athar`.
    #[test]
    fn a_build_tree_resolves_the_cli_beside_the_window() {
        let dir = std::env::temp_dir().join(format!("athar-beside-dev-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let window = dir.join("athar-desktop");
        let cli = dir.join("athar");
        std::fs::write(&window, b"the window").unwrap();
        std::fs::write(&cli, b"the collector").unwrap();

        assert_eq!(beside(&window), Some(cli));
        std::fs::remove_dir_all(&dir).ok();
    }
}
