use std::fs;

pub fn install_fonts(ctx: &egui::Context) {
    let loaded = language_fallback_fonts();
    if loaded.is_empty() {
        return;
    }

    let mut fonts = egui::FontDefinitions::default();
    let mut names = Vec::new();

    for (name, bytes) in loaded {
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_owned(bytes).into());
        names.push(name);
    }

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let entries = fonts.families.entry(family).or_default();
        for name in names.iter().rev() {
            entries.insert(0, name.clone());
        }
    }

    ctx.set_fonts(fonts);
}

fn language_fallback_fonts() -> Vec<(String, Vec<u8>)> {
    let mut fonts = bundled_noto_fonts();
    let candidates = [
        ("segoe-ui", r"C:\Windows\Fonts\segoeui.ttf"),
        ("segoe-ui-symbol", r"C:\Windows\Fonts\seguisym.ttf"),
        ("segoe-ui-emoji", r"C:\Windows\Fonts\seguiemj.ttf"),
        ("tahoma", r"C:\Windows\Fonts\tahoma.ttf"),
        ("arial", r"C:\Windows\Fonts\arial.ttf"),
        ("arial-unicode-ms", r"C:\Windows\Fonts\arialuni.ttf"),
        ("nirmala-ui", r"C:\Windows\Fonts\Nirmala.ttf"),
        ("leelawadee-ui", r"C:\Windows\Fonts\LeelUIsl.ttf"),
        ("malgun-gothic", r"C:\Windows\Fonts\malgun.ttf"),
        ("meiryo", r"C:\Windows\Fonts\meiryo.ttc"),
        ("yu-gothic", r"C:\Windows\Fonts\YuGothR.ttc"),
        ("microsoft-yahei", r"C:\Windows\Fonts\msyh.ttc"),
        ("simsun", r"C:\Windows\Fonts\simsun.ttc"),
        ("mingliu", r"C:\Windows\Fonts\mingliu.ttc"),
        ("microsoft-jhenghei", r"C:\Windows\Fonts\msjh.ttc"),
        ("gadugi", r"C:\Windows\Fonts\gadugi.ttf"),
        ("euphemia", r"C:\Windows\Fonts\euphemia.ttf"),
        ("nyala", r"C:\Windows\Fonts\nyala.ttf"),
        ("mv-boli", r"C:\Windows\Fonts\mvboli.ttf"),
        ("estrangelo-edessa", r"C:\Windows\Fonts\estre.ttf"),
        ("mongolian-baiti", r"C:\Windows\Fonts\monbaiti.ttf"),
        (
            "noto-sans",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        ),
        (
            "noto-sans-arabic",
            "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
        ),
        (
            "noto-sans-hebrew",
            "/usr/share/fonts/truetype/noto/NotoSansHebrew-Regular.ttf",
        ),
        (
            "noto-sans-devanagari",
            "/usr/share/fonts/truetype/noto/NotoSansDevanagari-Regular.ttf",
        ),
        (
            "noto-sans-bengali",
            "/usr/share/fonts/truetype/noto/NotoSansBengali-Regular.ttf",
        ),
        (
            "noto-sans-gurmukhi",
            "/usr/share/fonts/truetype/noto/NotoSansGurmukhi-Regular.ttf",
        ),
        (
            "noto-sans-gujarati",
            "/usr/share/fonts/truetype/noto/NotoSansGujarati-Regular.ttf",
        ),
        (
            "noto-sans-tamil",
            "/usr/share/fonts/truetype/noto/NotoSansTamil-Regular.ttf",
        ),
        (
            "noto-sans-telugu",
            "/usr/share/fonts/truetype/noto/NotoSansTelugu-Regular.ttf",
        ),
        (
            "noto-sans-kannada",
            "/usr/share/fonts/truetype/noto/NotoSansKannada-Regular.ttf",
        ),
        (
            "noto-sans-malayalam",
            "/usr/share/fonts/truetype/noto/NotoSansMalayalam-Regular.ttf",
        ),
        (
            "noto-sans-thai",
            "/usr/share/fonts/truetype/noto/NotoSansThai-Regular.ttf",
        ),
        (
            "noto-sans-lao",
            "/usr/share/fonts/truetype/noto/NotoSansLao-Regular.ttf",
        ),
        (
            "noto-sans-khmer",
            "/usr/share/fonts/truetype/noto/NotoSansKhmer-Regular.ttf",
        ),
        (
            "noto-sans-myanmar",
            "/usr/share/fonts/truetype/noto/NotoSansMyanmar-Regular.ttf",
        ),
        (
            "noto-sans-ethiopic",
            "/usr/share/fonts/truetype/noto/NotoSansEthiopic-Regular.ttf",
        ),
        (
            "noto-sans-georgian",
            "/usr/share/fonts/truetype/noto/NotoSansGeorgian-Regular.ttf",
        ),
        (
            "noto-sans-armenian",
            "/usr/share/fonts/truetype/noto/NotoSansArmenian-Regular.ttf",
        ),
        (
            "noto-sans-cjk",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ),
        (
            "dejavu-sans",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ),
        (
            "mac-arial-unicode",
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ),
        ("mac-helvetica", "/System/Library/Fonts/Helvetica.ttc"),
        ("mac-pingfang", "/System/Library/Fonts/PingFang.ttc"),
        (
            "mac-apple-symbols",
            "/System/Library/Fonts/Apple Symbols.ttf",
        ),
    ];

    fonts.extend(candidates.iter().filter_map(|(name, path)| {
        fs::read(path)
            .ok()
            .map(|bytes| ((*name).to_string(), bytes))
    }));
    fonts
}

fn bundled_noto_fonts() -> Vec<(String, Vec<u8>)> {
    [
        (
            "bundled-noto-sans",
            include_bytes!("../../assets/fonts/NotoSans-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-arabic",
            include_bytes!("../../assets/fonts/NotoSansArabic-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-hebrew",
            include_bytes!("../../assets/fonts/NotoSansHebrew-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-armenian",
            include_bytes!("../../assets/fonts/NotoSansArmenian-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-georgian",
            include_bytes!("../../assets/fonts/NotoSansGeorgian-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-devanagari",
            include_bytes!("../../assets/fonts/NotoSansDevanagari-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-bengali",
            include_bytes!("../../assets/fonts/NotoSansBengali-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-gujarati",
            include_bytes!("../../assets/fonts/NotoSansGujarati-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-gurmukhi",
            include_bytes!("../../assets/fonts/NotoSansGurmukhi-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-tamil",
            include_bytes!("../../assets/fonts/NotoSansTamil-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-telugu",
            include_bytes!("../../assets/fonts/NotoSansTelugu-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-kannada",
            include_bytes!("../../assets/fonts/NotoSansKannada-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-malayalam",
            include_bytes!("../../assets/fonts/NotoSansMalayalam-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-sinhala",
            include_bytes!("../../assets/fonts/NotoSansSinhala-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-thai",
            include_bytes!("../../assets/fonts/NotoSansThai-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-lao",
            include_bytes!("../../assets/fonts/NotoSansLao-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-khmer",
            include_bytes!("../../assets/fonts/NotoSansKhmer-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-myanmar",
            include_bytes!("../../assets/fonts/NotoSansMyanmar-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-ethiopic",
            include_bytes!("../../assets/fonts/NotoSansEthiopic-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-serif-tibetan",
            include_bytes!("../../assets/fonts/NotoSerifTibetan-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-mongolian",
            include_bytes!("../../assets/fonts/NotoSansMongolian-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-cherokee",
            include_bytes!("../../assets/fonts/NotoSansCherokee-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-canadian-aboriginal",
            include_bytes!("../../assets/fonts/NotoSansCanadianAboriginal-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-tifinagh",
            include_bytes!("../../assets/fonts/NotoSansTifinagh-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-thaana",
            include_bytes!("../../assets/fonts/NotoSansThaana-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-syriac",
            include_bytes!("../../assets/fonts/NotoSansSyriac-Regular.ttf").as_slice(),
        ),
        (
            "bundled-noto-sans-cjk-sc",
            include_bytes!("../../assets/fonts/NotoSansCJKsc-Regular.otf").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.to_string(), bytes.to_vec()))
    .collect()
}
