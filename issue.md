# Issue #256: zh-Hans: pour the Phase 1 drafts into the gettext catalogue and publish /zh-Hans/

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement

## Body

Phase 1 of the Chinese translation is merged as parallel drafts (`README.zh-Hans.md`, `bindings/python/README.zh-Hans.md`, `docs/book/po/zh-Hans/**`), and Phase 0's tooling is live: catalogue `docs/book/po/zh-Hans.po` (1936 messages, all `msgstr` empty), glossary gate (`msgid`-anchored, 49 rows), `make check-i18n`, and the `/zh-Hans/` Pages build (English fallback until poured).

The pour: move the drafts' paragraphs into the catalogue's `msgstr`s per the conventions in `docs/RELEASE.md`-adjacent notes and PR #245's body — paragraphs joined single-space with no trailing newline; headings without `#`; one message per table cell; unlexed fences are ONE message including the fence lines (do not translate inside them); never run `mdbook-i18n-normalize` (at 0.4.0 it rewrites 83 msgids). Chinese-only additions (mirror paragraphs, tracking banner) attach to the neighbouring paragraph's `msgstr` via a blank line, never to the English source.

Rules: `git add` before any gate; `make check-i18n` green; every "where it stops" clause carries its boundary exactly; uncertain renderings stay English with a gloss. PR #244's traceability comment lists the twelve least-confident renderings (warrant, fallible view, context lens, lane, front-coded, sideband, fail-closed, "poisoning the fold", the EL++ saturation clause, glyph, oracle, wall deadline) — spot-check those by blind back-translation before or after the pour (see the vetting protocol used on #244: blind back-translation + native-reader review, findings in English with every quote glossed, because the maintainer does not read Chinese).

After the pour: re-render README deltas whenever English positioning changes (the drift line in `make check-i18n` reports how many messages need `make book-po-update`), and consider serving the zh book from blackcatinformatics.cn per assessment §6.

## Comments (0)

