# maijp

[![maijp in 10 seconds](docs/promo.gif)](docs/promo.mp4)

A desktop app for drilling basic Japanese by typing fast. Built with [Rune](https://github.com/anomalyco/rune).

- **Kana drill**: hiragana, katakana or both; basic 五十音, dakuten, and combos (きゃ, しゅ…). Type the romaji; each card advances as soon as it's right.
- **JLPT vocab drill**: real vocabulary for the N5–N1 kanji. Pick levels, or hand-pick kanji. Type the word's reading in romaji and press Enter.
- **Study**: a kana chart (五十音, dakuten, combos) and a JLPT kanji browser. Each kanji opens a card with meanings, on/kun readings, its words and your score on each, plus a "Drill its words" button. Colours show what you know: new, shaky, learning, solid.
- **Progress is saved locally**: settings, fonts, picked kanji, per-card right/seen counts, round history and a day streak (JSON in your config dir: `~/.config/dev.maitodesu.maijp`, `%APPDATA%\dev.maitodesu.maijp` or `~/Library/Application Support/dev.maitodesu.maijp`).
- No hints: a wrong answer only flashes red. Space skips (counts as a miss). Answers show on the results screen, with a "Review misses" round.
- Time limits of 0:30 / 1:00 / 2:00 / 5:00, or ∞ practice with no clock.
- Light and dark themes (or follow the system) over a static grain gradient; the toggle sits next to Fonts.
- Fonts dialog: Geist, Inter or Geist Mono for the interface; Noto Sans JP or Noto Serif JP for Japanese.

## Download

Grab the build for your OS from [Releases](../../releases).

- **Windows:** unzip and run `maijp.exe` (the icon is built in).
- **macOS:** unpack and drag `maijp.app` to Applications. It isn't notarised, so the first time right-click it and choose Open.
- **Linux:** unpack and run `./install.sh`. It puts `maijp` in `~/.local/bin` and adds it to your app menu with its icon. All fonts ship inside the app, so it looks the same on every OS.

## Build

```sh
cargo run --release
```

Linux needs `libxkbcommon` and `wayland` development packages (Fedora: `dnf install libxkbcommon-devel wayland-devel`; Debian/Ubuntu: `apt install libxkbcommon-dev libwayland-dev`).

Romaji rules: `shi`/`si`, `chi`/`ti`, `tsu`/`tu`, `fu`/`hu`, `ji`/`zi` all work; double a consonant for っ (`gakkou`); `n'` or `nn` for ん before a vowel (`kin'en`); `-` for ー.

## Credits

- JLPT kanji levels: [yosida.com](https://yosida.com/en/kanji.php), used with permission.
- Vocabulary: [JMdict](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project) © the Electronic Dictionary Research and Development Group, [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). `assets/words.json` is a trimmed derivative and is shared under the same licence.
- Fonts: [Geist](https://vercel.com/font), [Inter](https://rsms.me/inter/), [Noto Sans JP / Noto Serif JP](https://fonts.google.com/noto) (subset to the characters maijp uses) and [Instrument Serif](https://github.com/Instrument/instrument-serif), all SIL Open Font License 1.1 (see `assets/*-LICENSE.txt`).
