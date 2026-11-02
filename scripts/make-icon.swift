// Draws Mooring's app icon and tray icon.
// Usage: swift scripts/make-icon.swift <output dir>
import AppKit

let outDir = URL(fileURLWithPath: CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : ".")

func render(size: Int, _ draw: (CGContext, CGFloat) -> Void) -> Data {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size,
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let ctx = NSGraphicsContext.current!.cgContext
    // Flip so y grows downwards, like the designs below.
    ctx.translateBy(x: 0, y: CGFloat(size))
    ctx.scaleBy(x: 1, y: -1)
    draw(ctx, CGFloat(size))
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

/// Two windows resting on a dock, with a mooring line to a bollard.
func drawMark(_ ctx: CGContext, unit u: CGFloat, color: CGColor, cutout: CGColor?) {
    ctx.setFillColor(color)
    ctx.setStrokeColor(color)
    // Back window
    let back = CGRect(x: 30 * u, y: 22 * u, width: 40 * u, height: 30 * u)
    ctx.addPath(CGPath(roundedRect: back, cornerWidth: 4 * u, cornerHeight: 4 * u, transform: nil))
    ctx.fillPath()
    // Front window, with a gap around it so the two read as separate windows
    let front = CGRect(x: 18 * u, y: 34 * u, width: 44 * u, height: 32 * u)
    if let cutout {
        ctx.setFillColor(cutout)
        ctx.addPath(CGPath(roundedRect: front.insetBy(dx: -3 * u, dy: -3 * u), cornerWidth: 6 * u, cornerHeight: 6 * u, transform: nil))
        ctx.fillPath()
        ctx.setFillColor(color)
    } else {
        ctx.setBlendMode(.clear)
        ctx.addPath(CGPath(roundedRect: front.insetBy(dx: -3 * u, dy: -3 * u), cornerWidth: 6 * u, cornerHeight: 6 * u, transform: nil))
        ctx.fillPath()
        ctx.setBlendMode(.normal)
    }
    ctx.addPath(CGPath(roundedRect: front, cornerWidth: 4 * u, cornerHeight: 4 * u, transform: nil))
    ctx.fillPath()
    // Dock
    ctx.addPath(CGPath(roundedRect: CGRect(x: 12 * u, y: 72 * u, width: 76 * u, height: 7 * u), cornerWidth: 3.5 * u, cornerHeight: 3.5 * u, transform: nil))
    ctx.fillPath()
    // Bollard
    ctx.addPath(CGPath(roundedRect: CGRect(x: 74 * u, y: 60 * u, width: 9 * u, height: 14 * u), cornerWidth: 3 * u, cornerHeight: 3 * u, transform: nil))
    ctx.fillPath()
    // Mooring line
    ctx.setLineWidth(3 * u)
    ctx.setLineCap(.round)
    ctx.move(to: CGPoint(x: 62 * u, y: 50 * u))
    ctx.addQuadCurve(to: CGPoint(x: 78.5 * u, y: 62 * u), control: CGPoint(x: 74 * u, y: 48 * u))
    ctx.strokePath()
}

// App icon: 1024px, rounded square with a deep-water gradient.
let icon = render(size: 1024) { ctx, s in
    let inset = s * 0.09
    let rect = CGRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let shape = CGPath(roundedRect: rect, cornerWidth: rect.width * 0.225, cornerHeight: rect.width * 0.225, transform: nil)
    ctx.saveGState()
    ctx.addPath(shape)
    ctx.clip()
    let colors = [CGColor(red: 0.13, green: 0.55, blue: 0.66, alpha: 1), CGColor(red: 0.04, green: 0.20, blue: 0.33, alpha: 1)]
    let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(gradient, start: CGPoint(x: 0, y: inset), end: CGPoint(x: 0, y: s - inset), options: [])
    ctx.restoreGState()

    ctx.saveGState()
    let u = rect.width / 100
    ctx.translateBy(x: rect.minX, y: rect.minY + 2 * u)
    drawMark(ctx, unit: u, color: CGColor(gray: 1, alpha: 1), cutout: nil)
    ctx.restoreGState()
}

// Tray icon: black template image (macOS tints it for light/dark menu bars).
let tray = render(size: 44) { ctx, s in
    drawMark(ctx, unit: s / 100 * 1.1, color: CGColor(gray: 0, alpha: 1), cutout: nil)
}

try! FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
try! icon.write(to: outDir.appendingPathComponent("source.png"))
try! tray.write(to: outDir.appendingPathComponent("tray-template.png"))
print("Wrote source.png and tray-template.png to \(outDir.path)")
