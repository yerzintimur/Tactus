//! Turn a parameter's raw value into a localizable [`Message`]. Scaling/units come
//! from the device profile; the actual phrasing lives in the Fluent catalogs.

use crate::i18n::Message;
use device::ParameterDef;

/// Build a localizable message for a numeric parameter's raw value.
///
/// Applies the profile's `scale` for display (e.g. tempo raw 1200, scale 10 ->
/// "120.0"), and uses the parameter's `i18n_key` as the message id.
pub fn format_parameter(param: &ParameterDef, raw: i64) -> Message {
    // A sentinel is not a quantity: -1 on a set-list step is the end of the list,
    // -601 on a level is silence. Speaking the number instead would be a lie the
    // user cannot see through — they have only what we say.
    if let Some(sentinel) = &param.sentinel
        && sentinel.raw == raw
    {
        return Message::new(sentinel.i18n_key.clone());
    }

    // An enum value is the module's own word ("SRV-2000", "WARM HALL") — spoken
    // verbatim and tagged as device content so a localized voice does not mangle
    // it (ADR-0011). It is also what the module's own screen and Roland's manual
    // say, which is what a user cross-checking either will expect.
    if let Some(label) = param.enum_label(raw) {
        return Message::new("param.enum_value").device_arg("value", label);
    }

    let id = param
        .i18n_key
        .clone()
        .unwrap_or_else(|| format!("param.{}", param.id));

    // Kits and set-list steps count from 0 on the wire and from 1 on the module's
    // own screen; speak the number the user would read there.
    let shown = raw + param.display_offset;
    match param.scale {
        Some(scale) if scale > 1 => {
            let digits = (scale as f64).log10().round() as usize;
            let value = shown as f64 / scale as f64;
            Message::new(id).arg("value", format!("{value:.digits$}"))
        }
        _ => Message::new(id).arg("value", shown),
    }
}

/// Build a localizable *label* for a parameter (e.g. "Tempo"), distinct from its
/// value phrasing. Uses the parameter's `<i18n_key>.label` message id (falling
/// back to `param.<id>.label`); the UI uses it as the control's accessibility
/// label, so it never carries the value.
pub fn format_parameter_label(param: &ParameterDef) -> Message {
    let base = param
        .i18n_key
        .clone()
        .unwrap_or_else(|| format!("param.{}", param.id));
    Message::new(format!("{base}.label"))
}

/// Build a localizable label for a kit. `display_number` is 1-based (the value
/// shown to the user; the wire value is 0-based).
pub fn format_kit(display_number: u32, name: &str) -> Message {
    Message::new("kit.label")
        .arg("number", display_number)
        // The name comes from the module and is English (ADR-0011).
        .device_arg("name", name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Localizer;
    use device::DeviceProfile;

    const PROFILE: &str = r#"{
        "schema_version": 1,
        "profile_id": "t",
        "display_name": "T",
        "model_id": [1, 6, 1],
        "areas": { "kit": { "address": [4, 0, 0, 0], "stride": [0, 4, 0, 0], "count": 200 } },
        "parameters": [
            { "id": "kit.common.tempo", "area": "kit", "offset": [0, 108], "len": 4,
              "encoding": "nibble", "scale": 10, "unit": "bpm", "i18n_key": "param.tempo" },
            { "id": "kit.common.tempo_switch", "area": "kit", "offset": [0, 112], "len": 1,
              "encoding": "plain7", "i18n_key": "param.tempo_switch" }
        ]
    }"#;

    fn profile() -> DeviceProfile {
        DeviceProfile::from_json(PROFILE).unwrap()
    }

    #[test]
    fn scaled_tempo_renders_with_unit() {
        let p = profile();
        let tempo = p.parameter("kit.common.tempo").unwrap();
        let msg = format_parameter(tempo, 1200);
        let loc = Localizer::new();
        assert_eq!(loc.format(&msg, "en"), "120.0 BPM");
        assert_eq!(loc.format(&msg, "ru"), "120.0 уд/мин");
    }

    #[test]
    fn unscaled_value_is_integer() {
        let p = profile();
        let sw = p.parameter("kit.common.tempo_switch").unwrap();
        let msg = format_parameter(sw, 1);
        assert_eq!(Localizer::new().format(&msg, "en"), "Tempo switch: 1");
    }

    #[test]
    fn kit_label() {
        let loc = Localizer::new();
        assert_eq!(loc.format(&format_kit(5, "Jazz"), "en"), "Kit 5: Jazz");
        assert_eq!(loc.format(&format_kit(5, "Jazz"), "ru"), "Кит 5: Jazz");
    }

    /// The guard that keeps a blind user from being read raw identifiers: every
    /// parameter the built-in profile exposes must have a key, and that key must
    /// resolve — label and value — in every language we offer.
    #[test]
    fn every_profile_parameter_speaks_in_every_locale() {
        let registry = device::ProfileRegistry::with_builtin();
        let profile = registry.match_model(&[1, 6, 1]).expect("built-in V31");
        let loc = Localizer::new();

        for param in &profile.parameters {
            let key = param
                .i18n_key
                .as_deref()
                .unwrap_or_else(|| panic!("{} has no i18n_key", param.id));
            let unresolved = key.replace(['.', '_'], "-");

            for locale in crate::i18n::AVAILABLE_LOCALES.iter().map(|l| l.code) {
                let label = loc.format(&format_parameter_label(param), locale);
                assert_ne!(
                    label,
                    format!("{unresolved}-label"),
                    "{} has no {locale} label",
                    param.id
                );
                assert!(!label.is_empty());

                // Enums render through the shared message, text through the raw
                // device string — only plain numerics need their own phrasing.
                if param.labels.is_none()
                    && let Some(range) = param.range
                {
                    let value = loc.format(&format_parameter(param, range.min), locale);
                    assert_ne!(
                        value, unresolved,
                        "{} has no {locale} value phrasing",
                        param.id
                    );
                    assert!(!value.is_empty());
                }
            }
        }
    }

    /// The module counts kits and set-list steps from 0 on the wire and from 1 on
    /// its own screen. A blind user has only what we say, so we say the number
    /// they would read there — and the number the module's manual uses.
    #[test]
    fn zero_based_values_speak_the_number_on_the_module() {
        let registry = device::ProfileRegistry::with_builtin();
        let profile = registry.match_model(&[1, 6, 1]).expect("built-in V31");
        let kit_num = profile.parameter("current.kit_num").expect("kit num");
        let loc = Localizer::new();

        assert_eq!(loc.format(&format_parameter(kit_num, 4), "en"), "Kit 5");
        assert_eq!(loc.format(&format_parameter(kit_num, 199), "en"), "Kit 200");
        assert_eq!(loc.format(&format_parameter(kit_num, 4), "ru"), "Кит 5");
    }

    /// A sentinel is not a quantity: the last step of a set list holds −1, which
    /// means "the list ends here", not "kit zero".
    #[test]
    fn sentinel_values_speak_their_meaning_not_their_number() {
        let registry = device::ProfileRegistry::with_builtin();
        let profile = registry.match_model(&[1, 6, 1]).expect("built-in V31");
        let step = profile.parameter("setlist.step").expect("set-list step");
        let loc = Localizer::new();

        assert_eq!(
            loc.format(&format_parameter(step, -1), "en"),
            "End of the set list"
        );
        assert_eq!(
            loc.format(&format_parameter(step, -1), "ru"),
            "Конец сет-листа"
        );
        // Every other value is still a kit, counted from 1.
        assert_eq!(loc.format(&format_parameter(step, 46), "en"), "Kit 47");
    }

    #[test]
    fn enum_values_speak_the_modules_own_word() {
        let registry = device::ProfileRegistry::with_builtin();
        let profile = registry.match_model(&[1, 6, 1]).expect("built-in V31");
        let reverb = profile.parameter("kit.reverb.type").expect("reverb type");
        let loc = Localizer::new();

        // Roland's name, not a number and not a translation — in either language.
        let msg = format_parameter(reverb, 2);
        assert_eq!(loc.format(&msg, "en"), "WARM HALL");
        assert_eq!(loc.format(&msg, "ru"), "WARM HALL");
        // …and tagged as English so a Russian voice doesn't mangle it (ADR-0011).
        let spans = loc.format_spans(&msg, "ru").spans;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].lang, "en");
    }

    #[test]
    fn parameter_label_is_localized_and_value_free() {
        let p = profile();
        let tempo = p.parameter("kit.common.tempo").unwrap();
        let loc = Localizer::new();
        assert_eq!(loc.format(&format_parameter_label(tempo), "en"), "Tempo");
        assert_eq!(loc.format(&format_parameter_label(tempo), "ru"), "Темп");
    }
}
