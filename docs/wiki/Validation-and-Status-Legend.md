# Validation and Status Legend

TranslateR shows validation markers so translators and maintainers can find
entries that need attention. Warnings are guidance; they are meant to catch
common mistakes without replacing human review.

## Status Markers

- `U`: untranslated or empty translation.
- `F`: fuzzy entry.
- `P`: plural entry.
- `C`: entry has context (`msgctxt`).
- `%`: entry uses `c-format` placeholders.
- `!`: validation warning.
- `~`: obsolete entry when obsolete entries are visible.

## Empty Translation

An entry is untranslated when its `msgstr` is empty. For plural entries,
TranslateR treats the entry as incomplete when one or more plural forms are
empty.

## Fuzzy

Fuzzy entries are marked with the gettext `fuzzy` flag. They usually mean the
source text changed and the existing translation needs review.

## Missing Plural Forms

TranslateR reads `nplurals=N` from the `.po` header. A plural entry should have
`msgstr[0]` through `msgstr[N-1]`.

If a plural form is missing, TranslateR shows a warning and exposes the missing
form in the editor so it can be filled in.

## C-Format Placeholders

For entries marked `#, c-format`, TranslateR compares printf-style placeholders
between source and translation.

Examples of placeholders:

```text
%s
%d
%1$s
```

Keep placeholders intact unless the project maintainer specifically instructs
otherwise. `%%` is a literal percent sign and is handled separately.

## Trailing Newlines

If the source text ends with `\n`, the translation usually needs to end with
`\n` too. If the source does not end with `\n`, adding one can change how text is
displayed in the application.

## Header Entry

The first gettext header entry is shown separately from normal translation work
where the UI supports it. Translators normally should not translate the header
as if it were a user-facing string.
