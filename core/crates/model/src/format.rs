//! Turn a parameter's raw value into a localizable [`Message`]. Scaling/units come
//! from the device profile; the actual phrasing lives in the Fluent catalogs.

use crate::i18n::Message;
use device::ParameterDef;

/// Build a localizable message for a numeric parameter's raw value.
///
/// Applies the profile's `scale` for display (e.g. tempo raw 1200, scale 10 ->
/// "120.0"), and uses the parameter's `i18n_key` as the message id.
pub fn format_parameter(param: &ParameterDef, raw: i64) -> Message {
    let id = param
        .i18n_key
        .clone()
        .unwrap_or_else(|| format!("param.{}", param.id));

    match param.scale {
        Some(scale) if scale > 1 => {
            let digits = (scale as f64).log10().round() as usize;
            let value = raw as f64 / scale as f64;
            Message::new(id).arg("value", format!("{value:.digits$}"))
        }
        _ => Message::new(id).arg("value", raw),
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
        .arg("name", name)
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

    #[test]
    fn parameter_label_is_localized_and_value_free() {
        let p = profile();
        let tempo = p.parameter("kit.common.tempo").unwrap();
        let loc = Localizer::new();
        assert_eq!(loc.format(&format_parameter_label(tempo), "en"), "Tempo");
        assert_eq!(loc.format(&format_parameter_label(tempo), "ru"), "Темп");
    }
}
