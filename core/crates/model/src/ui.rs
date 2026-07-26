//! The app's own interface strings — section headings, control labels, hints
//! and status words.
//!
//! They live here rather than in each platform's own resources for the same
//! reason announcements do (ADR-0008): in a nonvisual-first app, a button label
//! *is* speech — the screen reader reads it aloud — so it belongs in the one
//! tested source of phrasing, shared by iOS and Android alike.
//!
//! Adding a variant here forces the platform mirror to be updated (the
//! conversions are exhaustive) and is checked against every catalog by
//! [`tests::every_ui_string_resolves_in_every_locale`], so a missing translation
//! fails the build instead of being read out as a raw identifier.

/// A piece of the app's own interface text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiString {
    // Language
    SectionLanguage,
    LanguageSystem,
    // Connection
    SectionConnection,
    LabelStatus,
    LabelDevice,
    LabelFirmware,
    StatusDisconnected,
    StatusIdentifying,
    StatusReady,
    ConnectPrompt,
    FirmwareNewer,
    FirmwareOlder,
    FirmwareUnknown,
    // Kit
    SectionKit,
    LabelCurrentKit,
    /// The current kit as one accessibility label, e.g. "Current kit: 5 · Jazz".
    ValueCurrentKit,
    ButtonPreviousKit,
    ButtonNextKit,
    ButtonRenameKit,
    HintRenameKit,
    TitleRenameKit,
    LabelKitName,
    ButtonSave,
    ButtonCancel,
    // Set lists
    SectionSetlist,
    /// A set list the user hasn't opened yet, e.g. "Set list 7".
    ValueSetlistNumber,
    LabelSetlistName,
    /// One step as one accessibility label, e.g. "Step 1: 5 · Jazz".
    ValueSetlistStep,
    ValueSetlistEmpty,
    HintSetlist,
    ButtonAddCurrentKit,
    ButtonMoveStepUp,
    ButtonMoveStepDown,
    ButtonRemoveStep,
    ButtonRenameSetlist,
    TitleRenameSetlist,
    // Tempo
    SectionTempo,
    LabelTempo,
    /// Spoken while an edit is in flight — the value is not settled yet.
    ValueUpdating,
    HintTempoAdjust,
    /// Stands in for a value the device has not reported yet.
    ValueUnknown,
}

impl UiString {
    /// Every variant — the exhaustive list the catalog test walks.
    pub const ALL: &'static [UiString] = &[
        UiString::SectionLanguage,
        UiString::LanguageSystem,
        UiString::SectionConnection,
        UiString::LabelStatus,
        UiString::LabelDevice,
        UiString::LabelFirmware,
        UiString::StatusDisconnected,
        UiString::StatusIdentifying,
        UiString::StatusReady,
        UiString::ConnectPrompt,
        UiString::FirmwareNewer,
        UiString::FirmwareOlder,
        UiString::FirmwareUnknown,
        UiString::SectionKit,
        UiString::LabelCurrentKit,
        UiString::ValueCurrentKit,
        UiString::ButtonPreviousKit,
        UiString::ButtonNextKit,
        UiString::ButtonRenameKit,
        UiString::HintRenameKit,
        UiString::TitleRenameKit,
        UiString::LabelKitName,
        UiString::ButtonSave,
        UiString::ButtonCancel,
        UiString::SectionSetlist,
        UiString::ValueSetlistNumber,
        UiString::LabelSetlistName,
        UiString::ValueSetlistStep,
        UiString::ValueSetlistEmpty,
        UiString::HintSetlist,
        UiString::ButtonAddCurrentKit,
        UiString::ButtonMoveStepUp,
        UiString::ButtonMoveStepDown,
        UiString::ButtonRemoveStep,
        UiString::ButtonRenameSetlist,
        UiString::TitleRenameSetlist,
        UiString::SectionTempo,
        UiString::LabelTempo,
        UiString::ValueUpdating,
        UiString::HintTempoAdjust,
        UiString::ValueUnknown,
    ];

    /// The Fluent message id backing this string.
    pub fn message_id(self) -> &'static str {
        match self {
            UiString::SectionLanguage => "ui-section-language",
            UiString::LanguageSystem => "ui-language-system",
            UiString::SectionConnection => "ui-section-connection",
            UiString::LabelStatus => "ui-label-status",
            UiString::LabelDevice => "ui-label-device",
            UiString::LabelFirmware => "ui-label-firmware",
            UiString::StatusDisconnected => "ui-status-disconnected",
            UiString::StatusIdentifying => "ui-status-identifying",
            UiString::StatusReady => "ui-status-ready",
            UiString::ConnectPrompt => "ui-connect-prompt",
            UiString::FirmwareNewer => "ui-firmware-newer",
            UiString::FirmwareOlder => "ui-firmware-older",
            UiString::FirmwareUnknown => "ui-firmware-unknown",
            UiString::SectionKit => "ui-section-kit",
            UiString::LabelCurrentKit => "ui-label-current-kit",
            UiString::ValueCurrentKit => "ui-value-current-kit",
            UiString::ButtonPreviousKit => "ui-button-previous-kit",
            UiString::ButtonNextKit => "ui-button-next-kit",
            UiString::ButtonRenameKit => "ui-button-rename-kit",
            UiString::HintRenameKit => "ui-hint-rename-kit",
            UiString::TitleRenameKit => "ui-title-rename-kit",
            UiString::LabelKitName => "ui-label-kit-name",
            UiString::ButtonSave => "ui-button-save",
            UiString::ButtonCancel => "ui-button-cancel",
            UiString::SectionSetlist => "ui-section-setlist",
            UiString::ValueSetlistNumber => "ui-value-setlist-number",
            UiString::LabelSetlistName => "ui-label-setlist-name",
            UiString::ValueSetlistStep => "ui-value-setlist-step",
            UiString::ValueSetlistEmpty => "ui-value-setlist-empty",
            UiString::HintSetlist => "ui-hint-setlist",
            UiString::ButtonAddCurrentKit => "ui-button-add-current-kit",
            UiString::ButtonMoveStepUp => "ui-button-move-step-up",
            UiString::ButtonMoveStepDown => "ui-button-move-step-down",
            UiString::ButtonRemoveStep => "ui-button-remove-step",
            UiString::ButtonRenameSetlist => "ui-button-rename-setlist",
            UiString::TitleRenameSetlist => "ui-title-rename-setlist",
            UiString::SectionTempo => "ui-section-tempo",
            UiString::LabelTempo => "ui-label-tempo",
            UiString::ValueUpdating => "ui-value-updating",
            UiString::HintTempoAdjust => "ui-hint-tempo-adjust",
            UiString::ValueUnknown => "ui-value-unknown",
        }
    }

    /// Whether the string takes a `{ $value }` argument.
    pub fn takes_value(self) -> bool {
        matches!(
            self,
            UiString::ValueCurrentKit | UiString::ValueSetlistNumber | UiString::ValueSetlistStep
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{AVAILABLE_LOCALES, Localizer, Message};

    #[test]
    fn every_ui_string_resolves_in_every_locale() {
        // A missing catalog entry renders as the raw id — which a screen reader
        // would happily read out to a blind user. Fail here instead. Driven by
        // AVAILABLE_LOCALES, so offering a new language also demands its strings.
        let loc = Localizer::new();
        for &s in UiString::ALL {
            for locale in AVAILABLE_LOCALES.iter().map(|l| l.code) {
                let mut msg = Message::new(s.message_id());
                if s.takes_value() {
                    msg = msg.arg("value", "X");
                }
                let rendered = loc.format(&msg, locale);
                assert_ne!(
                    rendered,
                    s.message_id(),
                    "{s:?} has no {locale} translation"
                );
                assert!(!rendered.is_empty(), "{s:?} is empty in {locale}");
            }
        }
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut ids: Vec<&str> = UiString::ALL.iter().map(|s| s.message_id()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "duplicate entry in UiString::ALL");
    }

    #[test]
    fn value_strings_interpolate_their_argument() {
        let loc = Localizer::new();
        let msg = Message::new(UiString::ValueCurrentKit.message_id()).arg("value", "5 · Jazz");
        assert_eq!(loc.format(&msg, "en"), "Current kit: 5 · Jazz");
        assert_eq!(loc.format(&msg, "ru"), "Текущий кит: 5 · Jazz");
    }
}
