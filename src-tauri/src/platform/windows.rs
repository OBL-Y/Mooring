//! Windows backend: Win32 for displays and windows, the shell's own
//! IFolderView (the desktop's folder view) for desktop icon positions.

use crate::model::{Display, IconState, WinState, WindowState};
use std::ffi::c_void;
use windows::core::{Interface, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider, CLSCTX_ALL,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellWindows, IVirtualDesktopManager, SID_STopLevelBrowser,
    ShellWindows, StrRetToStrW, VirtualDesktopManager, SHGDNF, SHGDN_FORPARSING, SHGDN_INFOLDER,
    SVGIO_ALLVIEW, SVSI_POSITIONITEM, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindow, GetWindowLongPtrW, GetWindowPlacement, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, SetWindowPlacement,
    SetWindowPos, GWL_EXSTYLE, GW_OWNER, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, WINDOWPLACEMENT,
    WPF_ASYNCWINDOWPLACEMENT, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW,
};

pub const NAME: &str = "windows";
pub const SUPPORTS_ICONS: bool = true;

const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 0x1;
const CSIDL_DESKTOP: i32 = 0;
const SKIP_CLASSES: &[&str] = &[
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
];

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn hwnd_of(id: u64) -> HWND {
    HWND(id as usize as *mut c_void)
}

// ---------- Displays ----------

unsafe extern "system" fn monitor_proc(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let out = &mut *(data.0 as *mut Vec<Display>);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if !GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO).as_bool() {
        return true.into();
    }
    let device = wide_to_string(&info.szDevice);

    // The device interface name includes the monitor's EDID id, which stays the
    // same across reconnects, unlike \\.\DISPLAYn.
    let mut dd = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    let id = if EnumDisplayDevicesW(
        PCWSTR(info.szDevice.as_ptr()),
        0,
        &mut dd,
        EDD_GET_DEVICE_INTERFACE_NAME,
    )
    .as_bool()
    {
        wide_to_string(&dd.DeviceID)
    } else {
        device.clone()
    };
    let name = wide_to_string(&dd.DeviceString);
    let r = info.monitorInfo.rcMonitor;
    out.push(Display {
        builtin: is_builtin(&id),
        id,
        name: if name.is_empty() { device } else { name },
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
        primary: info.monitorInfo.dwFlags & 1 != 0, // MONITORINFOF_PRIMARY
    });
    true.into()
}

/// Laptop panels usually report one of these in their device id.
fn is_builtin(id: &str) -> bool {
    let upper = id.to_ascii_uppercase();
    [
        "#SHP", "#LGD", "#AUO", "#BOE", "#SDC", "#CMN", "#LEN", "#IVO", "#MS_",
    ]
    .iter()
    .any(|p| upper.contains(p))
}

pub fn displays() -> Vec<Display> {
    let mut out: Vec<Display> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(monitor_proc),
            LPARAM(&mut out as *mut _ as isize),
        );
    }
    out
}

// ---------- Windows ----------

fn process_path(pid: u32) -> String {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if ok {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
    }
}

/// True for a window that's cloaked only because it's on another virtual
/// desktop. Those are real windows Mooring should save and restore; other
/// cloaked windows (suspended Store apps, hidden shell windows) are not.
fn on_other_desktop(desktops: Option<&IVirtualDesktopManager>, hwnd: HWND) -> bool {
    let Some(desktops) = desktops else {
        return false;
    };
    unsafe {
        desktops
            .IsWindowOnCurrentVirtualDesktop(hwnd)
            .is_ok_and(|on_current| !on_current.as_bool())
            && desktops
                .GetWindowDesktopId(hwnd)
                .is_ok_and(|id| id != windows::core::GUID::zeroed())
    }
}

struct Enum {
    own_pid: u32,
    desktops: Option<IVirtualDesktopManager>,
    out: Vec<WindowState>,
}

unsafe extern "system" fn window_proc(hwnd: HWND, data: LPARAM) -> BOOL {
    let state = &mut *(data.0 as *mut Enum);
    let order = state.out.len() as u32;
    if let Some(w) = read_window(hwnd, state.own_pid, order, state.desktops.as_ref()) {
        state.out.push(w);
    }
    true.into()
}

unsafe fn read_window(
    hwnd: HWND,
    own_pid: u32,
    order: u32,
    desktops: Option<&IVirtualDesktopManager>,
) -> Option<WindowState> {
    if !IsWindowVisible(hwnd).as_bool() || (is_cloaked(hwnd) && !on_other_desktop(desktops, hwnd)) {
        return None;
    }
    // Only real app windows: unowned (or explicitly on the taskbar), not tool windows.
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    let owned = GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid());
    if ex & WS_EX_TOOLWINDOW.0 != 0 || (owned && ex & WS_EX_APPWINDOW.0 == 0) {
        return None;
    }
    let mut title = [0u16; 512];
    let n = GetWindowTextW(hwnd, &mut title);
    if n <= 0 {
        return None;
    }
    let mut class = [0u16; 256];
    let c = GetClassNameW(hwnd, &mut class);
    let class = String::from_utf16_lossy(&class[..c.max(0) as usize]);
    if SKIP_CLASSES.contains(&class.as_str()) {
        return None;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == own_pid {
        return None;
    }
    let mut wp = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    GetWindowPlacement(hwnd, &mut wp).ok()?;
    let r = wp.rcNormalPosition;
    let state = if IsIconic(hwnd).as_bool() {
        WinState::Minimized
    } else if IsZoomed(hwnd).as_bool() {
        WinState::Maximized
    } else {
        WinState::Normal
    };
    Some(WindowState {
        id: hwnd.0 as usize as u64,
        pid,
        app: process_path(pid),
        title: String::from_utf16_lossy(&title[..n as usize]),
        class,
        // Note: workspace coordinates, as used by Get/SetWindowPlacement.
        x: r.left,
        y: r.top,
        width: r.right - r.left,
        height: r.bottom - r.top,
        state,
        order,
    })
}

pub fn windows() -> Vec<WindowState> {
    let _com = Com::init();
    let mut state = Enum {
        own_pid: std::process::id(),
        desktops: unsafe { CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL) }.ok(),
        out: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(window_proc), LPARAM(&mut state as *mut _ as isize));
    }
    // Release the COM object before COM is uninitialized.
    let out = std::mem::take(&mut state.out);
    drop(state);
    out
}

unsafe extern "system" fn collect_proc(hwnd: HWND, data: LPARAM) -> BOOL {
    let ids = &mut *(data.0 as *mut std::collections::HashSet<u64>);
    ids.insert(hwnd.0 as usize as u64);
    true.into()
}

/// Ids of every top-level window that exists, seen or not.
pub fn existing_windows() -> std::collections::HashSet<u64> {
    let mut ids = std::collections::HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(collect_proc), LPARAM(&mut ids as *mut _ as isize));
    }
    ids
}

pub fn apply(target: &WindowState, current: &WindowState) -> Result<(), String> {
    let hwnd = hwnd_of(current.id);
    unsafe {
        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(hwnd, &mut wp).map_err(|e| e.to_string())?;
        wp.flags = WPF_ASYNCWINDOWPLACEMENT;
        wp.rcNormalPosition = RECT {
            left: target.x,
            top: target.y,
            right: target.x + target.width,
            bottom: target.y + target.height,
        };
        match target.state {
            WinState::Maximized => {
                // A maximized window only moves monitors if it is restored onto the
                // new monitor first, then maximized there.
                if current.state == WinState::Maximized {
                    wp.showCmd = SW_SHOWNOACTIVATE.0 as u32;
                    SetWindowPlacement(hwnd, &wp).map_err(|e| e.to_string())?;
                }
                wp.showCmd = SW_SHOWMAXIMIZED.0 as u32;
            }
            WinState::Minimized => wp.showCmd = SW_SHOWMINNOACTIVE.0 as u32,
            WinState::Normal => wp.showCmd = SW_SHOWNOACTIVATE.0 as u32,
        }
        // Fails for windows of elevated apps when Mooring isn't elevated.
        SetWindowPlacement(hwnd, &wp).map_err(|e| e.to_string())
    }
}

/// Puts windows in the given front-to-back order without activating any of
/// them: the first goes on top, each next one directly behind the previous.
/// Returns how many windows were placed.
pub fn set_stacking(front_to_back: &[WindowState]) -> usize {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    let mut above = HWND_TOP;
    let mut placed = 0;
    for target in front_to_back {
        if target.state == WinState::Minimized {
            continue;
        }
        let hwnd = hwnd_of(target.id);
        // Fails for windows of elevated apps when Mooring isn't elevated.
        if unsafe { SetWindowPos(hwnd, Some(above), 0, 0, 0, 0, flags) }.is_ok() {
            above = hwnd;
            placed += 1;
        }
    }
    placed
}

// ---------- Desktop icons ----------

/// Initializes COM for the calling thread for as long as it lives.
struct Com(bool);

impl Com {
    fn init() -> Self {
        Self(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() })
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

/// The desktop's folder view, found the way Explorer exposes it to automation.
fn desktop_view() -> windows::core::Result<IFolderView> {
    unsafe {
        let shell: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
        let loc = VARIANT::from(CSIDL_DESKTOP);
        let empty = VARIANT::default();
        let mut hwnd = 0i32;
        let disp = shell.FindWindowSW(&loc, &empty, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
        let provider: IServiceProvider = disp.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view = browser.QueryActiveShellView()?;
        view.cast()
    }
}

/// Calls `f` for each desktop item with its pidl and parsing name.
fn for_each_icon(
    view: &IFolderView,
    mut f: impl FnMut(*mut ITEMIDLIST, String),
) -> windows::core::Result<()> {
    unsafe {
        let folder: windows::Win32::UI::Shell::IShellFolder = view.GetFolder()?;
        let items: windows::Win32::UI::Shell::IEnumIDList = view.Items(SVGIO_ALLVIEW)?;
        loop {
            let mut pidl = [std::ptr::null_mut::<ITEMIDLIST>(); 1];
            let mut fetched = 0u32;
            if items.Next(&mut pidl, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let item = pidl[0];
            let mut name = Default::default();
            if folder
                .GetDisplayNameOf(
                    item,
                    SHGDNF(SHGDN_INFOLDER.0 | SHGDN_FORPARSING.0),
                    &mut name,
                )
                .is_ok()
            {
                let mut text = PWSTR::null();
                if StrRetToStrW(&mut name, Some(item), &mut text).is_ok() {
                    f(item, text.to_string().unwrap_or_default());
                    CoTaskMemFree(Some(text.0 as *const c_void));
                }
            }
            CoTaskMemFree(Some(item as *const c_void));
        }
        Ok(())
    }
}

pub fn desktop_icons() -> Vec<IconState> {
    let _com = Com::init();
    let mut out = Vec::new();
    let Ok(view) = desktop_view() else {
        return out;
    };
    let _ = for_each_icon(&view, |item, name| {
        if let Ok(pt) = unsafe { view.GetItemPosition(item) } {
            out.push(IconState {
                name,
                x: pt.x,
                y: pt.y,
            });
        }
    });
    out
}

pub fn set_desktop_icons(icons: &[IconState]) -> Result<usize, String> {
    let _com = Com::init();
    let view = desktop_view().map_err(|e| e.to_string())?;
    let mut moved = 0;
    for_each_icon(&view, |item, name| {
        if let Some(saved) = icons.iter().find(|i| i.name == name) {
            let pt = POINT {
                x: saved.x,
                y: saved.y,
            };
            let ok = unsafe {
                view.SelectAndPositionItems(
                    1,
                    &(item as *const ITEMIDLIST),
                    Some(&pt),
                    SVSI_POSITIONITEM.0 as u32,
                )
            };
            if ok.is_ok() {
                moved += 1;
            }
        }
    })
    .map_err(|e| e.to_string())?;
    Ok(moved)
}

pub fn has_permission() -> bool {
    true
}

pub fn request_permission() {}
