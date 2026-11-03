use crate::model::{Display, IconState, WindowState};

pub const NAME: &str = "other";
pub const SUPPORTS_ICONS: bool = false;

pub fn displays() -> Vec<Display> {
    Vec::new()
}
pub fn windows() -> Vec<WindowState> {
    Vec::new()
}
pub fn apply(_target: &WindowState, _current: &WindowState) -> Result<(), String> {
    Err("unsupported platform".into())
}
pub fn set_stacking(_front_to_back: &[WindowState]) -> usize {
    0
}

pub fn desktop_icons() -> Vec<IconState> {
    Vec::new()
}
pub fn set_desktop_icons(_icons: &[IconState]) -> Result<usize, String> {
    Err("unsupported platform".into())
}
pub fn has_permission() -> bool {
    true
}
pub fn request_permission() {}
