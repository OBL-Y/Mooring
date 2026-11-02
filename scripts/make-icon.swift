// Draws Mooring's app icon and tray icon: an anchor with a window as its stock.
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
    // Flip so y grows downwards, matching the 100×100 design grid below.
    ctx.translateBy(x: 0, y: CGFloat(size))
    ctx.scaleBy(x: 1, y: -1)
    draw(ctx, CGFloat(size))
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

let white = CGColor(gray: 1, alpha: 1)
let navy = CGColor(red: 0x0C / 255, green: 0x44 / 255, blue: 0x7C / 255, alpha: 1)
let paleBlue = CGColor(red: 0xB5 / 255, green: 0xD4 / 255, blue: 0xF4 / 255, alpha: 1)

/// The anchor, on a 100×100 grid. `weight` thickens strokes for tiny sizes.
/// With `titleBar` nil the title bar is cut out (for one-colour template images).
func drawAnchor(_ ctx: CGContext, color: CGColor, titleBar: CGColor?, weight: CGFloat = 1) {
    ctx.setStrokeColor(color)
    ctx.setFillColor(color)
    ctx.setLineCap(.round)

    // Ring
    ctx.setLineWidth(4.5 * weight)
    ctx.strokeEllipse(in: CGRect(x: 44, y: 14, width: 12, height: 12))

    // Shank, above and below the window
    ctx.setLineWidth(5 * weight)
    ctx.setLineCap(.butt)
    ctx.move(to: CGPoint(x: 50, y: 26)); ctx.addLine(to: CGPoint(x: 50, y: 35))
    ctx.move(to: CGPoint(x: 50, y: 60)); ctx.addLine(to: CGPoint(x: 50, y: 82))
    ctx.strokePath()

    // Arms
    ctx.setLineCap(.round)
    ctx.move(to: CGPoint(x: 22, y: 62))
    ctx.addQuadCurve(to: CGPoint(x: 50, y: 84), control: CGPoint(x: 24, y: 84))
    ctx.addQuadCurve(to: CGPoint(x: 78, y: 62), control: CGPoint(x: 76, y: 84))
    ctx.strokePath()

    // Flukes
    let w = weight
    for x in [22.0, 78.0] as [CGFloat] {
        ctx.move(to: CGPoint(x: x - 6 * w, y: 66))
        ctx.addLine(to: CGPoint(x: x, y: 56 - 2 * (w - 1)))
        ctx.addLine(to: CGPoint(x: x + 6 * w, y: 66))
        ctx.closePath()
    }
    ctx.fillPath()

    // Window as the stock, with a title bar
    let window = CGRect(x: 30, y: 34, width: 40, height: 27)
    let shape = CGPath(roundedRect: window, cornerWidth: 4, cornerHeight: 4, transform: nil)
    ctx.addPath(shape)
    ctx.fillPath()
    ctx.saveGState()
    ctx.addPath(shape)
    ctx.clip()
    let bar = CGRect(x: 30, y: 34, width: 40, height: 7 * weight)
    if let titleBar {
        ctx.setFillColor(titleBar)
        ctx.fill(bar)
    } else {
        ctx.setBlendMode(.clear)
        ctx.fill(bar.insetBy(dx: 3 * weight, dy: 0).offsetBy(dx: 0, dy: 2.5 * weight).intersection(window.insetBy(dx: 3 * weight, dy: 2.5 * weight)))
    }
    ctx.restoreGState()
}

// App icon: 1024px rounded square in deep blue. The design grid's
// background square (4…96) maps onto the standard macOS icon area.
let icon = render(size: 1024) { ctx, s in
    let inset = s * 0.09
    let rect = CGRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    ctx.addPath(CGPath(roundedRect: rect, cornerWidth: rect.width * 0.225, cornerHeight: rect.width * 0.225, transform: nil))
    ctx.setFillColor(navy)
    ctx.fillPath()

    let u = rect.width / 92
    ctx.translateBy(x: inset - 4 * u, y: inset - 4 * u)
    ctx.scaleBy(x: u, y: u)
    drawAnchor(ctx, color: white, titleBar: paleBlue)
}

// Tray icon: black template image (macOS tints it for light and dark menu bars).
let tray = render(size: 44) { ctx, s in
    let u = s / 78
    ctx.translateBy(x: s / 2 - 50 * u, y: s / 2 - 50 * u)
    ctx.scaleBy(x: u, y: u)
    drawAnchor(ctx, color: CGColor(gray: 0, alpha: 1), titleBar: nil, weight: 1.35)
}

try! FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
try! icon.write(to: outDir.appendingPathComponent("source.png"))
try! tray.write(to: outDir.appendingPathComponent("tray-template.png"))
print("Wrote source.png and tray-template.png to \(outDir.path)")
