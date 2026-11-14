//! `t(key)` as in `src/lib/i18n`: the active locale, then English, then the key itself.
//! Catalogues are `assets/locales/*.json`, generated from the TS ones by `scripts/gen-assets.ts`.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

pub const LOCALES: [(&str, &str); 12] = [
    ("en", "English"),
    ("fr", "Français"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("pt", "Português"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("tr", "Türkçe"),
    ("ar", "العربية"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("zh", "中文"),
];

pub const RTL_LOCALES: [&str; 1] = ["ar"];

type Catalogue = HashMap<String, String>;

macro_rules! catalogue {
    ($code:literal) => {
        ($code, include_str!(concat!("../../assets/locales/", $code, ".json")))
    };
}

static CATALOGUES: LazyLock<HashMap<&'static str, Catalogue>> = LazyLock::new(|| {
    [
        catalogue!("en"), catalogue!("fr"), catalogue!("es"), catalogue!("de"),
        catalogue!("pt"), catalogue!("pl"), catalogue!("ru"), catalogue!("tr"),
        catalogue!("ar"), catalogue!("ja"), catalogue!("ko"), catalogue!("zh"),
    ]
    .into_iter()
    .map(|(code, json)| (code, serde_json::from_str(json).unwrap_or_default()))
    .collect()
});

static CURRENT: RwLock<&'static str> = RwLock::new("en");

/// Base-language match as in i18n/index.ts (`fr-FR` is `fr`); unknown codes are English.
pub fn normalize(code: &str) -> &'static str {
    let base = code.get(..2).unwrap_or("").to_ascii_lowercase();
    LOCALES.iter().find(|(locale_code, _)| *locale_code == code || *locale_code == base).map_or("en", |(locale_code, _)| locale_code)
}

pub fn set_locale(code: &str) {
    *CURRENT.write().unwrap() = normalize(code);
}

pub fn locale() -> &'static str {
    *CURRENT.read().unwrap()
}

pub fn is_rtl() -> bool {
    RTL_LOCALES.contains(&locale())
}

pub fn t(key: &str) -> String {
    let catalogues = &*CATALOGUES;
    catalogues.get(locale())
        .and_then(|catalogue| catalogue.get(key))
        .or_else(|| catalogues.get("en").and_then(|catalogue| catalogue.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `t` with `{name}` placeholders: `tf("dock.moreOnHome", &[("count", &n.to_string())])`.
pub fn tf(key: &str, params: &[(&str, &str)]) -> String {
    let mut translated = t(key);
    for (name, value) in params {
        translated = translated.replace(&format!("{{{name}}}"), value);
    }
    translated
}
