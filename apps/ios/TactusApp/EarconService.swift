import Tactus
import UIKit

/// Nonvisual feedback for the core's `Earcon` events. For a drummer holding the
/// phone, haptics are immediate and silent — they work eyes-closed and don't
/// compete with the kit's own sound. Each earcon maps to a distinct haptic.
///
/// (Audible tones can be layered on later; haptics are the MVP channel.)
@MainActor
final class EarconService {
    private let notify = UINotificationFeedbackGenerator()
    private let impact = UIImpactFeedbackGenerator(style: .light)

    func play(_ earcon: Earcon) {
        switch earcon {
        case .connected:
            notify.notificationOccurred(.success)
        case .disconnected:
            notify.notificationOccurred(.warning)
        case .kitChanged:
            impact.impactOccurred()
        case .confirmed:
            notify.notificationOccurred(.success)
        case .error:
            notify.notificationOccurred(.error)
        }
    }
}
