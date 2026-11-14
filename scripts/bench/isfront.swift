// Usage: isfront <pid>; prints "front" when <pid> owns the frontmost normal window, else "behind".
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
let first = list.first { ($0[kCGWindowLayer as String] as? Int) == 0 }
print((first?[kCGWindowOwnerPID as String] as? Int32) == pid ? "front" : "behind")
