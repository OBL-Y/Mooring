//! macOS backend: CoreGraphics for displays and the window list,
//! the Accessibility (AX) API to read and move other apps' windows.

use crate::model::{Display, IconState, WinState, WindowState};
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::display::CGDisplay;
use core_graphics::geometry::{CGPoint, CGSize};
use core_graphics::window::{
    copy_window_info, kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionAll,
};
use std::collections::BTreeMap;
use std::ffi::c_void;

pub const NAME: &str = "macos";
/// Finder owns desktop icon placement and offers no API to set it.
pub const SUPPORTS_ICONS: bool = false;

type AXUIElementRef = *const c_void;
type AXValueRef = *const c_void;
type AXError = i32;
const AX_OK: AXError = 0;
const AX_VALUE_CGPOINT: u32 = 1;
const AX_VALUE_CGSIZE: u32 = 2;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        el: AXUIElementRef,
        attr: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        el: AXUIElementRef,
        attr: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetMessagingTimeout(el: AXUIElementRef, seconds: f32) -> AXError;
    fn AXUIElementPerformAction(el: AXUIElementRef, action: CFStringRef) -> AXError;
    fn AXValueCreate(kind: u32, ptr: *const c_void) -> AXValueRef;
    fn AXValueGetValue(value: AXValueRef, kind: u32, ptr: *mut c_void) -> u8;
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    // Private but stable for many years; used by most window managers to get a window's id.
    fn _AXUIElementGetWindow(el: AXUIElementRef, id: *mut u32) -> AXError;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

fn attr(name: &'static str) -> CFString {
    CFString::from_static_string(name)
}

fn copy_attr(el: AXUIElementRef, name: &'static str) -> Option<CFType> {
    let mut value: CFTypeRef = std::ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(el, attr(name).as_concrete_TypeRef(), &mut value) };
    (err == AX_OK && !value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
}

fn set_attr(el: AXUIElementRef, name: &'static str, value: CFTypeRef) -> Result<(), String> {
    let err = unsafe { AXUIElementSetAttributeValue(el, attr(name).as_concrete_TypeRef(), value) };
    if err == AX_OK {
        Ok(())
    } else {
        Err(format!("setting {name} failed (AX error {err})"))
    }
}

fn string_attr(el: AXUIElementRef, name: &'static str) -> String {
    copy_attr(el, name)
        .and_then(|v| v.downcast::<CFString>())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn bool_attr(el: AXUIElementRef, name: &'static str) -> bool {
    copy_attr(el, name)
        .and_then(|v| v.downcast::<CFBoolean>())
        .map(bool::from)
        .unwrap_or(false)
}

fn point_attr(el: AXUIElementRef) -> Option<CGPoint> {
    let v = copy_attr(el, "AXPosition")?;
    let mut p = CGPoint::new(0.0, 0.0);
    let ok = unsafe {
        AXValueGetValue(
            v.as_CFTypeRef(),
            AX_VALUE_CGPOINT,
            &mut p as *mut _ as *mut c_void,
        )
    };
    (ok != 0).then_some(p)
}

fn size_attr(el: AXUIElementRef) -> Option<CGSize> {
    let v = copy_attr(el, "AXSize")?;
    let mut s = CGSize::new(0.0, 0.0);
    let ok = unsafe {
        AXValueGetValue(
            v.as_CFTypeRef(),
            AX_VALUE_CGSIZE,
            &mut s as *mut _ as *mut c_void,
        )
    };
    (ok != 0).then_some(s)
}

fn set_position(el: AXUIElementRef, x: i32, y: i32) -> Result<(), String> {
    let p = CGPoint::new(x as f64, y as f64);
    let v = unsafe {
        CFType::wrap_under_create_rule(AXValueCreate(
            AX_VALUE_CGPOINT,
            &p as *const _ as *const c_void,
        ))
    };
    set_attr(el, "AXPosition", v.as_CFTypeRef())
}

fn set_size(el: AXUIElementRef, w: i32, h: i32) -> Result<(), String> {
    let s = CGSize::new(w as f64, h as f64);
    let v = unsafe {
        CFType::wrap_under_create_rule(AXValueCreate(
            AX_VALUE_CGSIZE,
            &s as *const _ as *const c_void,
        ))
    };
    set_attr(el, "AXSize", v.as_CFTypeRef())
}

/// An app's AX element, released on drop.
struct AppElement(CFType);

impl AppElement {
    fn new(pid: u32) -> Self {
        let el = unsafe { AXUIElementCreateApplication(pid as i32) };
        unsafe { AXUIElementSetMessagingTimeout(el, 0.5) };
        Self(unsafe { CFType::wrap_under_create_rule(el) })
    }

    /// The app's windows as AX elements (kept alive by the returned array).
    fn windows(&self) -> Vec<CFType> {
        let Some(list) = copy_attr(self.0.as_CFTypeRef(), "AXWindows") else {
            return Vec::new();
        };
        let array: CFArray<CFType> =
            unsafe { CFArray::wrap_under_get_rule(list.as_CFTypeRef() as CFArrayRef) };
        array.iter().map(|w| w.clone()).collect()
    }
}

fn window_id(el: AXUIElementRef) -> u64 {
    let mut id = 0u32;
    if unsafe { _AXUIElementGetWindow(el, &mut id) } == AX_OK {
        id as u64
    } else {
        0
    }
}

fn dict_i64(d: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<i64> {
    d.find(attr(key))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_i64())
}

fn dict_string(d: &CFDictionary<CFString, CFType>, key: &'static str) -> Option<String> {
    d.find(attr(key))
        .and_then(|v| v.downcast::<CFString>())
        .map(|s| s.to_string())
}

struct AppInfo {
    name: String,
    /// Front-to-back position of each of the app's on-screen windows.
    order: BTreeMap<u64, u32>,
}

/// Apps that own normal (layer 0) windows, with z-order for each window.
fn window_owners() -> BTreeMap<u32, AppInfo> {
    let own_pid = std::process::id();
    let mut apps: BTreeMap<u32, AppInfo> = BTreeMap::new();
    let Some(list) = copy_window_info(
        kCGWindowListOptionAll | kCGWindowListExcludeDesktopElements,
        kCGNullWindowID,
    ) else {
        return apps;
    };
    for (index, item) in list.iter().enumerate() {
        let d: CFDictionary<CFString, CFType> =
            unsafe { CFDictionary::wrap_under_get_rule(*item as CFDictionaryRef) };
        if dict_i64(&d, "kCGWindowLayer") != Some(0) {
            continue;
        }
        let Some(pid) = dict_i64(&d, "kCGWindowOwnerPID").map(|p| p as u32) else {
            continue;
        };
        if pid == own_pid {
            continue;
        }
        let app = apps.entry(pid).or_insert_with(|| AppInfo {
            name: dict_string(&d, "kCGWindowOwnerName").unwrap_or_default(),
            order: BTreeMap::new(),
        });
        if let Some(id) = dict_i64(&d, "kCGWindowNumber") {
            app.order.insert(id as u64, index as u32);
        }
    }
    apps
}

pub fn displays() -> Vec<Display> {
    let Ok(ids) = CGDisplay::active_displays() else {
        return Vec::new();
    };
    ids.into_iter()
        .map(|id| {
            let d = CGDisplay::new(id);
            let b = d.bounds();
            let builtin = d.is_builtin();
            Display {
                id: format!(
                    "{:x}-{:x}-{:x}",
                    d.vendor_number(),
                    d.model_number(),
                    d.serial_number()
                ),
                name: if builtin {
                    "Built-in display".into()
                } else {
                    format!("Display {id}")
                },
                x: b.origin.x as i32,
                y: b.origin.y as i32,
                width: b.size.width as i32,
                height: b.size.height as i32,
                primary: d.is_main(),
                builtin,
            }
        })
        .collect()
}

pub fn windows() -> Vec<WindowState> {
    if !has_permission() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (pid, info) in window_owners() {
        let app = AppElement::new(pid);
        for w in app.windows() {
            let el = w.as_CFTypeRef();
            let subrole = string_attr(el, "AXSubrole");
            if subrole != "AXStandardWindow" && subrole != "AXDialog" {
                continue;
            }
            // Full-screen windows live in their own Space; leave them alone.
            if bool_attr(el, "AXFullScreen") {
                continue;
            }
            let (Some(pos), Some(size)) = (point_attr(el), size_attr(el)) else {
                continue;
            };
            let id = window_id(el);
            out.push(WindowState {
                id,
                pid,
                app: info.name.clone(),
                title: string_attr(el, "AXTitle"),
                class: subrole,
                x: pos.x.round() as i32,
                y: pos.y.round() as i32,
                width: size.width.round() as i32,
                height: size.height.round() as i32,
                state: if bool_attr(el, "AXMinimized") {
                    WinState::Minimized
                } else {
                    WinState::Normal
                },
                order: info.order.get(&id).copied().unwrap_or(u32::MAX),
            });
        }
    }
    out
}

/// Ids of every app window that exists, including ones on other Spaces that
/// the Accessibility API doesn't list.
pub fn existing_windows() -> std::collections::HashSet<u64> {
    window_owners()
        .into_values()
        .flat_map(|app| app.order.into_keys())
        .collect()
}

pub fn apply(target: &WindowState, current: &WindowState) -> Result<(), String> {
    let app = AppElement::new(current.pid);
    let windows = app.windows();
    let w = windows
        .iter()
        .find(|w| current.id != 0 && window_id(w.as_CFTypeRef()) == current.id)
        .or_else(|| {
            windows
                .iter()
                .find(|w| string_attr(w.as_CFTypeRef(), "AXTitle") == current.title)
        })
        .ok_or("window has closed")?;
    let el = w.as_CFTypeRef();

    let minimized = current.state == WinState::Minimized;
    if minimized && target.state != WinState::Minimized {
        set_attr(el, "AXMinimized", CFBoolean::false_value().as_CFTypeRef())?;
    }
    // Move, resize, then move again: the size can be clamped to the display the
    // window is on, and the position can be clamped by the old size.
    set_position(el, target.x, target.y)?;
    set_size(el, target.width, target.height)?;
    set_position(el, target.x, target.y)?;
    if !minimized && target.state == WinState::Minimized {
        set_attr(el, "AXMinimized", CFBoolean::true_value().as_CFTypeRef())?;
    }
    Ok(())
}

/// Puts windows in the given front-to-back order, back first. Each window's app
/// is brought forward and the window raised, so this briefly changes the
/// focused app. Returns how many windows were raised.
pub fn set_stacking(front_to_back: &[WindowState]) -> usize {
    let mut raised = 0;
    for target in front_to_back.iter().rev() {
        if target.state == WinState::Minimized || target.id == 0 {
            continue;
        }
        let app = AppElement::new(target.pid);
        let windows = app.windows();
        let Some(w) = windows
            .iter()
            .find(|w| window_id(w.as_CFTypeRef()) == target.id)
        else {
            continue;
        };
        let _ = set_attr(
            app.0.as_CFTypeRef(),
            "AXFrontmost",
            CFBoolean::true_value().as_CFTypeRef(),
        );
        let raise = attr("AXRaise");
        if unsafe { AXUIElementPerformAction(w.as_CFTypeRef(), raise.as_concrete_TypeRef()) }
            == AX_OK
        {
            raised += 1;
        }
        // Give the window server a moment, or later raises can land first.
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    raised
}

pub fn desktop_icons() -> Vec<IconState> {
    Vec::new()
}

pub fn set_desktop_icons(_icons: &[IconState]) -> Result<usize, String> {
    Err("Desktop icon positions can't be set on macOS".into())
}

pub fn has_permission() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Shows the system prompt that sends the user to Privacy & Security > Accessibility.
pub fn request_permission() {
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options =
        CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
}
