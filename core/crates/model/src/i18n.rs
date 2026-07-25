//! Localization via Fluent — the single, tested source of announcement/UI
//! phrasing across both platforms (ADR-0008). Catalogs (`i18n/*.ftl`) are
//! embedded; the model emits a [`Message`] (id + args) and the [`Localizer`]
//! renders it for a locale. The voice belongs to the user's screen reader
//! (ADR-0014).

use fluent::concurrent::FluentBundle; // Send + Sync — required so the core/FFI is thread-safe
use fluent::{FluentArgs, FluentResource, FluentValue};
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

const EN_FTL: &str = include_str!("../i18n/en.ftl");
const RU_FTL: &str = include_str!("../i18n/ru.ftl");
const DEFAULT_LOCALE: &str = "en";

/// The language of text that comes from the module — kit and instrument names,
/// the model name. Roland writes them in English ASCII ("Jazz Funk"), and a
/// Russian voice reading them as Russian is unintelligible (ADR-0011). We cannot
/// detect the language of a name the user typed themselves, so this is a
/// deliberate default rather than a guess; a per-profile override can come later.
pub const DEVICE_CONTENT_LANG: &str = "en";

/// Private-use markers wrapped around device-sourced arguments before Fluent
/// formats them, so the span can be located afterwards wherever the translation
/// chose to put it — including a locale that reorders or repeats the argument.
/// Stripped from the rendered text; they never reach the user.
const SPAN_START: char = '\u{E000}';
const SPAN_END: char = '\u{E001}';

/// A localizable argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Str(String),
    Int(i64),
    Float(f64),
    /// Text that came from the module (a kit or instrument name). Rendered like
    /// [`Arg::Str`], but reported as its own language span so the screen reader
    /// pronounces it correctly — see [`DEVICE_CONTENT_LANG`].
    Device(String),
}

impl Arg {
    fn to_fluent(&self) -> FluentValue<'static> {
        match self {
            Arg::Str(s) | Arg::Device(s) => FluentValue::from(s.clone()),
            Arg::Int(i) => FluentValue::from(*i),
            Arg::Float(f) => FluentValue::from(*f),
        }
    }
}

/// One run of text in a single language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    pub text: String,
    /// BCP-47 language tag for this run.
    pub lang: String,
}

/// A localized string plus the language of each of its runs. `spans` always
/// concatenate back to `text` exactly — the platform uses `spans` to tag an
/// accessibility label or announcement, and `text` to display.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalizedText {
    pub text: String,
    pub spans: Vec<TextSpan>,
}

impl LocalizedText {
    /// A whole string in one language.
    pub fn plain(text: impl Into<String>, lang: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            return Self::default();
        }
        Self {
            spans: vec![TextSpan {
                text: text.clone(),
                lang: lang.into(),
            }],
            text,
        }
    }

    /// Whether every run is in the same language (the platform can then skip
    /// building an attributed string).
    pub fn is_single_language(&self) -> bool {
        self.spans.len() <= 1
    }

    /// Append literal text in the language of the preceding run — used for the
    /// separators the engine inserts between two localized sentences.
    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.text.push_str(s);
        match self.spans.last_mut() {
            Some(last) => last.text.push_str(s),
            None => self.spans.push(TextSpan {
                text: s.to_string(),
                lang: DEFAULT_LOCALE.to_string(),
            }),
        }
    }

    /// Append another localized string, merging the runs where the language is
    /// unchanged so the span list stays minimal.
    pub fn push(&mut self, other: &LocalizedText) {
        self.text.push_str(&other.text);
        for span in &other.spans {
            match self.spans.last_mut() {
                Some(last) if last.lang == span.lang => last.text.push_str(&span.text),
                _ => self.spans.push(span.clone()),
            }
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

    /// Builder: attach an argument whose text came from the module (a kit or
    /// instrument name), so it is reported as its own language span (ADR-0011).
    #[must_use]
    pub fn device_arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), Arg::Device(value.into())));
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
        self.format_spans(message, locale).text
    }

    /// Render `message` and report the language of each run: the sentence is in
    /// `locale`, while arguments added with [`Message::device_arg`] are marked as
    /// [`DEVICE_CONTENT_LANG`] (ADR-0011).
    pub fn format_spans(&self, message: &Message, locale: &str) -> LocalizedText {
        let lang = self.resolve_lang(locale);
        let bundle = self
            .bundles
            .get(lang)
            .or_else(|| self.bundles.get(DEFAULT_LOCALE))
            .expect("default locale bundle is always present");

        // Normalise message ids to Fluent style: '.' and '_' both become '-'
        // (e.g. "param.tempo_switch" -> "param-tempo-switch").
        let id = message.id.replace(['.', '_'], "-");
        let Some(msg) = bundle.get_message(&id) else {
            return LocalizedText::plain(id, lang);
        };
        let Some(pattern) = msg.value() else {
            return LocalizedText::plain(id, lang);
        };

        let mut args = FluentArgs::new();
        let mut has_device_arg = false;
        for (key, value) in &message.args {
            match value {
                Arg::Device(text) => {
                    has_device_arg = true;
                    args.set(
                        key.as_str(),
                        FluentValue::from(format!("{SPAN_START}{text}{SPAN_END}")),
                    );
                }
                other => args.set(key.as_str(), other.to_fluent()),
            }
        }
        let mut errors = Vec::new();
        let rendered = bundle
            .format_pattern(pattern, Some(&args), &mut errors)
            .into_owned();

        if has_device_arg {
            split_spans(&rendered, lang)
        } else {
            LocalizedText::plain(rendered, lang)
        }
    }

    /// The bundle language actually used for `locale` ("ru-RU" -> "ru", unknown
    /// -> the default), so spans report the language the text is really in.
    pub fn language(&self, locale: &str) -> &'static str {
        self.resolve_lang(locale)
    }

    fn resolve_lang(&self, locale: &str) -> &'static str {
        let requested = locale.split(['-', '_']).next().unwrap_or(DEFAULT_LOCALE);
        match requested {
            "en" => "en",
            "ru" if self.bundles.contains_key("ru") => "ru",
            _ => DEFAULT_LOCALE,
        }
    }
}

/// Split rendered text on the private-use markers into language runs, dropping
/// the markers. Adjacent runs of the same language are merged.
fn split_spans(rendered: &str, ui_lang: &str) -> LocalizedText {
    let mut out = LocalizedText::default();
    let mut buf = String::new();
    let mut in_device = false;

    let flush = |buf: &mut String, in_device: bool, out: &mut LocalizedText| {
        if buf.is_empty() {
            return;
        }
        let lang = if in_device {
            DEVICE_CONTENT_LANG
        } else {
            ui_lang
        };
        out.push(&LocalizedText::plain(std::mem::take(buf), lang));
    };

    for ch in rendered.chars() {
        match ch {
            SPAN_START if !in_device => {
                flush(&mut buf, in_device, &mut out);
                in_device = true;
            }
            SPAN_END if in_device => {
                flush(&mut buf, in_device, &mut out);
                in_device = false;
            }
            // A stray marker (never expected) is dropped rather than spoken.
            SPAN_START | SPAN_END => {}
            _ => buf.push(ch),
        }
    }
    flush(&mut buf, in_device, &mut out);
    out
}

fn build_bundle(lang: &str, ftl: &str) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = lang.parse().expect("valid language identifier");
    let resource = FluentResource::try_new(ftl.to_string()).expect("embedded FTL parses");
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    bundle
        .add_resource(resource)
        .expect("embedded FTL has no conflicts");
    // Disable Unicode isolation marks — we want clean strings for the screen
    // reader and the UI.
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
    fn localizer_is_send_and_sync() {
        // The FFI wraps the engine (which holds a Localizer) in a Send+Sync object.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Localizer>();
    }

    #[test]
    fn missing_message_returns_id() {
        let loc = Localizer::new();
        assert_eq!(
            loc.format(&Message::new("does.not.exist"), "en"),
            "does-not-exist"
        );
    }

    fn kit(name: &str) -> Message {
        Message::new("kit.label")
            .arg("number", 5u32)
            .device_arg("name", name)
    }

    #[test]
    fn device_text_is_its_own_language_span() {
        let loc = Localizer::new();
        let out = loc.format_spans(&kit("Jazz Funk"), "ru");
        assert_eq!(out.text, "Кит 5: Jazz Funk");
        assert_eq!(
            out.spans,
            vec![
                TextSpan {
                    text: "Кит 5: ".into(),
                    lang: "ru".into()
                },
                TextSpan {
                    text: "Jazz Funk".into(),
                    lang: "en".into()
                },
            ]
        );
    }

    #[test]
    fn spans_always_concatenate_back_to_text() {
        let loc = Localizer::new();
        for locale in ["en", "ru", "de"] {
            for name in ["Jazz Funk", "", "Кит"] {
                let out = loc.format_spans(&kit(name), locale);
                let joined: String = out.spans.iter().map(|s| s.text.as_str()).collect();
                assert_eq!(joined, out.text, "locale {locale}, name {name:?}");
            }
        }
    }

    #[test]
    fn markers_never_reach_the_rendered_text() {
        let loc = Localizer::new();
        let out = loc.format_spans(&kit("Jazz"), "ru");
        assert!(!out.text.contains('\u{E000}') && !out.text.contains('\u{E001}'));
        // Even a name that somehow contains a marker cannot leak one.
        let sneaky = loc.format_spans(&kit("A\u{E001}B"), "ru");
        assert!(!sneaky.text.contains('\u{E001}'));
        assert_eq!(sneaky.text, "Кит 5: AB");
    }

    #[test]
    fn english_ui_needs_no_second_span() {
        // Device content is English too, so the runs merge — the platform can
        // skip building an attributed string entirely.
        let loc = Localizer::new();
        let out = loc.format_spans(&kit("Jazz Funk"), "en");
        assert_eq!(out.text, "Kit 5: Jazz Funk");
        assert!(out.is_single_language());
    }

    #[test]
    fn unknown_locale_spans_report_the_language_actually_used() {
        let loc = Localizer::new();
        let out = loc.format_spans(&kit("Jazz"), "de"); // falls back to en
        assert_eq!(out.spans.first().map(|s| s.lang.as_str()), Some("en"));
    }

    #[test]
    fn plain_messages_are_one_span_in_the_ui_language() {
        let loc = Localizer::new();
        let out = loc.format_spans(&Message::new("edit.out-of-range"), "ru");
        assert_eq!(out.text, "Значение вне диапазона.");
        assert_eq!(out.spans.len(), 1);
        assert_eq!(out.spans[0].lang, "ru");
    }

    #[test]
    fn appending_merges_runs_of_the_same_language() {
        let loc = Localizer::new();
        let mut out = loc.format_spans(&kit("Jazz"), "ru");
        out.push_str(" ");
        out.push(&loc.format_spans(&Message::new("device.firmware_untested"), "ru"));
        // ru … en(name) … ru — the trailing Russian sentence merges with the space.
        let langs: Vec<&str> = out.spans.iter().map(|s| s.lang.as_str()).collect();
        assert_eq!(langs, ["ru", "en", "ru"]);
        let joined: String = out.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, out.text);
    }
}
