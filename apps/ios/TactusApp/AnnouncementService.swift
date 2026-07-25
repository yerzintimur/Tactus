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
        post(speech, priority: Self.effectivePriority(speech))
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

    private func post(_ speech: Speech, priority: SpeechPriority) {
        #if os(iOS)
        UIAccessibility.post(
            notification: .announcement,
            argument: Self.announcement(speech, priority: priority))
        #elseif os(macOS)
        // AppKit takes a plain localized string here — there is no attributed
        // form of this notification, so the per-run language tagging below is
        // iOS-only. The Mac build is our hardware-testing harness, not a
        // shipping target (docs/HARDWARE_TESTING.md).
        NSAccessibility.post(
            element: NSApp.mainWindow ?? NSApplication.shared,
            notification: .announcementRequested,
            userInfo: [
                .announcement: speech.text,
                .priority: Self.macPriority(priority).rawValue,
            ])
        #endif
    }

    #if os(iOS)
    /// The announcement to hand VoiceOver: the text, its priority, and — when the
    /// core reports more than one language run — the language of each range, so a
    /// Russian sentence quoting an English kit name is pronounced correctly on
    /// both sides (ADR-0011).
    ///
    /// `NSAttributedString` rather than Swift's `AttributedString`: the speech
    /// *language* attribute only exists as an `NSAttributedString.Key`, while the
    /// accessibility attribute scope covers priority alone.
    static func announcement(_ speech: Speech, priority: SpeechPriority) -> NSAttributedString {
        let text = NSMutableAttributedString(string: speech.text)
        let whole = NSRange(location: 0, length: text.length)
        text.addAttribute(
            .accessibilitySpeechAnnouncementPriority, value: uiPriority(priority), range: whole)

        // One language throughout needs no tagging — let VoiceOver use its own.
        guard speech.spans.count > 1 else { return text }

        var location = 0
        for span in speech.spans {
            let length = (span.text as NSString).length
            guard length > 0, location + length <= text.length else { break }
            text.addAttribute(
                .accessibilitySpeechLanguage, value: span.lang,
                range: NSRange(location: location, length: length))
            location += length
        }
        return text
    }

    private static func uiPriority(_ priority: SpeechPriority) -> UIAccessibilityPriority {
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
