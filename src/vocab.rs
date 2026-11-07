//! JLPT vocab drill data and romaji checking.
//! Data: kanji-study-app's yosida.com JLPT kanji levels plus JMdict common words
//! (JMdict (c) EDRDG, CC BY-SA 4.0), trimmed by hand into assets/*.json.

use std::collections::HashMap;
use std::sync::LazyLock;

pub struct Kanji { pub c: String, pub n: u8, pub m: String, pub on: Vec<String>, pub kun: Vec<String> }
pub struct Word { pub w: String, pub r: String, pub m: String, pub kanji: Vec<usize>, pub level: u8 }

fn strs(v: &serde_json::Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

pub static KANJI: LazyLock<Vec<Kanji>> = LazyLock::new(|| {
    let v: serde_json::Value = serde_json::from_str(include_str!("../assets/kanji.json")).expect("kanji.json");
    v.as_array().expect("kanji list").iter().map(|k| Kanji {
        c: k["c"].as_str().unwrap_or_default().to_string(),
        n: k["n"].as_u64().unwrap_or(1) as u8,
        m: strs(&k["m"]).join(", "),
        on: strs(&k["on"]),
        kun: strs(&k["kun"]),
    }).collect()
});

pub static WORDS: LazyLock<Vec<Word>> = LazyLock::new(|| {
    let at: HashMap<&str, usize> = KANJI.iter().enumerate().map(|(i, k)| (k.c.as_str(), i)).collect();
    let v: serde_json::Value = serde_json::from_str(include_str!("../assets/words.json")).expect("words.json");
    v.as_array().expect("word list").iter().filter_map(|w| {
        let kanji: Vec<usize> = w["k"].as_str()?.chars().filter_map(|c| at.get(c.to_string().as_str()).copied()).collect();
        // A word is as hard as its hardest kanji (N1 = 1).
        let level = kanji.iter().map(|&i| KANJI[i].n).min()?;
        Some(Word { w: w["w"].as_str()?.into(), r: norm(w["r"].as_str()?), m: strs(&w["m"]).join(", "), kanji, level })
    }).collect()
});

/// Words using each kanji, by KANJI index.
pub static KANJI_WORDS: LazyLock<Vec<Vec<usize>>> = LazyLock::new(|| {
    let mut by = vec![Vec::new(); KANJI.len()];
    for (w, word) in WORDS.iter().enumerate() { for &k in &word.kanji { by[k].push(w) } }
    by
});

/// Katakana to hiragana, and the spellings romaji can't tell apart (ぢ/じ, づ/ず) folded together.
pub fn norm(s: &str) -> String {
    s.chars().filter(|c| *c != '・').map(|c| match c {
        'ァ'..='ヶ' => char::from_u32(c as u32 - 0x60).unwrap(),
        'ぢ' => 'じ',
        'づ' => 'ず',
        c => c,
    }).collect()
}

/// romaji -> kana, from the kana table plus the loanword sounds katakana words need.
static ROMA: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    for k in crate::TABLE.iter() {
        for r in &k.roma { m.entry(*r).or_insert(k.kana); } // first wins: ji -> じ, zu -> ず
    }
    for (r, k) in [("she", "しぇ"), ("che", "ちぇ"), ("je", "じぇ"), ("fa", "ふぁ"), ("fi", "ふぃ"), ("fe", "ふぇ"),
                   ("fo", "ふぉ"), ("thi", "てぃ"), ("dhi", "でぃ"), ("wi", "うぃ"), ("we", "うぇ"), ("va", "ゔぁ"),
                   ("vi", "ゔぃ"), ("vu", "ゔ"), ("ve", "ゔぇ"), ("vo", "ゔぉ"), ("-", "ー")] {
        m.insert(r, k);
    }
    m.remove("n"); // ん is decided by what follows, below
    m.remove("nn");
    m
});

/// Converts typed romaji to kana. Returns the kana and any tail that isn't kana yet.
pub fn to_kana(input: &str) -> (String, String) {
    let b = input.as_bytes();
    let vowel = |i: usize| b.get(i).is_some_and(|c| b"aiueoy".contains(c));
    let (mut out, mut i) = (String::new(), 0);
    while i < b.len() {
        let rest = &input[i..];
        if b[i] == b'n' && !vowel(i + 1) {
            out.push('ん');
            // "nn" is ん, unless the second n starts the next mora (konnichi -> こん|に).
            i += if b.get(i + 1) == Some(&b'n') && !vowel(i + 2) { 2 } else { 1 };
            if b.get(i) == Some(&b'\'') { i += 1 }
            continue;
        }
        if rest.starts_with("tch") || (b.len() > i + 1 && b[i] == b[i + 1] && !b"aiueon-".contains(&b[i])) {
            out.push('っ');
            i += 1;
            continue;
        }
        let Some((len, kana)) = (1..=3).rev().filter(|l| i + l <= b.len()).find_map(|l| ROMA.get(&input[i..i + l]).map(|k| (l, *k))) else {
            return (out, rest.to_string());
        };
        out.push_str(kana);
        i += len;
    }
    (out, String::new())
}

/// Whether typed romaji spells the word's reading.
pub fn is_reading(w: &Word, input: &str) -> bool {
    let (kana, rest) = to_kana(input);
    rest.is_empty() && norm(&kana) == w.r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(r: &str) -> Word { Word { w: String::new(), r: norm(r), m: String::new(), kanji: vec![], level: 5 } }

    #[test]
    fn romaji_spells_readings() {
        assert!(is_reading(&word("こんにちは"), "konnichiha"));
        assert!(is_reading(&word("がっこう"), "gakkou"));
        assert!(is_reading(&word("まっちゃ"), "matcha"));
        assert!(is_reading(&word("しんぶん"), "shinbun"));
        assert!(is_reading(&word("きんえん"), "kin'en"));
        assert!(is_reading(&word("はなぢ"), "hanaji"));
        assert!(is_reading(&word("ローマじ"), "ro-maji"));
        assert!(is_reading(&word("チェック"), "chekku"));
        assert!(is_reading(&word("ほん"), "honn"));
        assert!(!is_reading(&word("ほん"), "hon x"));
        assert!(!is_reading(&word("げんご"), "gengu"));
    }

    #[test]
    fn data_loads_with_levels() {
        assert!(KANJI.len() > 2000);
        assert!(WORDS.len() > 9000);
        assert!(WORDS.iter().all(|w| (1..=5).contains(&w.level) && !w.r.is_empty()));
        // every reading is typeable
        let bad: Vec<_> = WORDS.iter().filter(|w| { let (_, rest) = to_kana(&romaji(&w.r)); !rest.is_empty() }).map(|w| &w.w).take(5).collect();
        assert!(bad.is_empty(), "{bad:?}");
    }

    /// Kana -> one romaji spelling, only to prove every reading round-trips.
    fn romaji(kana: &str) -> String {
        let back: HashMap<&str, &str> = ROMA.iter().map(|(r, k)| (*k, *r)).collect();
        let cs: Vec<char> = kana.chars().collect();
        let (mut out, mut i) = (String::new(), 0);
        while i < cs.len() {
            if cs[i] == 'ん' { out.push_str("n'"); i += 1; continue }
            if cs[i] == 'っ' { i += 1; continue } // dropped: still parses, just not the same word
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            if let Some(r) = back.get(two.as_str()).filter(|_| two.chars().count() == 2) { out.push_str(r); i += 2; continue }
            match back.get(cs[i].to_string().as_str()) { Some(r) => out.push_str(r), None => out.push('#') }
            i += 1;
        }
        out
    }
}
