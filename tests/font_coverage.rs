use ttf_parser::{Face, GlyphId};

struct FontSample {
    script: &'static str,
    text: &'static str,
}

struct BundledFont {
    name: &'static str,
    bytes: &'static [u8],
}

#[test]
fn bundled_fonts_cover_supported_script_samples() {
    let fonts = bundled_fonts();
    let mut failures = Vec::new();

    for sample in script_samples() {
        if !fonts
            .iter()
            .any(|font| font_covers_text(font.bytes, sample.text))
        {
            failures.push(format!("{}: {}", sample.script, sample.text));
        }
    }

    assert!(
        failures.is_empty(),
        "missing bundled font glyph coverage for:\n{}",
        failures.join("\n")
    );
}

#[test]
fn bundled_font_files_are_parseable() {
    let failures = bundled_fonts()
        .into_iter()
        .filter(|font| parseable_faces(font.bytes).is_empty())
        .map(|font| font.name)
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "unparseable bundled font files: {}",
        failures.join(", ")
    );
}

fn bundled_fonts() -> Vec<BundledFont> {
    vec![
        BundledFont {
            name: "NotoSans-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSans-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansArabic-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansHebrew-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansHebrew-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansSyriac-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansSyriac-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansThaana-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansThaana-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansDevanagari-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansDevanagari-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansBengali-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansBengali-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansGujarati-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansGujarati-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansGurmukhi-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansGurmukhi-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansTamil-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansTamil-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansTelugu-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansTelugu-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansKannada-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansKannada-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansMalayalam-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansMalayalam-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansSinhala-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansSinhala-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansThai-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansThai-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansLao-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansLao-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansKhmer-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansKhmer-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansMyanmar-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansMyanmar-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansEthiopic-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansEthiopic-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansArmenian-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansArmenian-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansGeorgian-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansGeorgian-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSerifTibetan-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSerifTibetan-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansMongolian-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansMongolian-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansCherokee-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansCherokee-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansCanadianAboriginal-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansCanadianAboriginal-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansTifinagh-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansTifinagh-Regular.ttf"),
        },
        BundledFont {
            name: "NotoSansCJKsc-Regular",
            bytes: include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf"),
        },
    ]
}

fn script_samples() -> Vec<FontSample> {
    vec![
        FontSample {
            script: "Latin",
            text: "Hello Español Français Deutsch",
        },
        FontSample {
            script: "Greek",
            text: "Ελληνικά",
        },
        FontSample {
            script: "Cyrillic",
            text: "Русский Українська",
        },
        FontSample {
            script: "Arabic",
            text: "العربية",
        },
        FontSample {
            script: "Hebrew",
            text: "עברית",
        },
        FontSample {
            script: "Syriac",
            text: "ܣܘܪܝܝܐ",
        },
        FontSample {
            script: "Thaana",
            text: "ދިވެހި",
        },
        FontSample {
            script: "Devanagari",
            text: "हिन्दी मराठी नेपाली",
        },
        FontSample {
            script: "Bengali",
            text: "বাংলা",
        },
        FontSample {
            script: "Gujarati",
            text: "ગુજરાતી",
        },
        FontSample {
            script: "Gurmukhi",
            text: "ਪੰਜਾਬੀ",
        },
        FontSample {
            script: "Tamil",
            text: "தமிழ்",
        },
        FontSample {
            script: "Telugu",
            text: "తెలుగు",
        },
        FontSample {
            script: "Kannada",
            text: "ಕನ್ನಡ",
        },
        FontSample {
            script: "Malayalam",
            text: "മലയാളം",
        },
        FontSample {
            script: "Sinhala",
            text: "සිංහල",
        },
        FontSample {
            script: "Thai",
            text: "ภาษาไทย",
        },
        FontSample {
            script: "Lao",
            text: "ພາສາລາວ",
        },
        FontSample {
            script: "Khmer",
            text: "ភាសាខ្មែរ",
        },
        FontSample {
            script: "Myanmar",
            text: "မြန်မာ",
        },
        FontSample {
            script: "Ethiopic",
            text: "አማርኛ",
        },
        FontSample {
            script: "Armenian",
            text: "Հայերեն",
        },
        FontSample {
            script: "Georgian",
            text: "ქართული",
        },
        FontSample {
            script: "Tibetan",
            text: "བོད་སྐད",
        },
        FontSample {
            script: "Mongolian",
            text: "ᠮᠣᠩᠭᠣᠯ",
        },
        FontSample {
            script: "Cherokee",
            text: "ᏣᎳᎩ",
        },
        FontSample {
            script: "Canadian Aboriginal",
            text: "ᐃᓄᒃᑎᑐᑦ",
        },
        FontSample {
            script: "Tifinagh",
            text: "ⵜⴰⵎⴰⵣⵉⵖⵜ",
        },
        FontSample {
            script: "Chinese",
            text: "中文",
        },
        FontSample {
            script: "Japanese",
            text: "日本語",
        },
        FontSample {
            script: "Korean",
            text: "한국어",
        },
    ]
}

fn font_covers_text(bytes: &[u8], text: &str) -> bool {
    parseable_faces(bytes).into_iter().any(|face| {
        text.chars()
            .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
            .all(|ch| face.glyph_index(ch).is_some_and(nonzero_glyph))
    })
}

fn parseable_faces(bytes: &[u8]) -> Vec<Face<'_>> {
    let count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    (0..count)
        .filter_map(|index| Face::parse(bytes, index).ok())
        .collect()
}

fn nonzero_glyph(glyph: GlyphId) -> bool {
    glyph.0 != 0
}
