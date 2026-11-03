//! OS-specific window access. Each backend provides the same free functions:
//! `displays`, `windows`, `apply`, `set_stacking`, `desktop_icons`, `set_desktop_icons`,
//! `has_permission`, `request_permission`, plus `NAME` and `SUPPORTS_ICONS`.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(any(target_os = "macos", windows)))]
mod unsupported;
#[cfg(not(any(target_os = "macos", windows)))]
pub use unsupported::*;

#[cfg(test)]
mod tests {
    /// Reads the real displays and windows. Run with `cargo test -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn live_read() {
        let displays = super::displays();
        println!("{displays:#?}");
        assert!(!displays.is_empty());
        println!("permission: {}", super::has_permission());
        for w in super::windows() {
            println!(
                "{:?} {:?} '{}' {}x{} at {},{}",
                w.state, w.app, w.title, w.width, w.height, w.x, w.y
            );
        }
        println!("icons: {}", super::desktop_icons().len());
    }
}
