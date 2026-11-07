//! Study tab: browse kana and kanji at your own pace, with your progress on each.

use rune::{Color, El, ScrollSpec, TextStyle, col, el, row, text};

use crate::store::{Stat, today};
use crate::vocab::{KANJI, KANJI_WORDS, WORDS};
use crate::*;

const AMBER: Color = rgb(0xFFC56B);

/// Colour for how well something is known: none yet, shaky, getting there, solid.
fn tone(seen: u32, right: u32) -> Option<Color> {
    if seen == 0 { return None }
    let acc = right as f32 / seen as f32;
    Some(if acc < 0.6 { SHU } else if acc < 0.85 { AMBER } else { MATCHA })
}

fn stat<'a>(s: &'a State, key: &str) -> Option<&'a Stat> { s.progress.stats.get(key) }

fn kanji_score(s: &State, k: usize) -> (u32, u32) {
    KANJI_WORDS[k].iter().filter_map(|&w| stat(s, &format!("word:{}", WORDS[w].w)))
        .fold((0, 0), |(a, b), st| (a + st.seen, b + st.right))
}

fn tinted(c: El, t: Option<Color>) -> El {
    match t {
        Some(c2) => c.fill(c2.alpha(0.22)).border(1.0, c2.alpha(0.55)),
        None => c.fill(W.alpha(0.06)).border(1.0, W.alpha(0.14)),
    }
}

fn ago(day: u32) -> String {
    match today().saturating_sub(day) { 0 => "today".into(), 1 => "yesterday".into(), n => format!("{n} days ago") }
}

fn progress(s: &State) -> El {
    let (seen, right) = s.progress.totals();
    let acc = if seen == 0 { "–".to_string() } else { format!("{:.0}%", 100.0 * right as f64 / seen as f64) };
    let streak = s.progress.streak(today());
    let tile = |name: &str, v: String| col().grow(1.0).gap(4.0).items_center().child(caps(name)).child(text(v).font(NUM).color(W));
    let recent = s.progress.history.iter().rev().take(5).map(|r| {
        row().w_full().items_center().gap(12.0).h(30.0)
            .child(text(if r.drill == "vocab" { "JLPT vocab" } else { "Kana" }).font(BODY).color(W).w(110.0))
            .child(text(format!("{}/{}", r.right, r.total)).font(BODY).color(W.alpha(0.85)).w(70.0))
            .child(text(words(r.secs)).font(SMALL).color(W.alpha(0.6)).grow(1.0))
            .child(text(ago(r.day)).font(SMALL).color(W.alpha(0.5)))
    }).collect::<Vec<_>>();
    let mut card = glass().w_full().gap(18.0)
        .child(row().w_full()
            .child(tile("Streak", format!("{streak} day{}", if streak == 1 { "" } else { "s" })))
            .child(tile("Answers", seen.to_string()))
            .child(tile("Accuracy", acc))
            .child(tile("Rounds", s.progress.history.len().to_string())));
    card = if recent.is_empty() {
        card.child(text("No rounds yet. Study a little, then try a drill.").font(SMALL).color(W.alpha(0.55)))
    } else {
        card.child(col().w_full().gap(2.0).child(caps("Recent")).children(recent))
    };
    card
}

fn legend() -> El {
    let dot = |c: Option<Color>, label: &str| row().gap(6.0).items_center()
        .child(tinted(el().size(12.0, 12.0).radius(4.0), c))
        .child(text(label.to_string()).font(SMALL).color(W.alpha(0.6)));
    row().gap(16.0).flex_wrap()
        .child(dot(None, "new")).child(dot(Some(SHU), "shaky")).child(dot(Some(AMBER), "learning")).child(dot(Some(MATCHA), "solid"))
}

fn kana_cell(s: &State, i: Option<usize>) -> El {
    let Some(i) = i else { return el().size(58.0, 66.0) };
    let card = Card::Kana(i, s.study_kata);
    let k = shown(card);
    let t = stat(s, &format!("kana:{k}")).and_then(|st| tone(st.seen, st.right));
    tinted(col().size(58.0, 66.0).radius(14.0).center().gap(2.0), t)
        .child(text(k).font(jp(s, if TABLE[i].kana.chars().count() > 1 { 20.0 } else { 26.0 })).color(W))
        .child(text(TABLE[i].roma[0]).font(TextStyle::new(11.0, 500.0)).color(W.alpha(0.6)))
}

fn grid(s: &State, rows: Vec<Vec<Option<usize>>>) -> El {
    col().gap(6.0).children(rows.into_iter().map(|r| row().gap(6.0).children(r.into_iter().map(|i| kana_cell(s, i)))))
}

fn kana_chart(s: &State) -> El {
    let of = |set| TABLE.iter().enumerate().filter(|(_, k)| k.set == set).map(|(i, _)| Some(i)).collect::<Vec<_>>();
    let basic = of(0);
    // 五十音 layout: や and わ rows have gaps, ん sits alone.
    let mut rows: Vec<Vec<Option<usize>>> = basic[..35].chunks(5).map(<[_]>::to_vec).collect();
    rows.push(vec![basic[35], None, basic[36], None, basic[37]]);
    rows.push(basic[38..43].to_vec());
    rows.push(vec![basic[43], None, None, None, basic[44]]);
    rows.push(vec![basic[45]]);
    let dakuten = of(1).chunks(5).map(<[_]>::to_vec).collect();
    let combos = of(2).chunks(3).map(<[_]>::to_vec).collect();
    let section = |title: &str, g: El| col().gap(10.0).child(caps(title)).child(g);
    glass().w_full().gap(18.0)
        .child(row().w_full().items_center().justify_between()
            .child(row().gap(8.0)
                .child(chip("study-hira".into(), "Hiragana", !s.study_kata, |s: &mut State| s.study_kata = false))
                .child(chip("study-kata".into(), "Katakana", s.study_kata, |s: &mut State| s.study_kata = true)))
            .child(legend()))
        .child(row().w_full().gap(28.0).flex_wrap()
            .child(section("五十音 · basic", grid(s, rows)))
            .child(col().gap(28.0)
                .child(section("Dakuten", grid(s, dakuten)))
                .child(section("Combos", grid(s, combos)))))
}

fn kanji_browser(s: &State) -> El {
    let levels = (0..5).map(|i| {
        let lv = 5 - i as u8;
        chip(format!("study-n{lv}"), &format!("N{lv}"), s.study_level == lv, move |s: &mut State| s.study_level = lv)
    }).collect::<Vec<_>>();
    let ks: Vec<usize> = KANJI.iter().enumerate().filter(|(_, k)| k.n == s.study_level).map(|(i, _)| i).collect();
    let solid = ks.iter().filter(|&&k| { let (a, b) = kanji_score(s, k); tone(a, b) == Some(MATCHA) }).count();
    let cells = ks.iter().map(|&i| {
        let (seen, right) = kanji_score(s, i);
        tinted(col().key(format!("sk-{i}")).role("button").label(format!("{} {}", KANJI[i].c, KANJI[i].m)).focusable(true)
            .size(48.0, 48.0).radius(12.0).center().press_scale(0.94), tone(seen, right))
            .on_press(move |s: &mut State| s.study_open = Some(i))
            .child(text(KANJI[i].c.clone()).font(jp(s, 24.0)).color(W))
    }).collect::<Vec<_>>();
    glass().w_full().gap(16.0)
        .child(row().w_full().items_center().justify_between()
            .child(row().gap(8.0).children(levels))
            .child(legend()))
        .child(text(format!("{} kanji · {solid} solid · tap one to study it", ks.len())).font(SMALL).color(W.alpha(0.6)))
        .child(row().w_full().flex_wrap().gap(6.0).children(cells))
}

pub fn view(s: &State, tabs: El) -> El {
    let sub = row().gap(8.0)
        .child(chip("study-tab-kana".into(), "Kana chart", s.study_tab == 0, |s: &mut State| s.study_tab = 0))
        .child(chip("study-tab-kanji".into(), "Kanji", s.study_tab == 1, |s: &mut State| s.study_tab = 1));
    let page = col().w_full().max_w(760.0).gap(22.0).padding(4.0)
        .child(tabs.w_full())
        .child(col().gap(8.0)
            .child(text("Learn first, drill after.").font(HERO).color(W))
            .child(text("Browse the kana and JLPT kanji. Colours show how well you know each one from your drills.").font(BODY).color(W.alpha(0.6))))
        .child(progress(s))
        .child(sub)
        .child(if s.study_tab == 0 { kana_chart(s) } else { kanji_browser(s) })
        .child(el().h(20.0));
    col().key("study").w_full().h_full().min_h(0.0).items_center().scroll(ScrollSpec { stick_bottom: false }).child(page)
}

/// A kanji's card: meanings, readings, the words that use it and how you've done on each.
pub fn kanji_card(s: &State) -> El {
    let Some(i) = s.study_open else { return el() };
    let k = &KANJI[i];
    let close = |s: &mut State| s.study_open = None;
    let readings = |label: &str, r: &[String]| row().gap(10.0).items_center()
        .child(caps(label).w(34.0))
        .child(text(if r.is_empty() { "–".to_string() } else { r.join("・") }).font(jp(s, 17.0)).color(W).max_w(380.0).wrap());
    let (seen, right) = kanji_score(s, i);
    let words = KANJI_WORDS[i].iter().take(12).map(|&w| {
        let word = &WORDS[w];
        let score = stat(s, &format!("word:{}", word.w)).map_or("new".to_string(), |st| format!("{}/{}", st.right, st.seen));
        row().w_full().h(40.0).gap(14.0).items_center()
            .child(text(word.w.clone()).font(jp(s, 20.0)).color(W).w(130.0))
            .child(text(word.r.clone()).font(jp(s, 14.0)).color(W.alpha(0.8)).w(130.0))
            .child(text(word.m.clone()).font(SMALL).color(W.alpha(0.6)).grow(1.0).min_w(0.0).truncate())
            .child(text(score).font(SMALL).color(W.alpha(0.5)))
    }).collect::<Vec<_>>();
    let deck: Vec<Card> = top_words(i, |_| true).into_iter().map(Card::Word).collect();
    let drill = if deck.is_empty() {
        text("No vocab for this kanji yet").font(SMALL).color(W.alpha(0.5))
    } else {
        button("study-drill", &format!("Drill its top {} words", deck.len()), true, move |s: &mut State| {
            s.study_open = None;
            s.drill = Drill::Vocab;
            start(s, deck.clone());
        })
    };
    let panel = glass().key("kanji-card").w(640.0).gap(18.0).fill(rgb(0x1A1433).alpha(0.75)).modal(true, close)
        .child(row().w_full().gap(24.0).items_start()
            .child(tinted(col().size(140.0, 140.0).radius(24.0).center(), tone(seen, right))
                .child(text(k.c.clone()).font(jp(s, 96.0)).color(W)))
            .child(col().grow(1.0).gap(10.0)
                .child(row().w_full().items_center().justify_between()
                    .child(text(format!("N{}", k.n)).font(CAPS).color(SAKURA))
                    .child(chip("kanji-close".into(), "Close", false, close)))
                .child(text(k.m.clone()).font(TextStyle::new(20.0, 600.0)).color(W).wrap())
                .child(readings("On", &k.on))
                .child(readings("Kun", &k.kun))
                .child(text(if seen == 0 { "Not drilled yet".to_string() } else { format!("{right}/{seen} right in drills") }).font(SMALL).color(W.alpha(0.55)))))
        .child(col().w_full().gap(4.0).child(caps("Words")).child(col().w_full().h(280.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(col().w_full().children(words))))
        .child(drill);
    col().absolute(0.0, 0.0).w_full().h_full().center().fill(Color::BLACK.alpha(0.45)).child(panel)
}
