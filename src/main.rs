//! maijp: kana and JLPT vocab speed-typing drills on Rune.
//! `cargo run` opens it; `cargo run -- --snap out.png --scene play` renders headless.

use std::f32::consts::PI;
use std::sync::LazyLock;
use std::time::Duration;

use rune::{AppSpec, Color, Cx, El, Family, Key, KeyEvent, ScrollSpec, TextStyle, Tx, col, el, row, text};

mod vocab;
use vocab::{KANJI, WORDS};

// ---------- kana table: a kana token, then its accepted romaji ----------
const BASIC: &str = "あ a い i う u え e お o か ka き ki く ku け ke こ ko \
    さ sa し shi si す su せ se そ so た ta ち chi ti つ tsu tu て te と to \
    な na に ni ぬ nu ね ne の no は ha ひ hi ふ fu hu へ he ほ ho \
    ま ma み mi む mu め me も mo や ya ゆ yu よ yo ら ra り ri る ru れ re ろ ro \
    わ wa を wo o ん n nn";
const DAKUTEN: &str = "が ga ぎ gi ぐ gu げ ge ご go ざ za じ ji zi ず zu ぜ ze ぞ zo \
    だ da ぢ ji di づ zu du で de ど do ば ba び bi ぶ bu べ be ぼ bo ぱ pa ぴ pi ぷ pu ぺ pe ぽ po";
const COMBOS: &str = "きゃ kya きゅ kyu きょ kyo しゃ sha sya しゅ shu syu しょ sho syo \
    ちゃ cha tya cya ちゅ chu tyu cyu ちょ cho tyo cyo にゃ nya にゅ nyu にょ nyo \
    ひゃ hya ひゅ hyu ひょ hyo みゃ mya みゅ myu みょ myo りゃ rya りゅ ryu りょ ryo \
    ぎゃ gya ぎゅ gyu ぎょ gyo じゃ ja jya zya じゅ ju jyu zyu じょ jo jyo zyo \
    びゃ bya びゅ byu びょ byo ぴゃ pya ぴゅ pyu ぴょ pyo";

struct Kana { kana: &'static str, set: usize, roma: Vec<&'static str> }

static TABLE: LazyLock<Vec<Kana>> = LazyLock::new(|| {
    let mut t: Vec<Kana> = Vec::new();
    for (set, src) in [BASIC, DAKUTEN, COMBOS].into_iter().enumerate() {
        for tok in src.split_whitespace() {
            if tok.is_ascii() { t.last_mut().expect("table starts with kana").roma.push(tok) }
            else { t.push(Kana { kana: tok, set, roma: Vec::new() }) }
        }
    }
    t
});

/// Hiragana sit exactly 0x60 below their katakana.
fn katakana(h: &str) -> String {
    h.chars().map(|c| if ('\u{3041}'..='\u{3096}').contains(&c) { char::from_u32(c as u32 + 0x60).unwrap() } else { c }).collect()
}

fn shown(card: Card) -> String {
    match card {
        Card::Kana(i, true) => katakana(TABLE[i].kana),
        Card::Kana(i, false) => TABLE[i].kana.to_string(),
        Card::Word(i) => WORDS[i].w.clone(),
    }
}

// ---------- state ----------
#[derive(PartialEq, Clone, Copy, Debug)]
enum Card { Kana(usize, bool), Word(usize) }
#[derive(PartialEq, Clone, Copy, Debug)]
enum Screen { Menu, Play, Done }
#[derive(PartialEq, Clone, Copy, Debug)]
enum Drill { Kana, Vocab }

const SCRIPTS: [&str; 3] = ["Hiragana", "Katakana", "Both"];
const SETS: [(&str, &str); 3] = [("Basic", "46 五十音"), ("Dakuten", "25 が ぱ"), ("Combos", "33 きゃ しゅ")];
/// Words per vocab round; MAX is every matching word.
const ROUNDS: [usize; 4] = [20, 50, 100, usize::MAX];
/// Infinity is practice: no clock, no pressure.
const LIMITS: [f64; 5] = [30.0, 60.0, 120.0, 300.0, f64::INFINITY];
/// (label, family): UI fonts are bundled; JP fonts are the system CJK faces rune loads.
const UI_FONTS: [(&str, &str); 3] = [("Geist", "Geist"), ("Inter", "Inter"), ("Geist Mono", "Geist Mono")];
const JP_FONTS: [(&str, &str); 2] = [("Noto Sans", "Noto Sans CJK JP"), ("Noto Serif", "Noto Serif CJK JP")];

struct State {
    screen: Screen,
    drill: Drill,
    /// JLPT levels on, indexed N5..N1.
    levels: [bool; 5],
    round: usize,
    /// Hand-picked kanji (by KANJI index); none picked means every kanji in the levels.
    picked: Vec<bool>,
    pick_open: bool,
    script: usize,
    sets: [bool; 3],
    limit: usize,
    deck: Vec<Card>,
    pos: usize,
    input: String,
    wrong: bool,
    /// ん accepts "n"; a second n typed out of habit is swallowed, not a miss.
    swallow_n: bool,
    keys: u32,
    bad_keys: u32,
    skips: usize,
    missed: Vec<Card>,
    started: f64,
    now: f64,
    ended: f64,
    cleared: bool,
    seed: u64,
    ui_font: usize,
    jp_font: usize,
    fonts_open: bool,
}

fn init() -> State {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) | 1;
    State { screen: Screen::Menu, drill: Drill::Kana, levels: [true, false, false, false, false], round: 0,
        picked: vec![false; KANJI.len()], pick_open: false, script: 0, sets: [true, false, false], limit: 1, deck: Vec::new(), pos: 0,
        input: String::new(), wrong: false, swallow_n: false, keys: 0, bad_keys: 0, skips: 0, missed: Vec::new(),
        started: 0.0, now: 0.0, ended: 0.0, cleared: false, seed,
        ui_font: 0, jp_font: 0, fonts_open: false }
}

fn level_on(s: &State, n: u8) -> bool { s.levels[5 - n as usize] }

fn full_deck(s: &State) -> Vec<Card> {
    if s.drill == Drill::Vocab {
        let any_picked = s.picked.contains(&true);
        return WORDS.iter().enumerate()
            .filter(|(_, w)| level_on(s, w.level) && (!any_picked || w.kanji.iter().any(|&k| s.picked[k])))
            .map(|(i, _)| Card::Word(i)).collect();
    }
    let scripts: &[bool] = match s.script { 0 => &[false], 1 => &[true], _ => &[false, true] };
    TABLE.iter().enumerate().filter(|(_, k)| s.sets[k.set])
        .flat_map(|(i, _)| scripts.iter().map(move |&kata| Card::Kana(i, kata))).collect()
}

/// Starts a round over `deck`, shuffled (xorshift: no rand crate for one shuffle).
fn start(s: &mut State, mut deck: Vec<Card>) {
    if deck.is_empty() { return }
    for i in (1..deck.len()).rev() {
        s.seed ^= s.seed << 13; s.seed ^= s.seed >> 7; s.seed ^= s.seed << 17;
        deck.swap(i, (s.seed % (i as u64 + 1)) as usize);
    }
    if s.drill == Drill::Vocab { deck.truncate(ROUNDS[s.round]) }
    s.screen = Screen::Play; s.deck = deck; s.started = -1.0;
    s.pos = 0; s.input.clear(); s.wrong = false; s.swallow_n = false; s.keys = 0; s.bad_keys = 0; s.skips = 0; s.missed.clear();
}

fn finish(s: &mut State, cleared: bool) {
    s.screen = Screen::Done;
    s.cleared = cleared;
    s.ended = (s.now - s.started).clamp(0.0, LIMITS[s.limit]);
}

fn type_char(s: &mut State, c: char) {
    if s.swallow_n && c == 'n' { s.swallow_n = false; return }
    s.swallow_n = false;
    s.keys += 1;
    s.input.push(c);
    let card = s.deck[s.pos];
    let Card::Kana(i, _) = card else { return };
    let roma = &TABLE[i].roma;
    if roma.iter().any(|r| *r == s.input) {
        s.swallow_n = s.input == "n";
        s.input.clear();
        s.wrong = false;
        s.pos += 1;
        if s.pos == s.deck.len() { finish(s, true) }
    } else if !roma.iter().any(|r| r.starts_with(s.input.as_str())) {
        s.bad_keys += 1;
        s.input.clear();
        s.wrong = true;
        if !s.missed.contains(&card) { s.missed.push(card) }
    }
}

/// Enter checks a vocab answer: the whole reading, typed as romaji.
fn submit(s: &mut State) {
    let card = s.deck[s.pos];
    let Card::Word(i) = card else { return };
    if s.input.is_empty() { return }
    s.keys += 1;
    if vocab::is_reading(&WORDS[i], &s.input) {
        s.input.clear();
        s.wrong = false;
        s.pos += 1;
        if s.pos == s.deck.len() { finish(s, true) }
    } else {
        s.bad_keys += 1;
        s.input.clear();
        s.wrong = true;
        if !s.missed.contains(&card) { s.missed.push(card) }
    }
}

fn vocab_round(s: &State) -> bool { matches!(s.deck.get(s.pos), Some(Card::Word(_))) }

/// Space gives up on a card: it counts as a miss and moves on.
fn skip(s: &mut State) {
    let card = s.deck[s.pos];
    if !s.missed.contains(&card) { s.missed.push(card) }
    s.skips += 1;
    s.keys += 1; // a skip is a failed attempt
    s.bad_keys += 1;
    s.input.clear();
    s.wrong = false;
    s.pos += 1;
    if s.pos == s.deck.len() { finish(s, true) }
}

fn on_key(s: &mut State, ev: &KeyEvent, _: &mut Tx) -> bool {
    if ev.mods.ctrl || ev.mods.cmd || ev.mods.alt { return false }
    match (s.screen, &ev.key) {
        (Screen::Play, Key::Text(t)) if t == " " => skip(s),
        (Screen::Play, Key::Text(t)) if vocab_round(s) => {
            s.wrong = false;
            s.input.extend(t.chars().filter(|c| c.is_ascii_alphabetic() || *c == '-' || *c == '\'').map(|c| c.to_ascii_lowercase()));
        }
        (Screen::Play, Key::Enter) if vocab_round(s) => submit(s),
        (Screen::Play, Key::Text(t)) => {
            for c in t.chars().filter(char::is_ascii_alphabetic) {
                if s.screen == Screen::Play { type_char(s, c.to_ascii_lowercase()) }
            }
        }
        (Screen::Play, Key::Backspace) => { s.input.pop(); }
        (Screen::Play | Screen::Done, Key::Escape) => s.screen = Screen::Menu,
        (Screen::Menu, _) if s.fonts_open || s.pick_open => return false,
        (Screen::Menu | Screen::Done, Key::Enter) => { let d = full_deck(s); start(s, d) }
        _ => return false,
    }
    true
}

fn poll(s: &mut State, t: f64, tx: &mut Tx) -> bool {
    s.now = t;
    if s.screen != Screen::Play { return false }
    if s.started < 0.0 { s.started = t }
    if t - s.started >= LIMITS[s.limit] { finish(s, false) } else { tx.wake_after(Duration::from_millis(50)) }
    true
}

// ---------- look ----------
const fn rgb(h: u32) -> Color {
    Color { r: ((h >> 16) & 255) as f32 / 255.0, g: ((h >> 8) & 255) as f32 / 255.0, b: (h & 255) as f32 / 255.0, a: 1.0 }
}
const W: Color = Color::WHITE;
const INK: Color = rgb(0x120E24);
const SAKURA: Color = rgb(0xFF8FB1);
const SHU: Color = rgb(0xFF5A4E);
const MATCHA: Color = rgb(0x7FE0A8);

const HERO: TextStyle = TextStyle::new(34.0, 600.0).tracking(-0.8);
fn jp(s: &State, size: f32) -> TextStyle { TextStyle::new(size, 400.0).family(Family::Named(JP_FONTS[s.jp_font].1)) }
const BODY: TextStyle = TextStyle::new(15.0, 500.0);
const SMALL: TextStyle = TextStyle::new(13.0, 500.0);
const CAPS: TextStyle = TextStyle::new(11.0, 600.0).tracking(1.0);
const NUM: TextStyle = TextStyle::new(30.0, 400.0).tracking(-0.8);
const TYPED: TextStyle = TextStyle::new(28.0, 400.0).mono();

fn glass() -> El {
    col().radius(24.0).backdrop_blur(24.0)
        .fill(W.alpha(0.07))
        .linear_gradient(PI, [(0.0, W.alpha(0.14)), (0.5, W.alpha(0.03)), (1.0, W.alpha(0.06))])
        .border(1.0, W.alpha(0.18))
        .shadow(20.0, 44.0, Color::BLACK.alpha(0.3))
        .padding(22.0)
}

fn caps(t: &str) -> El { text(t.to_uppercase()).font(CAPS).color(W.alpha(0.55)) }

fn chip(key: String, label: &str, on: bool, press: impl Fn(&mut State) + 'static) -> El {
    chip_in(key, label, SMALL, on, move |s: &mut State, _: &mut Tx| press(s))
}

fn chip_in(key: String, label: &str, font: TextStyle, on: bool, press: impl Fn(&mut State, &mut Tx) + 'static) -> El {
    row().key(key).role("button").label(label.to_string()).focusable(true).h(36.0).px(16.0).center().radius(18.0)
        .fill(if on { W.alpha(0.94) } else { W.alpha(0.08) })
        .border(1.0, W.alpha(if on { 0.0 } else { 0.18 }))
        .press_scale(0.96).on_press_tx(press)
        .child(text(label.to_string()).font(font).color(if on { INK } else { W }))
}

fn button(key: &str, label: &str, primary: bool, press: impl Fn(&mut State) + 'static) -> El {
    let b = row().key(key.to_string()).role("button").label(label.to_string()).focusable(true).h(46.0).px(26.0).center().radius(23.0)
        .press_scale(0.97).on_press(press);
    if primary {
        b.linear_gradient(PI / 2.0, [(0.0, rgb(0xE8433F)), (1.0, rgb(0xD9467E))]).shadow(10.0, 28.0, SHU.alpha(0.45))
            .child(text(label.to_string()).font(BODY).color(W))
    } else {
        b.fill(W.alpha(0.08)).border(1.0, W.alpha(0.2)).child(text(label.to_string()).font(BODY).color(W))
    }
}

fn logo(s: &State) -> El {
    row().items_center().gap(10.0)
        .child(col().size(30.0, 30.0).radius(15.0).center()
            .radial_gradient([0.35, 0.3], 22.0, [(0.0, rgb(0xFF9A8A)), (1.0, SHU)])
            .shadow(4.0, 16.0, SHU.alpha(0.5))
            .child(text("ま").font(TextStyle { weight: 700.0, ..jp(s, 16.0) }).color(W)))
        .child(text("maijp").font(TextStyle::new(19.0, 600.0).tracking(-0.4)).color(W))
}

/// 65.2 -> "1 min 5 secs".
fn words(secs: f64) -> String {
    let t = secs.round() as u32;
    let (m, s) = (t / 60, t % 60);
    let sec = format!("{s} sec{}", if s == 1 { "" } else { "s" });
    if m == 0 { sec } else if s == 0 { format!("{m} min") } else { format!("{m} min {sec}") }
}

fn clock(secs: f64) -> String {
    if secs.is_infinite() { return "∞ Practice".into() }
    let s = secs.max(0.0).ceil() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

// ---------- screens ----------
fn menu(s: &State) -> El {
    let group = |title: &str, chips: Vec<El>| col().gap(10.0).child(caps(title)).child(row().flex_wrap().gap(8.0).children(chips));
    let scripts = SCRIPTS.iter().enumerate().map(|(i, n)| chip(format!("script-{i}"), n, s.script == i, move |s: &mut State| s.script = i)).collect();
    let sets = SETS.iter().enumerate().map(|(i, (n, sub))| {
        chip(format!("set-{i}"), &format!("{n} · {sub}"), s.sets[i], move |s: &mut State| {
            s.sets[i] = !s.sets[i];
            if !s.sets.contains(&true) { s.sets[0] = true } // never an empty deck
        })
    }).collect();
    let limits = LIMITS.iter().enumerate().map(|(i, l)| chip(format!("limit-{i}"), &clock(*l), s.limit == i, move |s: &mut State| s.limit = i)).collect();
    let n = full_deck(s).len();
    let tabs = row().gap(8.0)
        .child(chip_in("tab-kana".into(), "Kana  かな", jp(s, 14.0), s.drill == Drill::Kana, |s: &mut State, _: &mut Tx| s.drill = Drill::Kana))
        .child(chip_in("tab-vocab".into(), "JLPT vocab  漢字", jp(s, 14.0), s.drill == Drill::Vocab, |s: &mut State, _: &mut Tx| s.drill = Drill::Vocab));
    let (title, sub, panel, count) = if s.drill == Drill::Kana {
        ("Type every kana before the clock runs out.",
         "Romaji in, kana out. No hints: a wrong key flashes red. Space skips (counts as a miss).",
         glass().w_full().gap(22.0).child(group("Script", scripts)).child(group("Sets", sets)).child(group("Time limit", limits)),
         format!("{n} cards · press Enter"))
    } else {
        let levels = (0..5).map(|i| {
            let lv = 5 - i as u8;
            let kanji = KANJI.iter().filter(|k| k.n == lv).count();
            chip(format!("level-{i}"), &format!("N{lv} · {kanji}"), s.levels[i], move |s: &mut State| {
                s.levels[i] = !s.levels[i];
                if !s.levels.contains(&true) { s.levels[i] = true } // never an empty deck
            })
        }).collect();
        let picked = s.picked.iter().enumerate().filter(|(k, on)| **on && level_on(s, KANJI[*k].n)).count();
        let mut pick = row().items_center().gap(10.0)
            .child(chip("pick-open".into(), "Pick kanji…", picked > 0, |s: &mut State| s.pick_open = true))
            .child(text(if picked == 0 { "All kanji in these levels".to_string() } else { format!("{picked} kanji picked") }).font(SMALL).color(W.alpha(0.6)));
        if picked > 0 {
            pick = pick.child(chip("pick-clear".into(), "Use all", false, |s: &mut State| s.picked.fill(false)));
        }
        let rounds = ROUNDS.iter().enumerate().map(|(i, r)| {
            chip(format!("round-{i}"), &if *r == usize::MAX { "All".to_string() } else { r.to_string() }, s.round == i, move |s: &mut State| s.round = i)
        }).collect();
        ("Read the word, type it in romaji.",
         "Real JLPT vocab for the kanji you pick. Enter checks, - types ー, Space skips. No hints.",
         glass().w_full().gap(20.0).child(group("JLPT level", levels))
             .child(col().gap(10.0).child(caps("Kanji")).child(pick))
             .child(group("Words per round", rounds)).child(group("Time limit", limits)),
         format!("{} words · press Enter", n.min(ROUNDS[s.round])))
    };
    col().w_full().max_w(620.0).gap(24.0)
        .child(tabs)
        .child(col().gap(8.0)
            .child(text(title).font(HERO).color(W))
            .child(text(sub).font(BODY).color(W.alpha(0.6))))
        .child(panel)
        .child(row().items_center().gap(16.0)
            .child(button("start", "Start drill", true, |s: &mut State| { let d = full_deck(s); start(s, d) }))
            .child(text(count).font(SMALL).color(W.alpha(0.55))))
}

fn pick_dialog(s: &State) -> El {
    let close = |s: &mut State| s.pick_open = false;
    let grid = row().w_full().flex_wrap().gap(6.0).children(KANJI.iter().enumerate().filter(|(_, k)| level_on(s, k.n)).map(|(i, k)| {
        let on = s.picked[i];
        col().key(format!("k-{i}")).role("checkbox").label(format!("{} {}", k.c, k.m)).checked(on).focusable(true)
            .size(44.0, 44.0).radius(12.0).center()
            .fill(if on { SAKURA.alpha(0.9) } else { W.alpha(0.06) }).border(1.0, W.alpha(if on { 0.0 } else { 0.14 }))
            .on_press(move |s: &mut State| s.picked[i] = !s.picked[i])
            .child(text(k.c.clone()).font(jp(s, 22.0)).color(if on { INK } else { W }))
    }));
    let n = s.picked.iter().enumerate().filter(|(k, on)| **on && level_on(s, KANJI[*k].n)).count();
    let panel = glass().key("pick").w(680.0).gap(16.0).fill(rgb(0x1A1433).alpha(0.75)).modal(true, close)
        .child(row().w_full().items_center().justify_between()
            .child(col().gap(2.0)
                .child(text("Pick kanji").font(TextStyle::new(20.0, 600.0)).color(W))
                .child(text(if n == 0 { "None picked: every kanji in your levels is used.".to_string() } else { format!("{n} picked: words using any of them.") })
                    .font(SMALL).color(W.alpha(0.6))))
            .child(row().gap(8.0)
                .child(chip("pick-none".into(), "Clear", false, |s: &mut State| s.picked.fill(false)))
                .child(chip("pick-done".into(), "Done", true, close))))
        .child(col().w_full().h(420.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(grid));
    col().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.45)).child(panel)
}

fn play(s: &State) -> El {
    let limit = LIMITS[s.limit];
    let elapsed = if s.started < 0.0 { 0.0 } else { s.now - s.started };
    let practice = limit.is_infinite();
    let left = if practice { elapsed } else { limit - elapsed };
    let frac = if practice { 1.0 } else { (left / limit).clamp(0.0, 1.0) as f32 };
    let urgent = !practice && left < 10.0;
    let bar = row().w_full().h(6.0).radius(3.0).fill(W.alpha(0.1))
        .child(el().h(6.0).radius(3.0).grow(frac.max(0.001))
            .linear_gradient(PI / 2.0, if urgent { [(0.0, SHU), (1.0, rgb(0xFFB36B))] } else { [(0.0, SAKURA), (1.0, rgb(0xB59CFF))] }))
        .child(el().grow(1.0 - frac));
    let top = row().w_full().items_center().justify_between()
        .child(row().gap(12.0).items_center()
            .child(text(clock(left)).font(NUM).color(if urgent { SHU } else { W }))
            .child(text(if practice { "practice · no clock" } else { "" }).font(SMALL).color(W.alpha(0.5))))
        .child(text(format!("{} / {}", s.pos, s.deck.len())).font(NUM).color(W.alpha(0.8)));

    let card = s.deck[s.pos.min(s.deck.len() - 1)];
    let typed = if s.input.is_empty() { "·".to_string() } else { s.input.clone() };
    let main = glass().key("card").w(320.0).h(300.0).center().gap(6.0)
        .border(1.0, if s.wrong { SHU.alpha(0.9) } else { W.alpha(0.18) })
        .child(text(shown(card)).font(jp(s, match shown(card).chars().count() { 1 => 150.0, 2 => 112.0, 3 => 84.0, _ => 62.0 })).color(W));
    let next = col().gap(14.0).items_center().children(s.deck.iter().skip(s.pos + 1).take(5).enumerate().map(|(i, c)| {
        text(shown(*c)).font(jp(s, 34.0 - i as f32 * 3.0)).color(W.alpha(0.5 - i as f32 * 0.08))
    }));
    col().w_full().max_w(620.0).gap(26.0).items_center()
        .child(col().w_full().gap(12.0).child(top).child(bar.opacity(if practice { 0.35 } else { 1.0 })))
        .child(row().gap(36.0).items_center().child(main).child(next))
        .child(col().items_center().gap(6.0)
            .child(text(typed).font(TYPED).color(if s.input.is_empty() { W.alpha(0.3) } else { W }))
            .child(el().w(160.0).h(2.0).radius(1.0).fill(W.alpha(0.3)))
            // What you've typed, as kana: your own input, not the answer.
            .child(text(if vocab_round(s) { let (k, rest) = vocab::to_kana(&s.input); format!("{k}{rest} ") } else { " ".into() })
                .font(jp(s, 18.0)).color(W.alpha(0.55))))
        .child(text(if vocab_round(s) { "Enter checks · Space skips · Esc quits" } else { "Space skips · Esc quits" }).font(SMALL).color(W.alpha(0.4)))
}

fn done(s: &State) -> El {
    let mins = (s.ended / 60.0).max(1.0 / 60.0);
    let acc = if s.keys == 0 { 100.0 } else { 100.0 * (s.keys - s.bad_keys) as f64 / s.keys as f64 };
    let tile = |name: &str, v: String| col().grow(1.0).gap(6.0).items_center().child(caps(name)).child(text(v).font(NUM).color(W));
    let stats = glass().w_full().child(row().w_full()
        .child(tile("Correct", format!("{}/{}", s.pos - s.skips, s.deck.len())))
        .child(tile("Time", words(s.ended)))
        .child(tile("Cards / min", format!("{:.0}", (s.pos - s.skips) as f64 / mins)))
        .child(tile("Accuracy", format!("{acc:.0}%"))));
    let missed = glass().w_full().gap(14.0)
        .child(caps(if s.missed.is_empty() { "No misses. Clean run." } else { "Missed: drill these" }))
        .child(row().flex_wrap().gap(8.0).children(s.missed.iter().map(|c| {
            let (reading, meaning) = match c { Card::Kana(i, _) => (TABLE[*i].roma[0].to_string(), String::new()), Card::Word(i) => (WORDS[*i].r.clone(), WORDS[*i].m.clone()) };
            row().h(40.0).px(14.0).gap(8.0).items_center().radius(20.0).fill(SHU.alpha(0.16)).border(1.0, SHU.alpha(0.4))
                .child(text(shown(*c)).font(jp(s, 20.0)).color(W))
                .child(text(reading).font(jp(s, 14.0)).color(W.alpha(0.8)))
                .child(text(meaning).font(SMALL).color(W.alpha(0.6)).max_w(320.0).truncate())
        })));
    let mut buttons = row().gap(12.0).child(button("retry", "Retry", true, |s: &mut State| { let d = full_deck(s); start(s, d) }));
    if !s.missed.is_empty() {
        buttons = buttons.child(button("review", "Review misses", false, |s: &mut State| { let d = s.missed.clone(); start(s, d) }));
    }
    buttons = buttons.child(button("menu", "Menu", false, |s: &mut State| s.screen = Screen::Menu));
    col().w_full().max_w(620.0).gap(22.0)
        .child(col().gap(6.0)
            .child(text(if s.cleared { "全部 Cleared!" } else { "Time's up" }).font(HERO).color(if s.cleared { MATCHA } else { W }))
            .child(text(if s.cleared { "Every card done inside the limit. Try a shorter clock." } else { "Keep going: review the misses, then go again." })
                .font(BODY).color(W.alpha(0.6))))
        .child(stats)
        .child(missed)
        .child(buttons)
}

fn fonts_dialog(s: &State) -> El {
    let close = |s: &mut State| s.fonts_open = false;
    let ui = UI_FONTS.iter().enumerate().map(|(i, (name, fam))| {
        chip_in(format!("ui-font-{i}"), name, TextStyle::new(14.0, 500.0).family(Family::Named(fam)), s.ui_font == i,
            move |s: &mut State, tx: &mut Tx| { s.ui_font = i; tx.set_font_families(UI_FONTS[i].1, "Geist Mono") })
    }).collect::<Vec<_>>();
    let ja = JP_FONTS.iter().enumerate().map(|(i, (name, fam))| {
        chip_in(format!("jp-font-{i}"), &format!("{name}  あア"), TextStyle::new(14.0, 400.0).family(Family::Named(fam)), s.jp_font == i,
            move |s: &mut State, _: &mut Tx| s.jp_font = i)
    }).collect::<Vec<_>>();
    let panel = glass().key("fonts").w(440.0).gap(20.0).fill(rgb(0x1A1433).alpha(0.7)).modal(true, close)
        .child(row().w_full().items_center().justify_between()
            .child(text("Fonts").font(TextStyle::new(20.0, 600.0)).color(W))
            .child(chip("fonts-done".into(), "Done", false, close)))
        .child(col().gap(10.0).child(caps("Interface")).child(row().flex_wrap().gap(8.0).children(ui)))
        .child(col().gap(10.0).child(caps("Japanese")).child(row().flex_wrap().gap(8.0).children(ja)))
        .child(col().w_full().gap(4.0).padding(16.0).radius(16.0).fill(W.alpha(0.06))
            .child(text("あいうえお かきくけこ").font(jp(s, 28.0)).color(W))
            .child(text("アイウエオ きゃ しゅ ちょ").font(jp(s, 28.0)).color(W.alpha(0.8)))
            .child(text("The quick kana drill · 0123456789").font(BODY).color(W.alpha(0.6))));
    col().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.45)).child(panel)
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let orb = |x: f32, y: f32, d: f32, c: Color| el().size(d, d).absolute(x, y)
        .radial_gradient([0.5, 0.5], d / 2.0, [(0.0, c.alpha(0.6)), (0.6, c.alpha(0.18)), (1.0, c.alpha(0.0))]);
    let body = match s.screen { Screen::Menu => menu(s), Screen::Play => play(s), Screen::Done => done(s) };
    let root = col().w_full().h_full().linear_gradient(PI, [(0.0, rgb(0x0E0A22)), (1.0, rgb(0x2A1745))])
        .child(orb(-160.0, -140.0, 520.0, SHU))
        .child(orb(560.0, 260.0, 560.0, rgb(0x7B5CFF)))
        .child(orb(200.0, 520.0, 420.0, SAKURA))
        .child(col().w_full().h_full().padding(28.0).gap(24.0)
            .child(row().w_full().items_center().justify_between()
                .child(logo(s))
                .child(if s.screen == Screen::Menu {
                    chip("fonts-open".into(), "Aa  Fonts", false, |s: &mut State| s.fonts_open = true)
                } else {
                    text("かな · カナ").font(jp(s, 13.0)).color(W.alpha(0.5))
                }))
            .child(col().w_full().grow(1.0).items_center().justify_center().child(body)));
    match s.screen {
        Screen::Menu if s.fonts_open => root.child(fonts_dialog(s)),
        Screen::Menu if s.pick_open => root.child(pick_dialog(s)),
        _ => root,
    }
}

fn main() {
    rune::run(AppSpec::<State> {
        fonts: &[
            include_bytes!("../assets/Geist-Light.ttf"),
            include_bytes!("../assets/Geist-Regular.ttf"),
            include_bytes!("../assets/Geist-Medium.ttf"),
            include_bytes!("../assets/Geist-SemiBold.ttf"),
            include_bytes!("../assets/GeistMono-Regular.ttf"),
            include_bytes!("../assets/GeistMono-Medium.ttf"),
            include_bytes!("../assets/GeistMono-SemiBold.ttf"),
            include_bytes!("../assets/Inter-Regular.ttf"),
            include_bytes!("../assets/Inter-Medium.ttf"),
            include_bytes!("../assets/Inter-SemiBold.ttf"),
        ],
        families: ("Geist", "Geist Mono"),
        poll: Some(poll),
        on_key: Some(on_key),
        min_size: Some((760.0, 720.0)),
        scenes: &[
            ("play", |s| { s.sets = [true, true, true]; let d = full_deck(s); start(s, d); s.input = "k".into(); }),
            ("practice", |s| { s.limit = 4; let d = full_deck(s); start(s, d); s.now = 42.0; s.started = 0.0; }),
            ("fonts", |s| { s.fonts_open = true; s.jp_font = 1; }),
            ("vocab", |s| { s.drill = Drill::Vocab; s.levels = [true, true, false, false, false]; }),
            ("vocab-play", |s| { s.drill = Drill::Vocab; let d = full_deck(s); start(s, d); s.input = "nihon".into(); }),
            ("vocab-wrong", |s| { s.drill = Drill::Vocab; let d = full_deck(s); start(s, d); s.input = "xyz".into(); submit(s); s.input = "ga".into(); }),
            ("pick", |s| { s.drill = Drill::Vocab; s.pick_open = true; for i in [0, 3, 7, 12] { s.picked[i] = true } }),
            ("vocab-done", |s| {
                s.drill = Drill::Vocab; let d = full_deck(s); start(s, d);
                for _ in 0..3 { skip(s) }
                s.limit = 2; s.started = 0.0; s.now = 75.0; finish(s, false);
            }),
            ("miss", |s| { let d = full_deck(s); start(s, d); type_char(s, 'q'); }),
            ("done", |s| {
                let d = full_deck(s); start(s, d);
                for _ in 0..30 { let Card::Kana(i, _) = s.deck[s.pos] else { break }; for c in TABLE[i].roma[0].chars() { type_char(s, c) } }
                for &c in &s.deck.clone()[30..34] { if !s.missed.contains(&c) { s.missed.push(c) } }
                s.keys += 10; s.bad_keys += 4; s.limit = 2; s.started = 0.0; s.now = 99.8; finish(s, false);
            }),
        ],
        ..AppSpec::new("maijp", (920.0, 800.0), init, view)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(s: &mut State, t: &str) { for c in t.chars() { type_char(s, c) } }

    fn round(cards: &[&str]) -> State {
        let mut s = init();
        let deck: Vec<Card> = cards.iter().map(|k| Card::Kana(TABLE.iter().position(|e| e.kana == *k).unwrap(), false)).collect();
        start(&mut s, deck.clone());
        s.deck = deck; // fixed order for the test
        s
    }

    #[test]
    fn table_has_every_set() {
        let count = |set| TABLE.iter().filter(|k| k.set == set).count();
        assert_eq!((count(0), count(1), count(2)), (46, 25, 33));
        assert!(TABLE.iter().all(|k| !k.roma.is_empty()));
        assert_eq!(katakana("しゃ"), "シャ");
        assert_eq!(katakana("ん"), "ン");
    }

    #[test]
    fn variants_misses_and_n() {
        let mut s = round(&["し", "ん", "な", "ちゃ"]);
        type_str(&mut s, "si");
        assert_eq!(s.pos, 1);
        type_str(&mut s, "nn"); // ん then the habitual second n: swallowed
        assert_eq!((s.pos, s.bad_keys), (2, 0));
        type_str(&mut s, "x"); // な: wrong key shows the hint and stays
        assert!(s.wrong && s.pos == 2 && s.bad_keys == 1);
        type_str(&mut s, "na");
        type_str(&mut s, "tya");
        assert_eq!(s.screen, Screen::Done);
        assert!(s.cleared);
        assert_eq!(s.missed.len(), 1);
    }

    #[test]
    fn vocab_round_checks_whole_reading() {
        let mut s = init();
        s.drill = Drill::Vocab;
        let i = WORDS.iter().position(|w| w.w == "学校").expect("学校 in data");
        start(&mut s, vec![Card::Word(i)]);
        s.input = "gakkoo".into();
        submit(&mut s);
        assert!(s.wrong && s.pos == 0 && s.missed.len() == 1);
        s.input = "gakkou".into();
        submit(&mut s);
        assert!(s.cleared && s.screen == Screen::Done);
    }

    #[test]
    fn picked_kanji_filter_the_deck() {
        let mut s = init();
        s.drill = Drill::Vocab;
        let all = full_deck(&s).len();
        let k = KANJI.iter().position(|k| k.c == "学").unwrap();
        s.picked[k] = true;
        let some = full_deck(&s);
        assert!(!some.is_empty() && some.len() < all);
        assert!(some.iter().all(|c| matches!(c, Card::Word(i) if WORDS[*i].kanji.contains(&k))));
    }

    #[test]
    fn time_in_words() {
        assert_eq!(words(65.2), "1 min 5 secs");
        assert_eq!(words(99.8), "1 min 40 secs");
        assert_eq!(words(60.0), "1 min");
        assert_eq!(words(1.0), "1 sec");
    }

    #[test]
    fn space_skips_as_a_miss() {
        let mut s = round(&["あ", "い"]);
        skip(&mut s);
        assert_eq!((s.pos, s.missed.len(), s.skips, s.bad_keys), (1, 1, 1, 1));
        type_str(&mut s, "i");
        assert!(s.cleared && s.screen == Screen::Done);
    }
}
