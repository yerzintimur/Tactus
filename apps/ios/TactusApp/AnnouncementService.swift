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
/// *announce* the changes the screen reader cannot observe itself (a
/// device-initiated edit, the connection lifecycle, a rejected edit); everything
/// the screen reader can already see (navigation, the focused control's value) the
/// app leaves to it, silently.
///
/// When no screen reader is running these posts are inert — a sighted user gets the
/// visual UI plus earcons/haptics and turns on VoiceOver if they want speech. We do
/// not reinvent the system's accessibility features; we feed them.
///
/// `SpeechPriority` maps to the platform announcement priority, so a newer
/// navigation announcement can interrupt a stale one (ADR-0014, §3).
@MainActor
final class AnnouncementService {
    func announce(_ speech: Speech) {
        #if os(iOS)
        var text = AttributedString(speech.text)
        text.accessibilitySpeechAnnouncementPriority = Self.iosPriority(speech.priority)
        UIAccessibility.post(notification: .announcement, argument: text)
        #elseif os(macOS)
        NSAccessibility.post(
            element: NSApp.mainWindow ?? NSApplication.shared,
            notification: .announcementRequested,
            userInfo: [
                .announcement: speech.text,
                .priority: Self.macPriority(speech.priority).rawValue,
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
