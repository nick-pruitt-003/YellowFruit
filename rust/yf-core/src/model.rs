//! The tournament data model. TS uses an object graph with shared references (a MatchTeam points at
//! a Team, a MatchPlayer at a Player); here teams and players live in arenas on `Tournament` and are
//! referred to by index. Phases are referred to by index into `Tournament::phases`.
//!
//! Numbers are f64 everywhere because the TS code does f64 arithmetic, and parity depends on doing
//! the same operations in the same order. `Option<f64>` is a TS `number | undefined`.

use crate::js;
use crate::validation::{Collection, Msg, Status, VType};

pub type TeamId = usize;
pub type PlayerId = usize;
pub type PhaseId = usize;
pub type AnswerTypeId = usize;

/// `x || 0`
pub fn or0(x: Option<f64>) -> f64 {
    match x {
        Some(v) if !v.is_nan() => v,
        _ => 0.0,
    }
}

/// `x ?? 0`
pub fn nn0(x: Option<f64>) -> f64 {
    x.unwrap_or(0.0)
}

#[derive(Clone, Debug)]
pub struct AnswerType {
    pub value: f64,
    pub label: Option<String>,
    pub short_label: Option<String>,
}

impl AnswerType {
    pub fn new(value: f64) -> Self {
        AnswerType { value, label: None, short_label: None }
    }
    pub fn is_power(&self) -> bool {
        self.value > 10.0
    }
    pub fn is_neg(&self) -> bool {
        self.value < 0.0
    }
    pub fn label(&self) -> String {
        match &self.label {
            Some(l) if !l.is_empty() => l.clone(),
            _ => js::num_to_string(self.value),
        }
    }
    pub fn id(&self) -> String {
        format!("AnswerType_{}", self.label())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BonusTrackingMode {
    Automatic,
    ManualHeardAutoPoints,
    Manual,
}

#[derive(Clone, Debug)]
pub struct ScoringRules {
    pub name: String,
    pub maximum_players_per_team: f64,
    pub timed: bool,
    pub maximum_regulation_tossup_count: f64,
    pub minimum_overtime_question_count: f64,
    pub overtime_includes_bonuses: bool,
    pub maximum_bonus_score: f64,
    pub use_bonuses: bool,
    pub bonus_divisor: f64,
    pub minimum_parts_per_bonus: f64,
    pub maximum_parts_per_bonus: f64,
    pub points_per_bonus_part: Option<f64>,
    pub bonuses_bounce_back: bool,
    pub lightning_count_per_team: f64,
    pub lightning_divisor: f64,
    pub answer_types: Vec<AnswerType>,
    pub bonus_tracking_mode: BonusTrackingMode,
}

impl Default for ScoringRules {
    /// `new ScoringRules()`, which applies the "ACF with powers" rule set
    fn default() -> Self {
        ScoringRules {
            name: String::new(),
            maximum_players_per_team: 4.0,
            timed: false,
            maximum_regulation_tossup_count: 20.0,
            minimum_overtime_question_count: 1.0,
            overtime_includes_bonuses: false,
            maximum_bonus_score: 30.0,
            use_bonuses: true,
            bonus_divisor: 10.0,
            minimum_parts_per_bonus: 3.0,
            maximum_parts_per_bonus: 3.0,
            points_per_bonus_part: Some(10.0),
            bonuses_bounce_back: false,
            lightning_count_per_team: 0.0,
            lightning_divisor: 10.0,
            answer_types: vec![AnswerType::new(15.0), AnswerType::new(10.0), AnswerType::new(-5.0)],
            bonus_tracking_mode: BonusTrackingMode::Automatic,
        }
    }
}

impl ScoringRules {
    pub fn regulation_tossup_count(&self) -> f64 {
        if self.timed {
            20.0
        } else {
            self.maximum_regulation_tossup_count
        }
    }
    pub fn bonuses_are_regular(&self) -> bool {
        self.points_per_bonus_part.is_some() && self.minimum_parts_per_bonus == self.maximum_parts_per_bonus
    }
    pub fn can_calculate_bounce_back_parts_heard(&self) -> bool {
        self.bonuses_are_regular()
    }
    pub fn use_lightning_rounds(&self) -> bool {
        self.lightning_count_per_team > 0.0
    }
    pub fn use_overtime_in_pptuh(&self) -> bool {
        self.overtime_includes_bonuses || !self.use_bonuses
    }
    pub fn set_use_bonuses(&mut self, b: bool) {
        self.use_bonuses = b;
        if !b {
            self.bonuses_bounce_back = false;
            self.overtime_includes_bonuses = false;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub name: String,
    pub year_string: String,
    pub is_ug: bool,
    pub is_d2: bool,
    pub id_number: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Team {
    pub name: String,
    pub letter: String,
    pub is_jv: bool,
    pub is_ug: bool,
    pub is_d2: bool,
    pub players: Vec<PlayerId>,
    /// Team.overallRank.position
    pub overall_rank: Option<f64>,
}

impl Team {
    pub fn id(&self) -> String {
        format!("Team_{}", self.name)
    }
    pub fn truncated_name(&self, length: usize) -> String {
        if js::cp_len(&self.name) <= length {
            return self.name.clone();
        }
        if self.letter.is_empty() {
            return js::trunc(&self.name, length);
        }
        let letter_len = js::cp_len(&self.letter);
        format!("{} {}", js::trunc(&self.name, length.saturating_sub(letter_len + 1)), self.letter)
    }
    /// `getOverallRank() || fallback`
    pub fn overall_rank_or(&self, fallback: f64) -> f64 {
        match self.overall_rank {
            Some(r) if r != 0.0 && !r.is_nan() => r,
            _ => fallback,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Registration {
    pub name: String,
    pub teams: Vec<TeamId>,
    pub is_small_school: bool,
}

#[derive(Clone, Debug)]
pub struct AutoAdvanceRule {
    pub tier: f64,
    pub ranks_that_advance: Vec<f64>,
    pub ranking_rule: String,
}

#[derive(Clone, Debug)]
pub struct PoolTeam {
    pub team: TeamId,
    pub position: Option<f64>,
    pub did_not_advance: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Pool {
    pub name: String,
    pub description: String,
    pub position: f64,
    pub pool_teams: Vec<PoolTeam>,
    pub size: f64,
    pub round_robins: f64,
    pub seeds: Vec<f64>,
    pub has_carryover: bool,
    pub auto_advance_rules: Vec<AutoAdvanceRule>,
}

impl Pool {
    pub fn new(size: f64, position: f64, name: &str, has_carryover: bool) -> Pool {
        Pool {
            name: name.to_string(),
            description: String::new(),
            position,
            pool_teams: vec![],
            size,
            round_robins: 1.0,
            seeds: vec![],
            has_carryover,
            auto_advance_rules: vec![],
        }
    }
    pub fn add_team(&mut self, t: TeamId) {
        self.pool_teams.push(PoolTeam { team: t, position: None, did_not_advance: None });
    }
    pub fn includes_team(&self, t: TeamId) -> bool {
        self.pool_teams.iter().any(|pt| pt.team == t)
    }
    pub fn tier_that_rank_advances_to(&self, rank: f64) -> Option<f64> {
        self.auto_advance_rules.iter().find(|ao| ao.ranks_that_advance.contains(&rank)).map(|ao| ao.tier)
    }
    pub fn seed_for_rank(&self, rank: f64) -> Option<f64> {
        let i = rank - 1.0;
        if i < 0.0 || i.fract() != 0.0 {
            return None;
        }
        self.seeds.get(i as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseType {
    Prelim,
    Playoff,
    Finals,
    Tiebreaker,
}

impl PhaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PhaseType::Prelim => "Prelim",
            PhaseType::Playoff => "Playoff",
            PhaseType::Finals => "Finals",
            PhaseType::Tiebreaker => "Tiebreaker",
        }
    }
    pub fn from_str(s: &str) -> Option<PhaseType> {
        Some(match s {
            "Prelim" => PhaseType::Prelim,
            "Playoff" => PhaseType::Playoff,
            "Finals" => PhaseType::Finals,
            "Tiebreaker" => PhaseType::Tiebreaker,
            _ => return None,
        })
    }
    fn default_name(&self) -> &'static str {
        match self {
            PhaseType::Prelim => "Prelims",
            PhaseType::Playoff => "Playoffs",
            PhaseType::Finals => "Finals",
            PhaseType::Tiebreaker => "Tiebreaker",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WildCardRankingMethod {
    RankThenPpb,
    RecordThenPpb,
}

#[derive(Clone, Debug)]
pub struct WildCardRule {
    pub tier: f64,
    pub number_of_teams: f64,
}

#[derive(Clone, Debug)]
pub struct Round {
    pub number: f64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub packet_name: String,
    pub matches: Vec<Match>,
}

impl Round {
    pub fn new(number: f64, name: Option<String>) -> Round {
        Round { number, name: name.filter(|n| !n.is_empty()), description: None, packet_name: String::new(), matches: vec![] }
    }
    pub fn name(&self) -> String {
        match &self.name {
            Some(n) if !n.is_empty() => n.clone(),
            _ => js::num_to_string(self.number),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Phase {
    pub name: String,
    pub description: String,
    pub phase_type: PhaseType,
    pub code: String,
    pub rounds: Vec<Round>,
    pub pools: Vec<Pool>,
    pub force_numeric_rounds: Option<bool>,
    pub wild_card_rules: Vec<WildCardRule>,
    pub wild_card_ranking_method: WildCardRankingMethod,
}

impl Phase {
    pub fn new(t: PhaseType, first_round: f64, last_round: f64, code: &str, name: Option<&str>) -> Phase {
        let name = match name {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => t.default_name().to_string(),
        };
        let mut rounds = vec![];
        let mut i = first_round;
        while i <= last_round {
            let rn = if t == PhaseType::Tiebreaker || t == PhaseType::Finals { Some(name.clone()) } else { None };
            rounds.push(Round::new(i, rn));
            i += 1.0;
        }
        Phase {
            name,
            description: String::new(),
            phase_type: t,
            code: code.to_string(),
            rounds,
            pools: vec![],
            force_numeric_rounds: None,
            wild_card_rules: vec![],
            wild_card_ranking_method: WildCardRankingMethod::RankThenPpb,
        }
    }
    pub fn is_full_phase(&self) -> bool {
        matches!(self.phase_type, PhaseType::Prelim | PhaseType::Playoff)
    }
    pub fn find_pool_with_team(&self, t: TeamId) -> Option<usize> {
        self.pools.iter().position(|p| p.includes_team(t))
    }
    pub fn has_wild_cards(&self) -> bool {
        !self.wild_card_rules.is_empty()
    }
    pub fn top_wild_card_seed(&self) -> f64 {
        if !self.has_wild_cards() {
            return 0.0;
        }
        let mut n = 0.0;
        for p in &self.pools {
            let mut s = 0.0;
            for r in &p.auto_advance_rules {
                s += r.ranks_that_advance.len() as f64;
            }
            n += s;
        }
        n + 1.0
    }
    pub fn tier_that_wc_seed_advances_to(&self, seed: f64) -> Option<f64> {
        if !self.has_wild_cards() {
            return None;
        }
        let wc_rank = seed - self.top_wild_card_seed() + 1.0;
        let mut count = 0.0;
        for r in &self.wild_card_rules {
            count += r.number_of_teams;
            if count >= wc_rank {
                return Some(r.tier);
            }
        }
        None
    }
    pub fn has_any_carryover(&self) -> bool {
        self.pools.iter().any(|p| p.has_carryover)
    }
    pub fn round_index_by_number(&self, n: f64) -> Option<usize> {
        self.rounds.iter().position(|r| r.number == n)
    }
}

/* ------------------------------ matches ------------------------------ */

#[derive(Clone, Debug)]
pub struct PlayerAnswerCount {
    pub at: AnswerTypeId,
    /// copy of the answer type's value, for convenience
    pub value: f64,
    pub number: Option<f64>,
    pub validation: Msg,
}

impl PlayerAnswerCount {
    pub fn new(at: AnswerTypeId, value: f64, number: Option<f64>) -> Self {
        PlayerAnswerCount { at, value, number, validation: Msg::new(VType::PlayerAnswerCountInvalid) }
    }
    pub fn points(&self) -> f64 {
        match self.number {
            None => 0.0,
            Some(n) => n * self.value,
        }
    }
    pub fn add_to_count(&mut self, n: Option<f64>) {
        let Some(n) = n else { return };
        self.number = Some(match self.number {
            None => n,
            Some(x) => x + n,
        });
    }
    fn number_is_valid(&self) -> bool {
        match self.number {
            None => true,
            Some(n) => 0.0 <= n && n <= 999.0,
        }
    }
    pub fn validate_all(&mut self, label: &str) {
        if !self.number_is_valid() {
            self.validation.set_error(format!("{label}: Number of {}-point buzzes is invalid", js::num_to_string(self.value)));
            return;
        }
        self.validation.set_ok();
    }
    fn errors(&self) -> Vec<String> {
        if self.validation.is_error() {
            vec![self.validation.message.clone()]
        } else {
            vec![]
        }
    }
    fn warnings(&self) -> Vec<String> {
        if self.validation.status == Status::Warning {
            vec![self.validation.message.clone()]
        } else {
            vec![]
        }
    }
}

pub fn answer_counts_for(rules: &ScoringRules) -> Vec<PlayerAnswerCount> {
    rules.answer_types.iter().enumerate().map(|(i, at)| PlayerAnswerCount::new(i, at.value, None)).collect()
}

/// `sortAnswerCounts`: descending point value, stable
pub fn sort_answer_counts(v: &mut Vec<PlayerAnswerCount>) {
    js::sort_by(v, |a, b| b.value - a.value);
}

#[derive(Clone, Debug)]
pub struct MatchPlayer {
    pub player: PlayerId,
    pub tossups_heard: Option<f64>,
    pub answer_counts: Vec<PlayerAnswerCount>,
    pub tuh_validation: Msg,
    pub total_buzzes_validation: Msg,
}

impl MatchPlayer {
    pub fn new(player: PlayerId, rules: Option<&ScoringRules>) -> Self {
        MatchPlayer {
            player,
            tossups_heard: None,
            answer_counts: rules.map(answer_counts_for).unwrap_or_default(),
            tuh_validation: Msg::new(VType::PlayerTuhInvalid),
            total_buzzes_validation: Msg::new(VType::PlayerHasTooManyBuzzes),
        }
    }
    pub fn points(&self) -> f64 {
        let mut t = 0.0;
        for a in &self.answer_counts {
            t += a.points();
        }
        t
    }
    pub fn any_stats_entered(&self) -> bool {
        self.answer_counts.iter().any(|ac| ac.number.is_some())
    }
    pub fn total_buzzes(&self, positive_only: bool) -> f64 {
        let mut t = 0.0;
        for ac in &self.answer_counts {
            if ac.value > 0.0 || !positive_only {
                t += or0(ac.number);
            }
        }
        t
    }
    fn errors(&self) -> Vec<String> {
        let mut e = vec![];
        if self.tuh_validation.is_error() {
            e.push(self.tuh_validation.message.clone());
        }
        if self.total_buzzes_validation.is_error() {
            e.push(self.total_buzzes_validation.message.clone());
        }
        for ac in &self.answer_counts {
            e.extend(ac.errors());
        }
        e
    }
    fn warnings(&self) -> Vec<String> {
        let mut w = vec![];
        if self.tuh_validation.status == Status::Warning {
            w.push(self.tuh_validation.message.clone());
        }
        if self.total_buzzes_validation.status == Status::Warning {
            w.push(self.total_buzzes_validation.message.clone());
        }
        for ac in &self.answer_counts {
            w.extend(ac.warnings());
        }
        w
    }
    fn set_tuh_heard_validation(&mut self, valid: bool, message: Option<&str>, name: &str) {
        if !valid {
            self.tuh_validation
                .set_error(format!("{name}: {}", message.unwrap_or("Value for tossups heard is invalid")));
            return;
        }
        self.tuh_validation.set_ok();
    }
    fn validate_answer_counts(&mut self, name: &str) {
        let mut total = 0.0;
        for ac in &mut self.answer_counts {
            ac.validate_all(name);
            total += or0(ac.number);
        }
        if total > or0(self.tossups_heard) {
            self.total_buzzes_validation.set_error(format!("{name}: Player has more buzzes than tossups heard"));
        } else {
            self.total_buzzes_validation.set_ok();
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchTeam {
    pub team: Option<TeamId>,
    pub forfeit_loss: bool,
    pub points: Option<f64>,
    /// The file had `null` here (old YF wrote that for some games). JS treats null as 0 in arithmetic
    /// and comparisons but not in ===, so `points` holds Some(0.0) and this flag covers the === sites.
    pub points_null: bool,
    pub over_time_buzzes: Vec<PlayerAnswerCount>,
    pub bonus_bounceback_points: Option<f64>,
    pub lightning_points: Option<f64>,
    pub imported_non_bonus_tossups: Option<f64>,
    pub manual_bonuses_heard: Option<f64>,
    pub manual_bonus_points: Option<f64>,
    pub manual_bounceback_parts_heard: Option<f64>,
    pub match_players: Vec<MatchPlayer>,
    pub total_score_field_validation: Msg,
    pub bounceback_field_validation: Msg,
    pub modal_bottom_validation: Collection,
}

/// Names the validation messages need
pub struct Names<'a> {
    pub teams: &'a [Team],
    pub players: &'a [Player],
}

impl MatchTeam {
    pub fn new(team: Option<TeamId>, rules: Option<&ScoringRules>, teams: &[Team]) -> Self {
        let match_players = match team {
            Some(t) => teams[t].players.iter().map(|p| MatchPlayer::new(*p, rules)).collect(),
            None => vec![],
        };
        MatchTeam {
            team,
            forfeit_loss: false,
            points: None,
            points_null: false,
            over_time_buzzes: rules.map(answer_counts_for).unwrap_or_default(),
            bonus_bounceback_points: None,
            lightning_points: None,
            imported_non_bonus_tossups: None,
            manual_bonuses_heard: None,
            manual_bonus_points: None,
            manual_bounceback_parts_heard: None,
            match_players,
            total_score_field_validation: Msg::new(VType::InvalidTeamScore),
            bounceback_field_validation: Msg::new(VType::InvalidBouncebackPoints),
            modal_bottom_validation: Collection::default(),
        }
    }

    pub fn any_player_stats_entered(&self) -> bool {
        self.match_players.iter().any(|mp| mp.any_stats_entered())
    }
    pub fn total_tossups_heard(&self) -> f64 {
        let mut s = 0.0;
        for mp in &self.match_players {
            s += or0(mp.tossups_heard);
        }
        s
    }
    pub fn total_buzzes(&self, positive_only: bool) -> f64 {
        let mut s = 0.0;
        for mp in &self.match_players {
            s += mp.total_buzzes(positive_only);
        }
        s
    }
    pub fn calculate_total_points(&mut self, bonus_points: f64) {
        self.points = Some(self.tossup_points() + bonus_points + or0(self.bonus_bounceback_points) + or0(self.lightning_points));
    }
    pub fn tossup_points(&self) -> f64 {
        let mut t = 0.0;
        for p in &self.match_players {
            t += p.points();
        }
        t
    }
    pub fn bonuses_heard(&self, rules: &ScoringRules) -> f64 {
        if let Some(m) = self.manual_bonuses_heard {
            return m;
        }
        let mut tot = 0.0;
        for mp in &self.match_players {
            tot += mp.total_buzzes(true);
        }
        if rules.overtime_includes_bonuses {
            return tot;
        }
        tot - self.num_overtime_buzzes(true) - nn0(self.imported_non_bonus_tossups)
    }
    pub fn bonus_points(&self) -> f64 {
        if let Some(m) = self.manual_bonus_points {
            return m;
        }
        or0(self.points) - self.tossup_points() - or0(self.bonus_bounceback_points) - or0(self.lightning_points)
    }
    pub fn points_for_ppg(&self, rules: &ScoringRules) -> f64 {
        if rules.use_overtime_in_pptuh() {
            return or0(self.points);
        }
        or0(self.points) - self.overtime_points()
    }
    pub fn correct_tossups_without_bonuses(&self) -> f64 {
        let mut total = 0.0;
        for ac in &self.over_time_buzzes {
            if ac.points() > 0.0 {
                total += or0(ac.number);
            }
        }
        total + nn0(self.imported_non_bonus_tossups)
    }
    pub fn overtime_points(&self) -> f64 {
        let mut t = 0.0;
        for ac in &self.over_time_buzzes {
            t += ac.points();
        }
        t
    }
    pub fn num_overtime_buzzes(&self, positive_only: bool) -> f64 {
        let mut t = 0.0;
        for ac in &self.over_time_buzzes {
            if !positive_only || ac.value > 0.0 {
                t += or0(ac.number);
            }
        }
        t
    }

    fn team_name<'a>(&self, n: &'a Names) -> Option<&'a str> {
        self.team.map(|t| n.teams[t].name.as_str())
    }

    pub fn error_messages(&self, ignore_hidden: bool, n: &Names) -> Vec<String> {
        let mut e = vec![];
        let name = self.team_name(n).unwrap_or("");
        if self.total_score_field_validation.status == Status::Error {
            let label = if name.is_empty() { "Total" } else { name };
            e.push(format!("{label} score: {}", self.total_score_field_validation.message));
        }
        if self.bounceback_field_validation.status == Status::Error {
            e.push(format!("{name} Bounceback points: {}", self.bounceback_field_validation.message));
        }
        e.extend(self.modal_bottom_validation.error_messages(ignore_hidden));
        for mp in &self.match_players {
            e.extend(mp.errors());
        }
        for ac in &self.over_time_buzzes {
            e.extend(ac.errors());
        }
        e
    }

    pub fn warning_messages(&self, n: &Names) -> Vec<String> {
        let mut w = vec![];
        let name = self.team_name(n).unwrap_or("");
        if self.total_score_field_validation.status == Status::Warning {
            let label = if name.is_empty() { "Total" } else { name };
            w.push(format!("{label} score: {}", self.total_score_field_validation.message));
        }
        if self.bounceback_field_validation.status == Status::Warning {
            w.push(format!("{name} Bounceback points: {}", self.bounceback_field_validation.message));
        }
        w.extend(self.modal_bottom_validation.warning_messages());
        for mp in &self.match_players {
            w.extend(mp.warnings());
        }
        for ac in &self.over_time_buzzes {
            w.extend(ac.warnings());
        }
        w
    }

    pub fn add_validation_message(&mut self, t: VType, s: Status, message: &str, n: &Names) {
        let full = match self.team {
            Some(tm) => format!("{}: {message}", n.teams[tm].truncated_name(35)),
            None => message.to_string(),
        };
        self.modal_bottom_validation.add(t, s, full);
    }

    pub fn clear_validation_message(&mut self, t: VType) {
        self.modal_bottom_validation.clear(t);
    }

    pub fn validate_all(&mut self, rules: &ScoringRules, n: &Names) {
        self.validate_total_points(n);
        self.validate_total_and_tu_pts_equal(rules, n);
        self.validate_bounceback_points(rules, n);
        self.validate_answer_counts(n);
        self.validate_bonus_points(rules, n);
        self.validate_overtime_buzzes(n);
        self.validate_total_and_lightning_points(rules, n);
        self.validate_lightning_points(rules, n);
        self.validate_manual_bonus_math(rules, n);
    }

    fn validate_total_points(&mut self, n: &Names) {
        let Some(points) = self.points else {
            self.add_validation_message(VType::MissingTotalPoints, Status::HiddenError, "Total score is required", n);
            self.total_score_field_validation.set_ok();
            return;
        };
        self.modal_bottom_validation.clear(VType::MissingTotalPoints);
        if points < -99999.0 || points > 99999.0 {
            self.total_score_field_validation.set_error("Invalid number".into());
            return;
        }
        self.total_score_field_validation.set_ok();
    }

    fn validate_total_and_tu_pts_equal(&mut self, rules: &ScoringRules, n: &Names) {
        if rules.use_bonuses || rules.use_lightning_rounds() {
            return;
        }
        let Some(points) = self.points else { return };
        if self.points_null || self.tossup_points() != points {
            self.add_validation_message(
                VType::TotalScoreAndTuPtsMismatch,
                Status::Error,
                "Players' points don't add up to total score",
                n,
            );
        } else {
            self.clear_validation_message(VType::TotalScoreAndTuPtsMismatch);
        }
    }

    fn validate_total_and_lightning_points(&mut self, rules: &ScoringRules, n: &Names) {
        if rules.use_bonuses || !rules.use_lightning_rounds() {
            return;
        }
        let Some(points) = self.points else { return };
        if self.points_null || self.tossup_points() + nn0(self.lightning_points) != points {
            self.add_validation_message(
                VType::TuPlusLtngNotEqualTotal,
                Status::Error,
                "Player tossup points plus lightning points should equal total score",
                n,
            );
        } else {
            self.clear_validation_message(VType::TuPlusLtngNotEqualTotal);
        }
    }

    fn validate_lightning_points(&mut self, rules: &ScoringRules, n: &Names) {
        if !rules.use_lightning_rounds() {
            return;
        }
        if nn0(self.lightning_points) % rules.lightning_divisor > 0.0 {
            let msg = format!("Lightning round points aren't divisble by {}", js::num_to_string(rules.lightning_divisor));
            self.add_validation_message(VType::LightningDivisorMismatch, Status::Warning, &msg, n);
        } else {
            self.clear_validation_message(VType::LightningDivisorMismatch);
        }
    }

    fn validate_manual_bonus_math(&mut self, rules: &ScoringRules, n: &Names) {
        let tag = VType::ManualBonusMathMismatch;
        let (Some(manual), Some(points)) = (self.manual_bonus_points, self.points) else {
            self.clear_validation_message(tag);
            return;
        };
        if !rules.use_bonuses || rules.bonus_tracking_mode != BonusTrackingMode::Manual {
            self.clear_validation_message(tag);
            return;
        }
        let tossup_pts = self.tossup_points();
        let bb = nn0(self.bonus_bounceback_points);
        let lt = nn0(self.lightning_points);
        let expected = tossup_pts + manual + bb + lt;
        if expected == points && !self.points_null {
            self.clear_validation_message(tag);
            return;
        }
        let diff = points - expected;
        let diff_str = if diff > 0.0 { format!("+{}", js::num_to_string(diff)) } else { js::num_to_string(diff) };
        let s = js::num_to_string;
        let msg = format!(
            "Score doesn't reconcile: tossups ({}) + bonus ({}) + bounceback ({}) + lightning ({}) = {}, but total is {} (off by {diff_str})",
            s(tossup_pts),
            s(manual),
            s(bb),
            s(lt),
            s(expected),
            if self.points_null { "null".to_string() } else { s(points) }
        );
        self.add_validation_message(tag, Status::Error, &msg, n);
    }

    fn validate_bounceback_points(&mut self, rules: &ScoringRules, n: &Names) {
        let Some(bb) = self.bonus_bounceback_points else {
            self.bounceback_field_validation.set_ok();
            self.clear_validation_message(VType::BouncebackDivisorMismatch);
            return;
        };
        if bb < 0.0 || bb > 9999.0 {
            self.bounceback_field_validation.set_error("Invalid number".into());
            self.clear_validation_message(VType::BouncebackDivisorMismatch);
            return;
        }
        self.bounceback_field_validation.set_ok();
        if bb % rules.bonus_divisor != 0.0 {
            let msg = format!("Bonus bounceback points aren't divisible by {}", js::num_to_string(rules.bonus_divisor));
            self.add_validation_message(VType::BouncebackDivisorMismatch, Status::Warning, &msg, n);
        } else {
            self.clear_validation_message(VType::BouncebackDivisorMismatch);
        }
    }

    fn validate_answer_counts(&mut self, n: &Names) {
        for mp in &mut self.match_players {
            let name = n.players[mp.player].name.clone();
            mp.validate_answer_counts(&name);
        }
    }

    fn validate_bonus_points(&mut self, rules: &ScoringRules, n: &Names) {
        if !rules.use_bonuses {
            return;
        }
        let bonus_points = self.bonus_points();
        let bonuses_heard = self.bonuses_heard(rules);
        let ppb = bonus_points / bonuses_heard;
        let max_ppb = rules.maximum_bonus_score;
        if bonus_points < 0.0 {
            self.add_validation_message(VType::NegativeBonusPoints, Status::Error, "Bonus points cannot be negative", n);
        } else {
            self.clear_validation_message(VType::NegativeBonusPoints);
        }
        if (bonus_points > 0.0 && bonuses_heard == 0.0) || ppb > max_ppb {
            let st = if self.any_player_stats_entered() { Status::Error } else { Status::HiddenError };
            let msg = format!("Points per bonus exceeds the maximum of {}", js::num_to_string(max_ppb));
            self.add_validation_message(VType::BonusPointsTooHigh, st, &msg, n);
        } else {
            self.clear_validation_message(VType::BonusPointsTooHigh);
        }
        if bonus_points % rules.bonus_divisor != 0.0 && bonuses_heard > 0.0 && ppb <= max_ppb {
            let msg = format!("Bonus points are not divisible by {}", js::num_to_string(rules.bonus_divisor));
            self.add_validation_message(VType::BonusDivisorMismatch, Status::Warning, &msg, n);
        } else {
            self.clear_validation_message(VType::BonusDivisorMismatch);
        }
    }

    fn validate_overtime_buzzes(&mut self, n: &Names) {
        let label = format!("{} Overtime tossups", self.team_name(n).unwrap_or(""));
        for ac in &mut self.over_time_buzzes {
            ac.validate_all(&label);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wlt {
    Win,
    Loss,
    Tie,
}

#[derive(Clone, Debug)]
pub struct Match {
    pub id_number: f64,
    pub tossups_read: Option<f64>,
    pub overtime_tossups_read: Option<f64>,
    pub location: Option<String>,
    pub packets: Option<String>,
    pub tiebreaker: bool,
    pub moderator: Option<String>,
    pub scorekeeper: Option<String>,
    pub serial: Option<String>,
    pub notes: Option<String>,
    pub left: MatchTeam,
    pub right: MatchTeam,
    pub carryover_phases: Vec<PhaseId>,
    pub imported_file: Option<String>,
    /// true = StatsValidity.omit
    pub omit: bool,
    pub total_tuh_field_validation: Msg,
    pub overtime_tuh_field_validation: Msg,
    pub modal_bottom_validation: Collection,
}

impl Match {
    pub fn new(id_number: f64, left: MatchTeam, right: MatchTeam) -> Match {
        Match {
            id_number,
            tossups_read: None,
            overtime_tossups_read: None,
            location: None,
            packets: None,
            tiebreaker: false,
            moderator: None,
            scorekeeper: None,
            serial: None,
            notes: None,
            left,
            right,
            carryover_phases: vec![],
            imported_file: None,
            omit: false,
            total_tuh_field_validation: Msg::new(VType::InvalidTotalTuh),
            overtime_tuh_field_validation: Msg::new(VType::InvalidOvertimeTuh),
            modal_bottom_validation: Collection::default(),
        }
    }

    pub fn side(&self, s: Side) -> &MatchTeam {
        match s {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }
    pub fn side_mut(&mut self, s: Side) -> &mut MatchTeam {
        match s {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    pub fn is_forfeit(&self) -> bool {
        self.left.forfeit_loss || self.right.forfeit_loss
    }

    pub fn result(&self, s: Side) -> Option<Wlt> {
        let mt = self.side(s);
        let other = self.side(s.other());
        if mt.forfeit_loss && !other.forfeit_loss {
            return Some(Wlt::Loss);
        }
        if other.forfeit_loss && !mt.forfeit_loss {
            return Some(Wlt::Win);
        }
        if mt.forfeit_loss && other.forfeit_loss {
            return None;
        }
        let (Some(a), Some(b)) = (mt.points, other.points) else { return None };
        if a > b {
            return Some(Wlt::Win);
        }
        if a < b {
            return Some(Wlt::Loss);
        }
        Some(Wlt::Tie)
    }

    pub fn regulation_tuh(&self) -> f64 {
        nn0(self.tossups_read) - nn0(self.overtime_tossups_read)
    }
    pub fn total_tu_converted(&self) -> f64 {
        self.left.total_buzzes(true) + self.right.total_buzzes(true)
    }
    pub fn overtime_tu_converted(&self) -> f64 {
        self.left.correct_tossups_without_bonuses() + self.right.correct_tossups_without_bonuses()
    }
    pub fn regulation_tu_converted(&self) -> f64 {
        self.total_tu_converted() - self.overtime_tu_converted()
    }

    pub fn bounceback_parts_heard(&self, s: Side, rules: &ScoringRules) -> f64 {
        if let Some(m) = self.side(s).manual_bounceback_parts_heard {
            return m;
        }
        if !rules.can_calculate_bounce_back_parts_heard() {
            return f64::NAN;
        }
        let other = self.side(s.other());
        let avail = other.bonuses_heard(rules) * rules.maximum_bonus_score - other.bonus_points();
        avail / or0(rules.points_per_bonus_part).max_if_zero(10.0)
    }

    pub fn bounceback_conv_pct(&self, s: Side, rules: &ScoringRules) -> f64 {
        let heard = self.bounceback_parts_heard(s, rules);
        if heard.is_nan() {
            return f64::NAN;
        }
        let Some(ppbp) = rules.points_per_bonus_part else { return f64::NAN };
        let converted = or0(self.side(s).bonus_bounceback_points) / ppbp;
        js::round((100.0 * converted) / heard)
    }

    pub fn error_messages(&self, ignore_hidden: bool, n: &Names) -> Vec<String> {
        let mut e = vec![];
        if self.total_tuh_field_validation.status == Status::Error {
            e.push(format!("Tossups heard: {}", self.total_tuh_field_validation.message));
        }
        e.extend(self.modal_bottom_validation.error_messages(ignore_hidden));
        e.extend(self.left.error_messages(ignore_hidden, n));
        e.extend(self.right.error_messages(ignore_hidden, n));
        e
    }

    pub fn warning_messages(&self, n: &Names) -> Vec<String> {
        let mut w = vec![];
        if self.total_tuh_field_validation.status == Status::Warning {
            w.push(format!("Tossups heard: {}", self.total_tuh_field_validation.message));
        }
        w.extend(self.modal_bottom_validation.warning_messages());
        w.extend(self.left.warning_messages(n));
        w.extend(self.right.warning_messages(n));
        w
    }

    pub fn is_error_status(&self, n: &Names) -> bool {
        !self.error_messages(false, n).is_empty()
    }

    pub fn validate_all(&mut self, rules: &ScoringRules, n: &Names) {
        self.validate_total_tuh(rules);
        self.validate_teams();
        self.validate_match_teams(rules, n);
        self.validate_total_buzzes(n);
        self.validate_all_match_players_tuh(rules, n);
        self.validate_forfeit();
        self.validate_overtime_tuh_field(rules);
        self.validate_total_and_ot_tuh_relationship(rules);
        self.validate_overtime_buzzes();
        self.validate_overtime_score_math(rules);
        self.validate_bounceback_conversion(rules, n);
    }

    fn validate_total_tuh(&mut self, rules: &ScoringRules) {
        if self.is_forfeit() {
            self.total_tuh_field_validation.set_ok();
            return;
        }
        let Some(tu) = self.tossups_read else {
            self.total_tuh_field_validation.set_error("Field is required".into());
            return;
        };
        if tu < 1.0 || tu > 999.0 {
            self.total_tuh_field_validation.set_error("Invalid number".into());
            return;
        }
        self.total_tuh_field_validation.set_ok();
        if !rules.timed && tu < rules.regulation_tossup_count() {
            self.modal_bottom_validation.add(
                VType::LowTotalTuh,
                Status::Warning,
                format!(
                    "Total tossups heard is less than {}, the standard number for a game",
                    js::num_to_string(rules.regulation_tossup_count())
                ),
            );
            self.modal_bottom_validation.clear(VType::RegulationTuhNotStandard);
            return;
        }
        self.modal_bottom_validation.clear(VType::LowTotalTuh);
    }

    fn validate_teams(&mut self) {
        let (Some(l), Some(r)) = (self.left.team, self.right.team) else {
            self.modal_bottom_validation.add(VType::MissingTeams, Status::HiddenError, "Teams are required".into());
            return;
        };
        self.modal_bottom_validation.clear(VType::MissingTeams);
        if l == r {
            self.modal_bottom_validation.add(VType::TeamPlayingItself, Status::Error, "A team cannot play itself".into());
            return;
        }
        self.modal_bottom_validation.clear(VType::TeamPlayingItself);
    }

    fn validate_match_teams(&mut self, rules: &ScoringRules, n: &Names) {
        if self.is_forfeit() {
            self.left.modal_bottom_validation = Collection::default();
            self.right.modal_bottom_validation = Collection::default();
            return;
        }
        self.left.validate_all(rules, n);
        self.right.validate_all(rules, n);
        let strict_eq = match (self.left.points_null, self.right.points_null) {
            (true, true) => true,
            (false, false) => self.left.points == self.right.points,
            _ => false,
        };
        if self.left.points.is_some() && strict_eq {
            self.modal_bottom_validation.add(VType::TieGame, Status::Warning, "This game is a tie".into());
        } else {
            self.modal_bottom_validation.clear(VType::TieGame);
        }
    }

    fn validate_all_match_players_tuh(&mut self, rules: &ScoringRules, n: &Names) {
        if self.is_forfeit() {
            self.left.clear_validation_message(VType::NoTossupsHeard);
            self.right.clear_validation_message(VType::NoTossupsHeard);
            return;
        }
        let tossups_read = self.tossups_read;
        for mt in [&mut self.left, &mut self.right] {
            for mp in &mut mt.match_players {
                let name = n.players[mp.player].name.clone();
                match (mp.tossups_heard, tossups_read) {
                    (None, _) | (_, None) => mp.set_tuh_heard_validation(true, None, &name),
                    (Some(tuh), Some(_)) if tuh < 0.0 => mp.set_tuh_heard_validation(false, None, &name),
                    (Some(tuh), Some(tr)) if tuh > tr => mp.set_tuh_heard_validation(
                        false,
                        Some("Tossups heard is greater than the total tossups in the game"),
                        &name,
                    ),
                    _ => mp.set_tuh_heard_validation(true, None, &name),
                }
            }
        }
        for mt in [&mut self.left, &mut self.right] {
            if mt.team.is_none() {
                continue;
            }
            let total = mt.total_tossups_heard();
            if total < 1.0 {
                mt.add_validation_message(
                    VType::NoTossupsHeard,
                    Status::Error,
                    "No players have heard any tossups for this team",
                    n,
                );
            } else {
                mt.modal_bottom_validation.clear(VType::NoTossupsHeard);
            }
            let n_players = mt.match_players.len() as f64;
            let expected = if n_players >= rules.maximum_players_per_team {
                rules.maximum_players_per_team * or0(tossups_read)
            } else {
                n_players * or0(tossups_read)
            };
            if tossups_read.is_some() && total > expected {
                let msg = format!("Players have heard more than {} tossups in aggregate", js::num_to_string(expected));
                mt.add_validation_message(VType::MoreThanAllowedTUH, Status::Error, &msg, n);
            } else {
                mt.modal_bottom_validation.clear(VType::MoreThanAllowedTUH);
            }
            if tossups_read.is_some() && total < expected && total > 0.0 {
                let msg = format!("Players have heard fewer than {} tossups in aggregate", js::num_to_string(expected));
                mt.add_validation_message(VType::FewerThanExpectedTUH, Status::Warning, &msg, n);
            } else {
                mt.modal_bottom_validation.clear(VType::FewerThanExpectedTUH);
            }
        }
    }

    fn validate_total_buzzes(&mut self, n: &Names) {
        let total_conv = self.total_tu_converted();
        let tr = self.tossups_read;
        if let Some(t) = tr.filter(|t| *t > 0.0 && total_conv > *t) {
            self.modal_bottom_validation.add(
                VType::MatchHasTooConvertedTU,
                Status::Error,
                format!(
                    "Total number of tossups converted ({}) exceeds the number of tossups in the game ({})",
                    js::num_to_string(total_conv),
                    js::num_to_string(t)
                ),
            );
        } else {
            self.modal_bottom_validation.clear(VType::MatchHasTooConvertedTU);
        }
        for mt in [&mut self.left, &mut self.right] {
            let buzzes = mt.total_buzzes(false);
            if let Some(t) = tr.filter(|t| *t > 0.0 && buzzes > *t) {
                let msg = format!(
                    "Team's total buzzes ({}) exceeds the number of tossups in the game ({})",
                    js::num_to_string(buzzes),
                    js::num_to_string(t)
                );
                mt.add_validation_message(VType::TeamHasTooManyBuzzes, Status::Error, &msg, n);
            } else {
                mt.clear_validation_message(VType::TeamHasTooManyBuzzes);
            }
        }
    }

    fn validate_forfeit(&mut self) {
        if self.left.forfeit_loss && self.right.forfeit_loss {
            self.modal_bottom_validation.add(
                VType::DoubleForfeit,
                Status::Info,
                "This game will be recorded as \"not played\" and won't count as a win, loss, or tie for either team".into(),
            );
        } else {
            self.modal_bottom_validation.clear(VType::DoubleForfeit);
        }
    }

    fn validate_overtime_tuh_field(&mut self, rules: &ScoringRules) {
        let ot = match self.overtime_tossups_read {
            Some(ot) if !self.is_forfeit() && ot != 0.0 => ot,
            _ => {
                self.overtime_tuh_field_validation.set_ok();
                self.modal_bottom_validation.clear(VType::OtTuhLessThanMinimum);
                self.modal_bottom_validation.clear(VType::OvertimeTuhTooHigh);
                return;
            }
        };
        if ot < 0.0 || ot > 999.0 {
            self.overtime_tuh_field_validation.set_error("Overtime tossups read is invalid".into());
            return;
        }
        self.overtime_tuh_field_validation.set_ok();
        if ot < rules.minimum_overtime_question_count {
            self.modal_bottom_validation.add(
                VType::OtTuhLessThanMinimum,
                Status::Warning,
                format!(
                    "Overtime tossups heard is less than the minimum of {}",
                    js::num_to_string(rules.minimum_overtime_question_count)
                ),
            );
        } else {
            self.modal_bottom_validation.clear(VType::OtTuhLessThanMinimum);
        }
        let reg_tuh = self.regulation_tuh();
        let reg_conv = self.regulation_tu_converted();
        if reg_conv > reg_tuh {
            self.modal_bottom_validation.add(
                VType::OvertimeTuhTooHigh,
                Status::Error,
                format!(
                    "Based on overtime stats, {} tossups were answered correctly in regulation, but only {} were read in regulation.",
                    js::num_to_string(reg_conv),
                    js::num_to_string(reg_tuh)
                ),
            );
        } else {
            self.modal_bottom_validation.clear(VType::OvertimeTuhTooHigh);
        }
    }

    fn validate_total_and_ot_tuh_relationship(&mut self, rules: &ScoringRules) {
        let skip = match self.tossups_read {
            None => true,
            Some(t) => (t == 0.0 && self.is_forfeit()) || self.modal_bottom_validation.has(VType::LowTotalTuh),
        };
        if skip {
            self.modal_bottom_validation.clear(VType::RegulationTuhNotStandard);
            return;
        }
        let reg = self.tossups_read.unwrap() - or0(self.overtime_tossups_read);
        if reg > rules.maximum_regulation_tossup_count {
            self.modal_bottom_validation.add(
                VType::RegulationTuhNotStandard,
                Status::Error,
                format!(
                    "Tossups read in regulation is {}, but the maximum allowed is {}",
                    js::num_to_string(reg),
                    js::num_to_string(rules.maximum_regulation_tossup_count)
                ),
            );
            return;
        }
        if !rules.timed && !self.tiebreaker && reg < rules.regulation_tossup_count() {
            self.modal_bottom_validation.add(
                VType::RegulationTuhNotStandard,
                Status::Warning,
                format!(
                    "Tossups read in regulation is {}, which is less than the standard number of {}",
                    js::num_to_string(reg),
                    js::num_to_string(rules.maximum_regulation_tossup_count)
                ),
            );
        } else {
            self.modal_bottom_validation.clear(VType::RegulationTuhNotStandard);
        }
    }

    fn validate_overtime_buzzes(&mut self) {
        let Some(ot) = self.overtime_tossups_read else {
            self.modal_bottom_validation.clear(VType::TotalOtBuzzesExceedsTuh);
            return;
        };
        if self.left.num_overtime_buzzes(true) + self.right.num_overtime_buzzes(true) > ot {
            self.modal_bottom_validation.add(
                VType::TotalOtBuzzesExceedsTuh,
                Status::Error,
                "More tossups were converted than read in overtime".into(),
            );
            return;
        }
        self.modal_bottom_validation.clear(VType::TotalOtBuzzesExceedsTuh);
    }

    fn validate_overtime_score_math(&mut self, rules: &ScoringRules) {
        let skip = self.is_forfeit()
            || self.left.points.is_none()
            || self.right.points.is_none()
            || self.overtime_tossups_read.is_none()
            || self.overtime_tossups_read == Some(0.0)
            || rules.overtime_includes_bonuses;
        if skip {
            self.modal_bottom_validation.clear(VType::OtButRegScoreNotTied);
            return;
        }
        let l = self.left.points.unwrap() - self.left.overtime_points();
        let r = self.right.points.unwrap() - self.right.overtime_points();
        if l != r {
            self.modal_bottom_validation.add(
                VType::OtButRegScoreNotTied,
                Status::Warning,
                "Game went to overtime but the score wasn't tied after regulation, based on each team's overtime stats".into(),
            );
        } else {
            self.modal_bottom_validation.clear(VType::OtButRegScoreNotTied);
        }
    }

    fn validate_bounceback_conversion(&mut self, rules: &ScoringRules, n: &Names) {
        if !rules.bonuses_bounce_back {
            return;
        }
        let lp = self.bounceback_conv_pct(Side::Left, rules);
        let rp = self.bounceback_conv_pct(Side::Right, rules);
        for (mt, pct) in [(&mut self.left, lp), (&mut self.right, rp)] {
            if !pct.is_nan() && pct > 100.0 {
                mt.add_validation_message(
                    VType::BouncebackConvOver100,
                    Status::Error,
                    "Bounceback conversion is greater than 100%",
                    n,
                );
            } else {
                mt.clear_validation_message(VType::BouncebackConvOver100);
            }
        }
    }
}

trait MaxIfZero {
    fn max_if_zero(self, fallback: f64) -> f64;
}
impl MaxIfZero for f64 {
    /// `x || fallback` for an already-unwrapped number
    fn max_if_zero(self, fallback: f64) -> f64 {
        if self == 0.0 || self.is_nan() {
            fallback
        } else {
            self
        }
    }
}
