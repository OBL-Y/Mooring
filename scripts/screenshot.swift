// Renders the Settings window with sample data, in light and dark, for the README.
// Usage: swift scripts/screenshot.swift <ui dir> <output dir>
import AppKit
import WebKit

let args = CommandLine.arguments
let uiDir = URL(fileURLWithPath: args.count > 1 ? args[1] : "ui")
let outDir = URL(fileURLWithPath: args.count > 2 ? args[2] : "docs/images")
let width: CGFloat = 560

// Stands in for the Rust back end so the page shows a typical Windows setup.
let mock = """
const now = Date.now() / 1000;
const data = {
  get_settings: { auto_restore: true, restore_on_undock: true, restore_desktop_icons: true,
                  launch_at_login: true, settle_delay_secs: 3, snapshot_interval_secs: 5 },
  get_status: {
    platform: "windows", supports_icons: true, has_permission: true,
    current_label: "3 displays: Built-in 1920×1200, 2560×1440, 2560×1440",
    last_event: "Restored 14 windows", last_event_at: now - 30,
    layouts: [
      { key: "a", label: "3 displays: Built-in 1920×1200, 2560×1440, 2560×1440", current: true,
        window_count: 14, icon_count: 22, saved_at: now - 4, pinned: false },
      { key: "b", label: "1 display: Built-in 1920×1200", current: false,
        window_count: 11, icon_count: 22, saved_at: now - 3 * 3600, pinned: false },
      { key: "c", label: "2 displays: Built-in 1920×1200, 3840×2160", current: false,
        window_count: 6, icon_count: 22, saved_at: now - 2 * 86400, pinned: true },
    ],
  },
};
window.__TAURI__ = { core: { invoke: async (cmd) => structuredClone(data[cmd]) } };
"""
// An off-screen web view doesn't animate, so switches would stop mid-transition.
let noTransitions = "document.head.insertAdjacentHTML('beforeend', '<style>*{transition:none!important}</style>');"

final class Shooter: NSObject, WKNavigationDelegate {
    var pending: [(String, NSAppearance.Name)] = [("settings-light.png", .aqua), ("settings-dark.png", .darkAqua)]
    let web: WKWebView
    let window: NSWindow

    override init() {
        let config = WKWebViewConfiguration()
        config.userContentController.addUserScript(WKUserScript(source: mock, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        config.userContentController.addUserScript(WKUserScript(source: noTransitions, injectionTime: .atDocumentEnd, forMainFrameOnly: true))
        web = WKWebView(frame: NSRect(x: 0, y: 0, width: width, height: 300), configuration: config)
        window = NSWindow(contentRect: web.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = web
        super.init()
        web.navigationDelegate = self
    }

    func next() {
        guard let (_, appearance) = pending.first else { exit(0) }
        web.appearance = NSAppearance(named: appearance)
        let page = uiDir.appendingPathComponent("index.html")
        web.loadFileURL(page, allowingReadAccessTo: uiDir)
    }

    func webView(_ web: WKWebView, didFinish _: WKNavigation!) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) {
            web.evaluateJavaScript("document.querySelector('main').getBoundingClientRect().bottom + 24") { h, _ in
                let height = (h as? CGFloat) ?? 1200
                self.window.setContentSize(NSSize(width: width, height: height))
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) { self.snap(height) }
            }
        }
    }

    func snap(_ height: CGFloat) {
        let config = WKSnapshotConfiguration()
        config.rect = NSRect(x: 0, y: 0, width: width, height: height)
        config.snapshotWidth = NSNumber(value: Double(width))
        web.takeSnapshot(with: config) { image, error in
            guard let image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
                  let png = rep.representation(using: .png, properties: [:]) else {
                print("Snapshot failed: \(String(describing: error))")
                exit(1)
            }
            let (name, _) = self.pending.removeFirst()
            try! png.write(to: outDir.appendingPathComponent(name))
            print("Wrote \(name) (\(rep.pixelsWide)×\(rep.pixelsHigh))")
            self.next()
        }
    }
}

try! FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
let shooter = Shooter()
shooter.next()
app.run()
