import AVFoundation
import Tactus
import UIKit

/// Speaks the localized strings the core emits (`Speech { text, priority }`).
///
/// Nonvisual-first: when the user already runs **VoiceOver**, we route through
/// `UIAccessibility` announcements so our speech queues with VoiceOver and honors
/// the user's chosen voice and rate — we don't fight the screen reader with a
/// second TTS. When VoiceOver is off, we speak with `AVSpeechSynthesizer`.
///
/// `SpeechPriority` maps to interrupt-vs-enqueue (synth) and to announcement
/// priority (VoiceOver).
///
/// The core's strings are currently single-language per utterance; per-segment
/// language tagging (ADR-0011, e.g. a Russian sentence with an English kit name)
/// is a later enhancement — for now we speak with one voice for the UI locale.
@MainActor
final class SpeechService {
    private let synth = AVSpeechSynthesizer()
    private var voiceLanguage: String

    init(locale: String) {
        voiceLanguage = Self.bcp47(locale)
        configureAudioSession()
    }

    func setLocale(_ locale: String) {
        voiceLanguage = Self.bcp47(locale)
    }

    func speak(_ speech: Speech) {
        if UIAccessibility.isVoiceOverRunning {
            announce(speech)
        } else {
            synthesize(speech)
        }
    }

    // MARK: - VoiceOver path

    private func announce(_ speech: Speech) {
        var text = AttributedString(speech.text)
        text.accessibilitySpeechAnnouncementPriority = priority(for: speech.priority)
        UIAccessibility.post(notification: .announcement, argument: text)
    }

    private func priority(
        for priority: SpeechPriority
    ) -> AttributeScopes.AccessibilityAttributes.AnnouncementPriorityAttribute.Value {
        switch priority {
        case .high: .high
        case .default: .default
        case .low: .low
        }
    }

    // MARK: - AVSpeechSynthesizer path

    private func synthesize(_ speech: Speech) {
        switch speech.priority {
        case .high:
            // Interrupt whatever is speaking so urgent feedback is heard now.
            synth.stopSpeaking(at: .immediate)
        case .low where synth.isSpeaking:
            // Don't let low-priority chatter pile up behind active speech.
            return
        case .default, .low:
            break
        }
        let utterance = AVSpeechUtterance(string: speech.text)
        utterance.voice = AVSpeechSynthesisVoice(language: voiceLanguage)
        synth.speak(utterance)
    }

    // MARK: - Helpers

    private func configureAudioSession() {
        let session = AVAudioSession.sharedInstance()
        // Spoken audio that ducks (not silences) any other playback; mixes so we
        // never take over the user's other audio.
        try? session.setCategory(.playback, mode: .spokenAudio, options: [.duckOthers, .mixWithOthers])
        try? session.setActive(true)
    }

    /// AVSpeech and VoiceOver want BCP-47 ("en-US"); the core uses bare codes.
    private static func bcp47(_ code: String) -> String {
        switch code {
        case "en": "en-US"
        case "ru": "ru-RU"
        default: code
        }
    }
}
