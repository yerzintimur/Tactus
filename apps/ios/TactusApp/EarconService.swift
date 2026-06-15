import Tactus

#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

/// Nonvisual feedback for the core's `Earcon` events.
///
/// On iOS, haptics — immediate, silent, eyes-closed feedback that doesn't compete
/// with the kit's sound (great for a drummer holding the phone). macOS has no app
/// haptics, so there we use distinct short system sounds instead.
@MainActor
final class EarconService {
    #if os(iOS)
    private let notify = UINotificationFeedbackGenerator()
    private let impact = UIImpactFeedbackGenerator(style: .light)
    #endif

    func play(_ earcon: Earcon) {
        #if os(iOS)
        switch earcon {
        case .connected: notify.notificationOccurred(.success)
        case .disconnected: notify.notificationOccurred(.warning)
        case .kitChanged: impact.impactOccurred()
        case .confirmed: notify.notificationOccurred(.success)
        case .error: notify.notificationOccurred(.error)
        }
        #elseif os(macOS)
        NSSound(named: Self.soundName(for: earcon))?.play()
        #endif
    }

    #if os(macOS)
    private static func soundName(for earcon: Earcon) -> NSSound.Name {
        switch earcon {
        case .connected: "Glass"
        case .disconnected: "Bottle"
        case .kitChanged: "Tink"
        case .confirmed: "Pop"
        case .error: "Basso"
        }
    }
    #endif
}
