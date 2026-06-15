import AVFoundation
import Tactus

#if canImport(UIKit)
import UIKit
#endif
#if canImport(AppKit)
import AppKit
#endif

/// Speaks the localized strings the core emits (`Speech { text, priority }`).
///
/// Nonvisual-first: when the user already runs a **screen reader** (VoiceOver), we
/// route through the platform's accessibility announcements so our speech queues
/// with it and honors the user's chosen voice and rate — we don't fight the screen
/// reader with a second TTS. When it's off, we speak with `AVSpeechSynthesizer`.
///
/// `SpeechPriority` maps to interrupt-vs-enqueue (synth) and to announcement
/// priority (screen reader).
///
/// The core's strings are currently single-language per utterance; per-segment
/// language tagging (ADR-0011) is a later enhancement.
@MainActor
final class SpeechService {
    private let synth = AVSpeechSynthesizer()
    private var voiceLanguage: String
    private var voice: AVSpeechSynthesisVoice?

    init(locale: String) {
        voiceLanguage = Self.bcp47(locale)
        selectVoice()
        configureAudioSession()
    }

    func setLocale(_ locale: String) {
        voiceLanguage = Self.bcp47(locale)
        selectVoice()
    }

    /// Prefer the highest-quality installed voice for the language (Premium >
    /// Enhanced); fall back to the system default if none is downloaded. The good
    /// voices are opt-in downloads (System Settings → Accessibility → Spoken
    /// Content → System Voice → Manage Voices); the bundled Compact voice is the
    /// robotic one. The default-quality tier is skipped to avoid novelty voices.
    private func selectVoice() {
        let language = voiceLanguage
        let prefix = String(language.prefix(2))
        let best = AVSpeechSynthesisVoice.speechVoices()
            .filter {
                ($0.language == language || $0.language.hasPrefix(prefix))
                    && $0.quality.rawValue > AVSpeechSynthesisVoiceQuality.default.rawValue
            }
            .sorted {
                $0.quality.rawValue != $1.quality.rawValue
                    ? $0.quality.rawValue > $1.quality.rawValue
                    : ($0.language == language && $1.language != language)
            }
            .first
        voice = best ?? AVSpeechSynthesisVoice(language: language)
    }

    func speak(_ speech: Speech) {
        if Self.isScreenReaderRunning {
            announce(speech)
        } else {
            synthesize(speech)
        }
    }

    // MARK: - Screen-reader path

    private static var isScreenReaderRunning: Bool {
        #if os(iOS)
        return UIAccessibility.isVoiceOverRunning
        #elseif os(macOS)
        return NSWorkspace.shared.isVoiceOverEnabled
        #else
        return false
        #endif
    }

    private func announce(_ speech: Speech) {
        #if os(iOS)
        var text = AttributedString(speech.text)
        text.accessibilitySpeechAnnouncementPriority = iosPriority(speech.priority)
        UIAccessibility.post(notification: .announcement, argument: text)
        #elseif os(macOS)
        NSAccessibility.post(
            element: NSApp.mainWindow ?? NSApplication.shared,
            notification: .announcementRequested,
            userInfo: [
                .announcement: speech.text,
                .priority: macPriority(speech.priority).rawValue,
            ])
        #endif
    }

    #if os(iOS)
    private func iosPriority(
        _ priority: SpeechPriority
    ) -> AttributeScopes.AccessibilityAttributes.AnnouncementPriorityAttribute.Value {
        switch priority {
        case .high: .high
        case .default: .default
        case .low: .low
        }
    }
    #elseif os(macOS)
    private func macPriority(_ priority: SpeechPriority) -> NSAccessibilityPriorityLevel {
        switch priority {
        case .high: .high
        case .default: .medium
        case .low: .low
        }
    }
    #endif

    // MARK: - AVSpeechSynthesizer path

    private func synthesize(_ speech: Speech) {
        switch speech.priority {
        case .high:
            synth.stopSpeaking(at: .immediate)
        case .low where synth.isSpeaking:
            return
        case .default, .low:
            break
        }
        let utterance = AVSpeechUtterance(string: speech.text)
        utterance.voice = voice ?? AVSpeechSynthesisVoice(language: voiceLanguage)
        synth.speak(utterance)
    }

    // MARK: - Helpers

    private func configureAudioSession() {
        #if os(iOS)
        // Spoken audio that ducks (not silences) other playback; mixes so we never
        // take over the user's other audio. macOS has no AVAudioSession.
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playback, mode: .spokenAudio, options: [.duckOthers, .mixWithOthers])
        try? session.setActive(true)
        #endif
    }

    /// AVSpeech and screen readers want BCP-47 ("en-US"); the core uses bare codes.
    private static func bcp47(_ code: String) -> String {
        switch code {
        case "en": "en-US"
        case "ru": "ru-RU"
        default: code
        }
    }
}
