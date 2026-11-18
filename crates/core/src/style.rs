//! The style grade (T6): how stylish one fight was. Every landed hit
//! scores its damage times a percentage: more in the air, more on an
//! airborne target, more for each extra target one move reaches, more for
//! a slam, and half for a string repeated from the last few. Named combos
//! and air time add points; a bite takes them away. The grade is points
//! per point of damage dealt, so a one-dog fight and a five-dog fight
//! grade alike. Numbers are in `data/style.json`.
use serde::Deserialize;

const STYLE_JSON: &str = include_str!("../../../data/style.json");

#[derive(Debug, Clone, Deserialize)]
pub struct StyleSpec {
    pub base_pct: i32,
    pub air_move_pct: i32,
    pub airborne_target_pct: i32,
    pub extra_target_pct: i32,
    pub slam_pct: i32,
    pub stale_pct: i32,
    pub recent_strings: usize,
    pub combo_points: i32,
    pub air_ticks_per_point: u32,
    pub bitten_times: i32,
    pub grades: Vec<(String, i32)>,
}

impl StyleSpec {
    pub fn shipped() -> StyleSpec {
        serde_json::from_str(STYLE_JSON).expect("style.json is checked by its test")
    }
}

/// What one landed hit was, for scoring.
pub struct HitKind {
    pub damage: i32,
    pub air_move: bool,
    pub airborne_target: bool,
    /// 0 for the move's first target, 1 for its second, and so on
    pub nth_target: u32,
    pub slam: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub points: i32,
    pub dealt: i32,
    /// the moves of the string under way, by name
    pub current: Vec<String>,
    /// the last few finished strings of two moves or more
    pub recent: Vec<Vec<String>>,
    pub air_ticks: u32,
    pub most_at_once: u32,
    pub slams: u32,
    pub repeats: u32,
    pub combos: u32,
    pub bites: u32,
    pub longest_air_chain: u32,
}

impl Style {
    /// A move starts: it joins the string under way.
    pub fn started(&mut self, name: &str) {
        self.current.push(name.to_string());
    }

    /// The string under way repeats one of the recent strings so far.
    pub fn stale(&self) -> bool {
        self.current.len() >= 2 && self.recent.iter().any(|r| r.starts_with(&self.current))
    }

    /// The string ends: it becomes one of the recent strings.
    pub fn ended(&mut self, spec: &StyleSpec) {
        let s = std::mem::take(&mut self.current);
        if s.len() >= 2 {
            if self.recent.contains(&s) {
                self.repeats += 1;
            }
            self.recent.push(s);
            let keep = spec.recent_strings;
            if self.recent.len() > keep {
                self.recent.drain(..self.recent.len() - keep);
            }
        }
    }

    pub fn hit(&mut self, spec: &StyleSpec, h: &HitKind) {
        let mut pct = spec.base_pct;
        if h.air_move {
            pct += spec.air_move_pct;
        }
        if h.airborne_target {
            pct += spec.airborne_target_pct;
        }
        if h.nth_target > 0 {
            pct += spec.extra_target_pct;
        }
        if h.slam {
            pct += spec.slam_pct;
        }
        if self.stale() {
            pct = pct * spec.stale_pct / 100;
        }
        self.points += h.damage * pct / 100;
        self.dealt += h.damage;
        self.most_at_once = self.most_at_once.max(h.nth_target + 1);
        if h.slam && h.nth_target == 0 {
            self.slams += 1;
        }
    }

    pub fn combo(&mut self, spec: &StyleSpec) {
        self.combos += 1;
        self.points += spec.combo_points;
    }

    /// One tick in the air with an air combo going.
    pub fn air_tick(&mut self, spec: &StyleSpec) {
        self.air_ticks += 1;
        if self.air_ticks.is_multiple_of(spec.air_ticks_per_point.max(1)) {
            self.points += 1;
        }
    }

    pub fn bitten(&mut self, spec: &StyleSpec, bite: i32) {
        self.bites += 1;
        self.points = (self.points - bite * spec.bitten_times).max(0);
    }

    /// Points per point of damage dealt, in percent; 0 before any hit.
    pub fn pct(&self) -> i32 {
        if self.dealt == 0 {
            0
        } else {
            self.points * 100 / self.dealt
        }
    }

    /// The grade's letter, or None before the first hit.
    pub fn grade<'a>(&self, spec: &'a StyleSpec) -> Option<&'a str> {
        if self.dealt == 0 {
            return None;
        }
        let p = self.pct();
        spec.grades
            .iter()
            .find(|(_, at)| p >= *at)
            .map(|(g, _)| g.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(damage: i32) -> HitKind {
        HitKind {
            damage,
            air_move: false,
            airborne_target: false,
            nth_target: 0,
            slam: false,
        }
    }

    /// Worked by hand from data/style.json: a plain hit of 10 scores 10
    /// (100%); an air move on an airborne dog scores 10 x 175% = 17; a
    /// slam (an air move) on its second target scores 30 x (100 + 50 + 50 + 100)% = 90.
    #[test]
    fn hits_score_by_hand() {
        let s = StyleSpec::shipped();
        let mut st = Style::default();
        st.hit(&s, &plain(10));
        assert_eq!(st.points, 10);
        st.hit(
            &s,
            &HitKind {
                damage: 10,
                air_move: true,
                airborne_target: true,
                nth_target: 0,
                slam: false,
            },
        );
        assert_eq!(st.points, 10 + 17);
        st.hit(
            &s,
            &HitKind {
                damage: 30,
                air_move: true,
                airborne_target: false,
                nth_target: 1,
                slam: true,
            },
        );
        assert_eq!(st.points, 10 + 17 + 90);
        assert_eq!(st.most_at_once, 2);
        assert_eq!(st.dealt, 50);
    }

    /// The same string again scores half from its second move on; a
    /// different string scores in full.
    #[test]
    fn a_repeated_string_scores_half() {
        let s = StyleSpec::shipped();
        let mut st = Style::default();
        for name in ["light_1", "light_2"] {
            st.started(name);
            st.hit(&s, &plain(10));
        }
        st.ended(&s);
        assert_eq!(st.points, 20);
        st.started("light_1");
        st.hit(&s, &plain(10));
        assert_eq!(st.points, 30, "one move alone is never stale");
        st.started("light_2");
        st.hit(&s, &plain(10));
        assert_eq!(st.points, 35, "the repeat scores 10 x 50%");
        st.ended(&s);
        assert_eq!(st.repeats, 1);
        st.started("light_1");
        st.started("heavy_1");
        st.hit(&s, &plain(20));
        assert_eq!(st.points, 55, "a new string scores in full");
    }

    /// Grades by hand: 200 points for 100 damage is 200% (S); 90 is C;
    /// a bite of 8 takes 16 points.
    #[test]
    fn grades_by_hand() {
        let s = StyleSpec::shipped();
        let mut st = Style::default();
        assert_eq!(st.grade(&s), None);
        st.dealt = 100;
        st.points = 200;
        assert_eq!(st.grade(&s), Some("S"));
        st.points = 90;
        assert_eq!(st.grade(&s), Some("C"));
        st.bitten(&s, 8);
        assert_eq!(st.points, 74);
        assert_eq!(st.grade(&s), Some("D"));
    }
}
