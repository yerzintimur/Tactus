//! Localization via Fluent — the single, tested source of spoken/UI phrasing
//! across both platforms (ADR-0008). Catalogs (`i18n/*.ftl`) are embedded; the
//! model emits a [`Message`] (id + args) and the [`Localizer`] renders it for a
//! locale. The OS provides only the TTS voice.

use fluent::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("../i18n/en.ftl");
const RU_FTL: &str = include_str!("../i18n/ru.ftl");
const DEFAULT_LOCALE: &str = "en";

/// A localizable argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Str(String),
    Int(i64),
    Float(f64),
}

impl Arg {
    fn to_fluent(&self) -> FluentValue<'static> {
        match self {
            Arg::Str(s) => FluentValue::from(s.clone()),
            Arg::Int(i) => FluentValue::from(*i),
            Arg::Float(f) => FluentValue::from(*f),
        }
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Str(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Str(s)
    }
}
impl From<i64> for Arg {
    fn from(n: i64) -> Self {
        Arg::Int(n)
    }
}
impl From<u32> for Arg {
    fn from(n: u32) -> Self {
        Arg::Int(i64::from(n))
    }
}
impl From<f64> for Arg {
    fn from(n: f64) -> Self {
        Arg::Float(n)
    }
}

/// A message to be localized: a stable id plus named arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub id: String,
    pub args: Vec<(String, Arg)>,
}

impl Message {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            args: Vec::new(),
        }
    }

    /// Builder: attach a named argument.
    #[must_use]
    pub fn arg(mut self, key: impl Into<String>, value: impl Into<Arg>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }
}

/// Renders [`Message`]s to localized strings using embedded Fluent catalogs.
pub struct Localizer {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
}

impl Default for Localizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Localizer {
    /// Build a localizer with all embedded locales (en, ru).
    pub fn new() -> Self {
        let mut bundles = HashMap::new();
        bundles.insert("en".to_string(), build_bundle("en", EN_FTL));
        bundles.insert("ru".to_string(), build_bundle("ru", RU_FTL));
        Self { bundles }
    }

    /// Render `message` for `locale` (e.g. "ru" or "ru-RU"), falling back to the
    /// default locale, then to the raw id — never panics, never returns empty.
    pub fn format(&self, message: &Message, locale: &str) -> String {
        let lang = locale.split(['-', '_']).next().unwrap_or(DEFAULT_LOCALE);
        let bundle = self
            .bundles
            .get(lang)
            .or_else(|| self.bundles.get(DEFAULT_LOCALE))
            .expect("default locale bundle is always present");

        // Normalise message ids to Fluent style: '.' and '_' both become '-'
        // (e.g. "param.tempo_switch" -> "param-tempo-switch").
        let id = message.id.replace(['.', '_'], "-");
        let Some(msg) = bundle.get_message(&id) else {
            return id;
        };
        let Some(pattern) = msg.value() else {
            return id;
        };

        let mut args = FluentArgs::new();
        for (key, value) in &message.args {
            args.set(key.as_str(), value.to_fluent());
        }
        let mut errors = Vec::new();
        bundle
            .format_pattern(pattern, Some(&args), &mut errors)
            .into_owned()
    }
}

fn build_bundle(lang: &str, ftl: &str) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = lang.parse().expect("valid language identifier");
    let resource = FluentResource::try_new(ftl.to_string()).expect("embedded FTL parses");
    let mut bundle = FluentBundle::new(vec![langid]);
    bundle
        .add_resource(resource)
        .expect("embedded FTL has no conflicts");
    // Disable Unicode isolation marks — we want clean strings for TTS / UI.
    bundle.set_use_isolating(false);
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_en_and_ru() {
        let loc = Localizer::new();
        let msg = Message::new("kit.label")
            .arg("number", 5u32)
            .arg("name", "Jazz");
        assert_eq!(loc.format(&msg, "en"), "Kit 5: Jazz");
        assert_eq!(loc.format(&msg, "ru"), "Кит 5: Jazz");
    }

    #[test]
    fn unknown_locale_falls_back_to_default() {
        let loc = Localizer::new();
        let msg = Message::new("kit.label")
            .arg("number", 1u32)
            .arg("name", "Rock");
        assert_eq!(loc.format(&msg, "de"), "Kit 1: Rock");
    }

    #[test]
    fn region_subtag_is_stripped() {
        let loc = Localizer::new();
        let msg = Message::new("kit.label")
            .arg("number", 2u32)
            .arg("name", "Funk");
        assert_eq!(loc.format(&msg, "ru-RU"), "Кит 2: Funk");
    }

    #[test]
    fn missing_message_returns_id() {
        let loc = Localizer::new();
        assert_eq!(
            loc.format(&Message::new("does.not.exist"), "en"),
            "does-not-exist"
        );
    }
}
