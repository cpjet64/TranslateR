# License and Fonts

TranslateR is licensed under the MIT License.

## Application License

The application source code is covered by `LICENSE` in the repository and in
release packages.

## Bundled Fonts

TranslateR bundles Noto fallback fonts for broad language and script coverage.
Bundled Noto fonts are licensed separately under the SIL Open Font License 1.1.

Release packages include third-party font license files in `LICENSES/`.

## Runtime Font Loading

TranslateR loads bundled Noto fonts and also attempts to use common platform
fonts on Windows, Linux, and macOS. This improves coverage for Arabic, Hebrew,
Indic scripts, CJK, Thai, Lao, Khmer, Myanmar, Ethiopic, Georgian, Armenian,
Tibetan, Mongolian, Cherokee, Canadian Aboriginal, Tifinagh, Thaana, Syriac, and
other scripts represented by the bundled or platform fonts.

Font coverage is tested with representative samples in `tests/font_coverage.rs`.

## When Adding Fonts

When a font is added or removed, update:

- `src/ui/fonts.rs`.
- `tests/font_coverage.rs`.
- `LICENSES/README.md`.
- Any relevant release package checks.
