#if canImport(UIKit)
import UIKit
#endif
#if canImport(AppKit)
import AppKit
#endif
import Tactus

/// Routes the core's localized messages to the **system screen reader's**
/// announcement channel (VoiceOver on iOS and macOS). It is *not* a text-to-speech
/// engine: the app never synthesizes a voice of its own.
///
/// Per ADR-0014 the screen reader is the single voice — the user's own, configured
/// with their chosen voice, rate, and verbosity. The app's only job is to
/// *announce* the changes the screen reader cannot observe itself; everything the
/// screen reader can already see (navigation, the focused control's value) the app
/// leaves to it, silently. The core tags each message with a `category` + `source`
/// (ADR-0014), and this router decides:
///
/// - `paramEdit` + `userInitiated` → **suppressed**: the screen reader voices the
///   focused control's new value itself — announcing it too would double-speak.
///   (The confirmation earcon still plays; `EarconService` is a separate channel.)
/// - `kitNav` → announced **interrupting** (high priority): a newer kit
///   announcement preempts the previous one, so a slow scroll voices each kit and
///   a fast scroll leaves the one you settled on — interruption, not debouncing.
/// - everything else (connection lifecycle, device-initiated changes, errors,
///   info tails) → announced with the core's priority.
///
/// When no screen reader is running these posts are inert — a sighted user gets
/// the visual UI plus earcons/haptics and turns on VoiceOver if they want speech.
/// We do not reinvent the system's accessibility features; we feed them.
@MainActor
final class AnnouncementService {
    func announce(_ speech: Speech) {
        guard Self.shouldAnnounce(speech) else { return }
        post(speech.text, priority: Self.effectivePriority(speech))
    }

    /// ADR-0014 §4 — no double speech: the screen reader already voices the
    /// focused control after a user-initiated edit.
    static func shouldAnnounce(_ speech: Speech) -> Bool {
        !(speech.category == .paramEdit && speech.source == .userInitiated)
    }

    /// ADR-0014 §3 — kit navigation interrupts; everything else keeps the core's
    /// priority (high already interrupts, low may be dropped by the system).
    static func effectivePriority(_ speech: Speech) -> SpeechPriority {
        speech.category == .kitNav ? .high : speech.priority
    }

    private func post(_ text: String, priority: SpeechPriority) {
        #if os(iOS)
        var attributed = AttributedString(text)
        attributed.accessibilitySpeechAnnouncementPriority = Self.iosPriority(priority)
        UIAccessibility.post(notification: .announcement, argument: attributed)
        #elseif os(macOS)
        NSAccessibility.post(
            element: NSApp.mainWindow ?? NSApplication.shared,
            notification: .announcementRequested,
            userInfo: [
                .announcement: text,
                .priority: Self.macPriority(priority).rawValue,
            ])
        #endif
    }

    #if os(iOS)
    private static func iosPriority(
        _ priority: SpeechPriority
    ) -> AttributeScopes.AccessibilityAttributes.AnnouncementPriorityAttribute.Value {
        switch priority {
        case .high: .high
        case .default: .default
        case .low: .low
        }
    }
    #elseif os(macOS)
    private static func macPriority(_ priority: SpeechPriority) -> NSAccessibilityPriorityLevel {
        switch priority {
        case .high: .high
        case .default: .medium
        case .low: .low
        }
    }
    #endif
}
