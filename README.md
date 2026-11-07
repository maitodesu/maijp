# maijp

A desktop app for drilling basic Japanese by typing fast. Built with [Rune](https://github.com/anomalyco/rune).

- **Kana drill**: hiragana, katakana or both; basic 五十音, dakuten, and combos (きゃ, しゅ…). Type the romaji; each card advances as soon as it's right.
- **JLPT vocab drill**: real vocabulary for the N5–N1 kanji. Pick levels, or hand-pick kanji. Type the word's reading in romaji and press Enter.
- No hints: a wrong answer only flashes red. Space skips (counts as a miss). Answers show on the results screen, with a "Review misses" round.
- Time limits of 0:30 / 1:00 / 2:00 / 5:00, or ∞ practice with no clock.
- Fonts dialog: Geist, Inter or Geist Mono for the interface; Noto Sans or Noto Serif CJK for Japanese.

## Download

Grab the build for your OS from [Releases](../../releases). Japanese text uses your system's CJK font (Noto CJK on Linux, Hiragino on macOS, Yu Gothic / Meiryo on Windows).

## Build

```sh
cargo run --release
```

Linux needs `libxkbcommon` and `wayland` development packages (Fedora: `dnf install libxkbcommon-devel wayland-devel`; Debian/Ubuntu: `apt install libxkbcommon-dev libwayland-dev`).

Romaji rules: `shi`/`si`, `chi`/`ti`, `tsu`/`tu`, `fu`/`hu`, `ji`/`zi` all work; double a consonant for っ (`gakkou`); `n'` or `nn` for ん before a vowel (`kin'en`); `-` for ー.

## Credits

- JLPT kanji levels: [yosida.com](https://yosida.com/en/kanji.php), used with permission.
- Vocabulary: [JMdict](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project) © the Electronic Dictionary Research and Development Group, [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). `assets/words.json` is a trimmed derivative and is shared under the same licence.
- Fonts: [Geist](https://vercel.com/font) and [Inter](https://rsms.me/inter/), both SIL Open Font License 1.1 (see `assets/Inter-LICENSE.txt`).
