// Usage: winwait <pid> <timeout-seconds>; prints seconds until <pid> owns an on-screen window
// at least 200x200 (the main window, not a menu-bar item), or "timeout".
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let timeout = Double(CommandLine.arguments[2])!
let start = Date()
while Date().timeIntervalSince(start) < timeout {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
    for w in list where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
        if let b = w[kCGWindowBounds as String] as? [String: Double], (b["Width"] ?? 0) >= 200, (b["Height"] ?? 0) >= 200 {
            print(String(format: "%.3f", Date().timeIntervalSince(start))); exit(0)
        }
    }
    usleep(5000)
}
print("timeout")
