# Attribution

## DesignFlow login reference

The owner supplied `78023.png` as the visual source for the login screen. The source-derived
`assets/designflow/login-background.png`, `login-email.png`, `login-lock.png` and `login-eye.png`
retain its decorative artwork and icon silhouettes. These assets were extracted locally,
without image generation. The login uses the original high-resolution `logo-transparent.png`.

Every asset bundled in this repository (images, icons, fonts, data files) is listed here with its
author, source and licence.

DesignFlow's user-interface icons are drawn in code (`crates/ui-designflow/src/icons.rs`): original work,
MIT OR Apache-2.0. No third-party icon sets are used.

## Assets in this repository

| Path | Author | Source | Licence | Notes |
|---|---|---|---|---|
| `assets/designflow/logo-transparent.png` | DesignFlow owner, background removal with ChatGPT | Transparent derivative of the supplied original `assets/designflow/logo.png` | MIT OR Apache-2.0 (project contribution terms) | Original logo preserved separately; transparent display asset for the account screen |
| `assets/designflow/logo.png` | DesignFlow owner, created with ChatGPT | Original logo supplied by the owner for this personal fork | MIT OR Apache-2.0 (project contribution terms) | Original PNG, unmodified; whitespace cropped only at display time |

## Embedded interface font

`assets/fonts/Inter-Regular.ttf` and `Inter-SemiBold.ttf` are the unmodified static TrueType
fonts from Inter 4.1, by Rasmus Andersson and the Inter Project contributors.
Source: https://github.com/rsms/inter/releases/tag/v4.1 . The SIL Open Font License 1.1 is
included in `assets/fonts/LICENSE.txt`. These faces are embedded in the app so its interface
does not depend on fonts installed on the computer.

## Optional fonts (build input, not in this repository)

Release builds embed fonts from the optional craft-fonts build input
([storytold/craft-fonts](https://github.com/storytold/craft-fonts), `CRAFT_FONTS_DIR`). Its own
`ATTRIBUTION.md` and licence files are authoritative; the packages ship each font's licence.

| Family | Licence | Upstream |
|---|---|---|
| Inter | SIL Open Font License 1.1 | https://github.com/rsms/inter |
| Archivo | SIL Open Font License 1.1 | [Google Fonts / Archivo](https://github.com/google/fonts/tree/6c70c829f09ea345d3590406693220ea35c6553f/ofl/archivo) |
| Roboto, Montserrat, Lato, Open Sans, Poppins, Nunito Sans, Merriweather, Playfair Display, Carlito, Caladea | SIL Open Font License 1.1 | Google Fonts / upstream repositories (see craft-fonts) |
| Liberation Sans, Serif, Mono | SIL Open Font License 1.1 | https://github.com/liberationfonts |
| BIZ UDPGothic, BIZ UDMincho, Shippori Mincho, Noto Sans Arabic, Noto Sans Georgian | SIL Open Font License 1.1 | Google Fonts (see craft-fonts) |

The central catalog also contains Adobe-authored Source Sans 3 and Source Serif 4.
DesignFlow filters both families from every build in accordance with its asset policy.

egui's default fonts (compiled into the `egui`/`epaint` crates) keep their own licences (OFL / Apache-2.0 / Ubuntu Font Licence); see those crates.

## egui dependency font icons

| Asset | Author | Source | Licence | Use |
|---|---|---|---|---|
| `epaint_default_fonts::EGUI_ICONS`, `epaint_default_fonts::EMOJI_ICON` (dependency font inputs) | John Slegers | [egui 6d518fc7](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts), [emoji-icon-font](https://github.com/jslegers/emoji-icon-font) | MIT (Copyright (c) 2014 John Slegers; full upstream text in `NOTICE`) | egui-icons.ttf is the five-glyph subset, included by default_fonts independently of monochrome_emoji_fonts; emoji-icon-font.ttf is the full monochrome fallback. Dependency assets unmodified here; no font file copied into this repository. |

## Arabic interface catalog

| Path | Contributor | Source | Licence | Review |
|---|---|---|---|---|
| `crates/ui-egui/locales/ar.toml` | @hussain-hz with OpenAI Codex (AI-assisted) | Original Arabic wording from DesignFlow's English catalog and engine command labels; no proprietary localisation consulted | MIT OR Apache-2.0 | Draft; native-speaker review and maintainer approval pending |

## DesignFlow supplied mark

| Path | Author | Source | Use |
|---|---|---|---|
| assets/designflow/mark-original.png | Project owner (supplied artwork) | User attachment, 2026-11-01 | Original F mark, supplied for the DesignFlow interface |
| assets/designflow/mark.png | Project owner; matte removal by Codex | mark-original.png | Transparent derivative, no generated or redrawn artwork |
