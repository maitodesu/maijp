//! What maijp remembers between launches, kept by rune's `Prefs` (JSON in the config dir):
//! settings, and progress (per-card right/seen counts plus a history of rounds).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "dev.maitodesu.maijp";

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Settings {
    pub script: usize,
    pub sets: [bool; 3],
    pub limit: usize,
    pub levels: [bool; 5],
    pub round: usize,
    /// Picked kanji, as the characters themselves so a data refresh can't shift them.
    pub picked: Vec<String>,
    pub ui_font: usize,
    pub jp_font: usize,
    /// 0 follows the system, 1 light, 2 dark.
    pub theme: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { script: 0, sets: [true, false, false], limit: 1, levels: [true, false, false, false, false],
            round: 0, picked: Vec::new(), ui_font: 0, jp_font: 0, theme: 0 }
    }
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq, Debug)]
pub struct Stat {
    pub seen: u32,
    /// Answered right on the first try.
    pub right: u32,
    /// Day (since the Unix epoch) it was last answered.
    pub day: u32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Run {
    pub day: u32,
    pub drill: String,
    pub right: usize,
    pub total: usize,
    pub secs: f64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Progress {
    /// Keyed `kana:あ`, `kana:ア` or `word:学校`.
    pub stats: HashMap<String, Stat>,
    pub history: Vec<Run>,
}

const HISTORY: usize = 200;

impl Progress {
    pub fn record(&mut self, key: String, right: bool, day: u32) {
        let st = self.stats.entry(key).or_default();
        st.seen += 1;
        st.right += right as u32;
        st.day = day;
    }

    pub fn add_run(&mut self, run: Run) {
        self.history.push(run);
        let extra = self.history.len().saturating_sub(HISTORY);
        self.history.drain(..extra);
    }

    /// Consecutive days with a round, ending today (or yesterday, so a streak survives until midnight).
    pub fn streak(&self, today: u32) -> u32 {
        let days: HashSet<u32> = self.history.iter().map(|r| r.day).collect();
        let mut d = if days.contains(&today) { today } else { today.saturating_sub(1) };
        let mut n = 0;
        while days.contains(&d) { n += 1; d = d.wrapping_sub(1) }
        n
    }

    pub fn totals(&self) -> (u32, u32) {
        self.stats.values().fold((0, 0), |(s, r), st| (s + st.seen, r + st.right))
    }
}

pub fn today() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| (d.as_secs() / 86_400) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(day: u32) -> Run { Run { day, drill: "kana".into(), right: 1, total: 1, secs: 1.0 } }

    #[test]
    fn streak_counts_back_from_today_or_yesterday() {
        let mut p = Progress::default();
        assert_eq!(p.streak(100), 0);
        for d in [97, 98, 99] { p.add_run(run(d)) }
        assert_eq!(p.streak(100), 3, "yesterday still counts");
        p.add_run(run(100));
        assert_eq!(p.streak(100), 4);
        assert_eq!(p.streak(102), 0);
    }

    #[test]
    fn records_and_round_trips_through_prefs() {
        let mut p = Progress::default();
        p.record("kana:あ".into(), true, 5);
        p.record("kana:あ".into(), false, 6);
        assert_eq!(p.stats["kana:あ"], Stat { seen: 2, right: 1, day: 6 });
        for d in 0..250 { p.add_run(run(d)) }
        assert_eq!((p.history.len(), p.history[0].day), (200, 50));

        let mut prefs = rune::prefs::Prefs::memory();
        prefs.set("progress", &p).unwrap();
        let back: Progress = prefs.get("progress").unwrap();
        assert_eq!(back.stats, p.stats);
        // Old files missing new fields still load.
        prefs.set("settings", &serde_json::json!({"limit": 3})).unwrap();
        let s: Settings = prefs.get("settings").unwrap();
        assert_eq!((s.limit, s.sets), (3, [true, false, false]));
    }
}
