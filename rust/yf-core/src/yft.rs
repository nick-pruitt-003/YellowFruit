//! Port of the .yft load path: TournamentManager.openYftFile -> earlyYftFileConversions ->
//! snakeCaseToCamelCase -> collectRefTargets -> FileParser.parseYftTournament -> conversions.
//!
//! Instead of rewriting keys, `field()` reads a camelCase name the way the TS sees it after
//! snakeCaseToCamelCase: for the ~60 renamed keys, only the snake_case spelling in the file counts.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::js;
use crate::jsval::{jv, Jv};
use crate::model::*;
use crate::tournament::Tournament;
use crate::validation::{Collection, Status, VType};

/// (camelCase, snake_case) pairs from CaseConversion.ts
const RENAMED: &[(&str, &str)] = &[
    ("shortName", "short_name"),
    ("tournamentSite", "tournament_site"),
    ("scoringRules", "scoring_rules"),
    ("startDate", "start_date"),
    ("endDate", "end_date"),
    ("questionSet", "question_set"),
    ("teamsPerMatch", "teams_per_match"),
    ("maximumPlayersPerTeam", "maximum_players_per_team"),
    ("regulationTossupCount", "regulation_tossup_count"),
    ("maximumRegulationTossupCount", "maximum_regulation_tossup_count"),
    ("minimumOvertimeQuestionCount", "minimum_overtime_question_count"),
    ("overtimeIncludesBonuses", "overtime_includes_bonuses"),
    ("totalDivisor", "total_divisor"),
    ("maximumBonusScore", "maximum_bonus_score"),
    ("bonusDivisor", "bonus_divisor"),
    ("minimumPartsPerBonus", "minimum_parts_per_bonus"),
    ("maximumPartsPerBonus", "maximum_parts_per_bonus"),
    ("pointsPerBonusPart", "points_per_bonus_part"),
    ("bonusesBounceBack", "bonuses_bounce_back"),
    ("lightningCountPerTeam", "lightning_count_per_team"),
    ("maximumLightningScore", "maximum_lightning_score"),
    ("lightningDivisor", "lightning_divisor"),
    ("lightningsBounceBack", "lightnings_bounce_back"),
    ("answerTypes", "answer_types"),
    ("shortLabel", "short_label"),
    ("awardsBonus", "awards_bonus"),
    ("cardsTraded", "cards_traded"),
    ("poolTeams", "pool_teams"),
    ("tossupsRead", "tossups_read"),
    ("overtimeTossupsRead", "overtime_tossups_read"),
    ("matchTeams", "match_teams"),
    ("carryoverPhases", "carryover_phases"),
    ("matchQuestions", "match_questions"),
    ("questionNumber", "question_number"),
    ("bonusPoints", "bonus_points"),
    ("bonusBouncebackPoints", "bonus_bounceback_points"),
    ("lightningPoints", "lightning_points"),
    ("forfeitLoss", "forfeit_loss"),
    ("correctTossupsWithoutBonuses", "correct_tossups_without_bonuses"),
    ("matchPlayers", "match_players"),
    ("suppressFromStatistics", "suppress_from_statistics"),
    ("tossupsHeard", "tossups_heard"),
    ("answerCounts", "answer_counts"),
    ("answerType", "answer_type"),
    ("firstQuestion", "first_question"),
    ("externalId", "external_id"),
    ("categoryGroup", "category_group"),
    ("controlledPoints", "controlled_points"),
    ("bouncebackPoints", "bounceback_points"),
];

const PLAYER_YEAR_ABBREVS: [&str; 19] =
    ["K", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "Fr.", "So.", "Jr.", "Sr.", "Post-Sr.", "Grad."];

fn snake_of(camel: &str) -> Option<&'static str> {
    RENAMED.iter().find(|(c, _)| *c == camel).map(|(_, s)| *s)
}

struct Parser<'a> {
    refs: HashMap<String, &'a Value>,
    t: Tournament,
    teams_by_id: HashMap<String, TeamId>,
    players_by_id: HashMap<String, PlayerId>,
    /// teams skipped for having no players; seeds and pools that point at them drop them too
    skipped_team_ids: HashSet<String>,
    /// players skipped for having no name; their match lines are dropped too
    skipped_player_ids: HashSet<String>,
    phases_by_id: HashMap<String, PhaseId>,
    answer_types_by_id: HashMap<String, AnswerTypeId>,
    warnings: Vec<String>,
    is_naqt: bool,
    /// earlyYftFileConversions: pre-4.0.18 files saved lightningPoints in camelCase
    lightning_fix: bool,
    next_match_id: f64,
}

pub struct YftLoad {
    pub tournament: Tournament,
    pub warnings: Vec<String>,
}

type R<T> = Result<T, String>;

/// Read a property as the TS sees it after snakeCaseToCamelCase
fn field<'v>(o: &'v Value, camel: &str, lightning_fix: bool) -> Jv<'v> {
    let obj = o.as_object();
    if camel == "lightningPoints" && lightning_fix {
        if let Some(v) = obj.and_then(|m| m.get("lightningPoints")) {
            return jv(Some(v));
        }
    }
    match snake_of(camel) {
        Some(snake) => jv(obj.and_then(|m| m.get(snake))),
        None => jv(obj.and_then(|m| m.get(camel))),
    }
}

fn unicode_truncate(s: &str, max: usize) -> String {
    if js::cp_len(s) <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect()
    }
}

fn bad_integer(v: f64, lo: f64, hi: f64) -> bool {
    if v < lo || v > hi {
        return true;
    }
    v % 1.0 > 0.0
}

fn drop_zero(n: Option<f64>) -> Option<f64> {
    if n == Some(0.0) {
        None
    } else {
        n
    }
}

/// Is this value a {$ref} pointer?
fn is_ref(v: &Value) -> bool {
    jv(v.get("$ref")).truthy()
}

fn ref_str(v: &Value) -> String {
    match v.get("$ref") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

impl<'a> Parser<'a> {
    fn f(&self, o: &'a Value, camel: &str) -> Jv<'a> {
        field(o, camel, self.lightning_fix)
    }

    fn num(&self, o: &'a Value, camel: &str) -> R<Option<f64>> {
        self.f(o, camel).opt_num(camel)
    }

    fn yf(&self, o: &'a Value) -> Option<&'a Value> {
        o.get("YfData").filter(|v| jv(Some(v)).truthy())
    }

    /// getBaseQbjObject
    fn base(&self, o: &'a Value) -> R<Option<&'a Value>> {
        if !o.is_object() {
            return Err(format!("Cannot destructure '$ref' of non-object {o}"));
        }
        if !is_ref(o) {
            return Ok(Some(o));
        }
        match self.refs.get(&ref_str(o)) {
            Some(target) if !is_ref(target) => Ok(Some(target)),
            _ => Ok(None),
        }
    }

    /// getYfObjectFromId
    /// Tournament.findTeamById, falling back to the trimmed name: files saved after importing an old YF
    /// file could have IDs like "Team_Alpha " while names are trimmed on load
    fn find_team_by_id(&self, id: &str) -> Option<TeamId> {
        if let Some(t) = self.t.find_team_by_id(id) {
            return Some(t);
        }
        let rest = id.strip_prefix("Team_")?;
        self.t.find_team_by_id(&format!("Team_{}", js::trim(rest)))
    }

    fn lookup<T: Copy>(o: Option<&Value>, dict: &HashMap<String, T>) -> Option<T> {
        let o = o?;
        if !jv(Some(o)).truthy() {
            return None;
        }
        if is_ref(o) {
            return dict.get(&ref_str(o)).copied();
        }
        match o.get("id") {
            Some(Value::String(id)) if !id.is_empty() => dict.get(id).copied(),
            _ => None,
        }
    }

    fn arr(&self, o: &'a Value, camel: &str) -> Option<&'a Vec<Value>> {
        match self.f(o, camel) {
            Jv::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// `for (const obj of ary)` where ary may be undefined
    fn iter_req(&self, o: &'a Value, camel: &str) -> R<&'a Vec<Value>> {
        self.arr(o, camel).ok_or_else(|| "ary is not iterable".to_string())
    }

    fn str_of(&self, o: &'a Value, camel: &str) -> Option<&'a str> {
        self.f(o, camel).as_str()
    }

    /* ---------------- tournament ---------------- */

    fn parse_tournament(&mut self, obj: &'a Value, cur_version: &str) -> R<()> {
        let yf = self.yf(obj);
        let version = yf.and_then(|y| y.get("YfVersion")).and_then(|v| v.as_str()).unwrap_or("");
        if version.is_empty() {
            return Err("Failed to determine the YellowFruit version of this file.".into());
        }
        if js::version_lt(cur_version, version) {
            return Err(format!(
                "This file was made with YellowFruit version {version}. You must upgrade to that version (or later) to open this file."
            ));
        }
        if obj.get("tournament_id").is_some() {
            self.is_naqt = true;
        }
        if let Some(n) = self.str_of(obj, "name") {
            if !n.is_empty() && n != "unnamed tournament" {
                self.t.name = n.to_string();
            }
        }
        if let Jv::Str(s) = self.f(obj, "startDate") {
            self.t.start_date = Some(s.to_string());
        }
        if let Jv::Str(s) = self.f(obj, "endDate") {
            self.t.end_date = Some(s.to_string());
        }
        if let Some(q) = self.str_of(obj, "questionSet").filter(|q| !q.is_empty()) {
            self.t.question_set = q.to_string();
        }
        if let Some(site) = obj.get("tournament_site").filter(|v| jv(Some(v)).truthy()) {
            if let Some(b) = self.base(site)? {
                let n = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if !n.is_empty() && n != "unknown site" {
                    self.t.site_name = n.to_string();
                }
            }
        }
        if let Some(rules) = obj.get("scoring_rules").filter(|v| jv(Some(v)).truthy()) {
            self.t.scoring_rules = self.parse_scoring_rules(rules)?;
        }
        if let Some(regs) = obj.get("registrations").filter(|v| jv(Some(v)).truthy()) {
            let regs = regs.as_array().ok_or("ary is not iterable")?;
            let mut out = vec![];
            for r in regs {
                if let Some(reg) = self.parse_registration(r)? {
                    out.push(reg);
                }
            }
            self.t.registrations = out;
        }
        if let Some(phases) = obj.get("phases").filter(|v| jv(Some(v)).truthy()) {
            let phases = phases.as_array().ok_or("ary is not iterable")?;
            self.t.phases = self.parse_phase_list(phases)?;
        }
        if let Some(yf) = yf {
            self.t.standard_rule_set = yf.get("standardRuleSet").and_then(|v| v.as_str()).map(|s| s.to_string());
            if let Some(seeds) = yf.get("seeds").filter(|v| jv(Some(v)).truthy()) {
                let seeds = seeds.as_array().ok_or("seedsFromFile is not iterable")?;
                let mut out = vec![];
                for ptr in seeds {
                    let id = ref_str(ptr);
                    let team = self.find_team_by_id(&id);
                    if team.is_none() && self.skipped_team_ids.contains(&id) {
                        continue;
                    }
                    let team = team.ok_or(format!("Seed list references team {id}, which doesn't exist."))?;
                    out.push(team);
                }
                self.t.seeds = out;
            }
            let b = |k: &str| jv(yf.get(k)).truthy();
            self.t.track_player_year = match jv(yf.get("trackPlayerYear")) {
                Jv::Undef | Jv::Null => true,
                other => other.truthy(),
            };
            self.t.track_small_school = b("trackSmallSchool");
            self.t.track_jv = b("trackJV");
            self.t.track_ug = b("trackUG");
            self.t.track_div2 = b("trackDiv2");
            self.t.finals_rankings_ready = b("finalRankingsReady");
            self.t.using_schedule_template = b("usingScheduleTemplate");
            self.t.app_version = version.to_string();
        } else {
            self.t.infer_carryover_status();
        }
        self.t.calc_has_match_data();
        Ok(())
    }

    /* ---------------- scoring rules ---------------- */

    fn parse_scoring_rules(&mut self, obj: &'a Value) -> R<ScoringRules> {
        let Some(q) = self.base(obj)? else { return Ok(ScoringRules::default()) };
        let Some(ats) = self.arr(q, "answerTypes") else {
            return Err("Scoring Rules: There are no tossup point values (Answer Types) defined for this tournament.".into());
        };
        if let Some(tpm) = self.num(q, "teamsPerMatch")? {
            if tpm != 0.0 && tpm != 2.0 {
                return Err(format!("YellowFruit doesn't support formates with {} teams per match.", js::num_to_string(tpm)));
            }
        }
        let mut r = ScoringRules { name: self.str_of(q, "name").unwrap_or("").to_string(), ..Default::default() };
        let yf = self.yf(q);

        // game length
        let reg = self.num(q, "regulationTossupCount")?.unwrap_or(20.0);
        let max_reg = self.num(q, "maximumRegulationTossupCount")?.unwrap_or(20.0);
        if !(1.0..=100.0).contains(&max_reg) {
            return Err("Scoring Rules: Unsupported value for maximum regulation toss-up count.".into());
        }
        r.maximum_regulation_tossup_count = max_reg;
        r.timed = match yf {
            Some(y) => jv(y.get("timed")).truthy(),
            None => max_reg != reg,
        };

        // answer types: (original order index, AnswerType, id)
        let mut parsed: Vec<(AnswerType, Option<String>)> = vec![];
        for a in ats {
            let Some(b) = self.base(a)? else { continue };
            let Some(v) = self.num(b, "value")? else {
                return Err("Scoring Rules contain an Answer Type with no point value.".into());
            };
            if bad_integer(v, -1000.0, 1000.0) {
                return Err(format!("Scoring Rules contain an Answer Type with an invalid point value: {}", js::num_to_string(v)));
            }
            if v <= 0.0 && self.f(b, "awardsBonus").truthy() {
                return Err("Answer Types with non-positive point values may not award bonuses.".into());
            }
            let mut at = AnswerType::new(v);
            if let Some(l) = self.str_of(b, "label").filter(|l| !l.is_empty()) {
                at.label = Some(l.to_string());
            }
            if let Some(l) = self.str_of(b, "shortLabel").filter(|l| !l.is_empty()) {
                at.short_label = Some(l.to_string());
            }
            let id = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
            parsed.push((at, id));
        }
        if !parsed.iter().any(|(a, _)| a.value > 0.0) {
            return Err("Scoring Rules: This tournament contains no positive toss-up point values.".into());
        }
        js::sort_by(&mut parsed, |a, b| b.0.value - a.0.value);
        for (i, (_, id)) in parsed.iter().enumerate() {
            if let Some(id) = id {
                self.answer_types_by_id.insert(id.clone(), i);
            }
        }
        r.answer_types = parsed.into_iter().map(|x| x.0).collect();

        // bonuses
        let max_bonus = self.num(q, "maximumBonusScore")?;
        r.use_bonuses = max_bonus.is_some();
        if let Some(max_bonus) = max_bonus {
            if let Some(m) = yf.and_then(|y| y.get("bonusTrackingMode")).and_then(|v| v.as_str()) {
                r.bonus_tracking_mode = match m {
                    "Manual" => BonusTrackingMode::Manual,
                    "ManualHeardAutoPoints" => BonusTrackingMode::ManualHeardAutoPoints,
                    _ => BonusTrackingMode::Automatic,
                };
            }
            if bad_integer(max_bonus, 1.0, 1000.0) {
                return Err(format!("Scoring Rules: Invalid maximum bonus score: {}", js::num_to_string(max_bonus)));
            }
            r.maximum_bonus_score = max_bonus;
            let min_parts = self.num(q, "minimumPartsPerBonus")?.unwrap_or(3.0);
            if bad_integer(min_parts, 1.0, 1000.0) {
                return Err(format!("Scoring Rules: Invalid minimum parts per bonus: {}", js::num_to_string(min_parts)));
            }
            r.minimum_parts_per_bonus = min_parts;
            let max_parts = self.num(q, "maximumPartsPerBonus")?.unwrap_or(3.0);
            if bad_integer(max_parts, 1.0, 1000.0) {
                return Err(format!("Scoring Rules: Invalid maximum parts per bonus: {}", js::num_to_string(max_parts)));
            }
            if max_parts < r.minimum_parts_per_bonus {
                return Err("Scoring Rules: Maximum parts per bonus is less than minimum parts per bonus.".into());
            }
            r.maximum_parts_per_bonus = max_parts;
            let ppbp = self.num(q, "pointsPerBonusPart")?;
            if let Some(p) = ppbp {
                if bad_integer(p, 1.0, 1000.0) {
                    return Err(format!("Scoring Rules: Invalid points per bonus part setting: {}", js::num_to_string(p)));
                }
                if p * r.maximum_parts_per_bonus != r.maximum_bonus_score {
                    return Err("Scoring Rules: Maximum bonus score, maximum parts per bonus, and points per bonus part settings must be consistent.".into());
                }
            }
            r.points_per_bonus_part = ppbp;
            let mut div = self.num(q, "bonusDivisor")?.unwrap_or(10.0);
            if bad_integer(div, 1.0, 1000.0) {
                return Err(format!("Scoring Rules: Invalid bonus divisor setting: {}", js::num_to_string(div)));
            }
            if r.maximum_bonus_score % div != 0.0 {
                div = 1.0;
            } else if let Some(p) = r.points_per_bonus_part.filter(|p| *p != 0.0 && p % div != 0.0) {
                return Err(format!(
                    "Scoring Rules: Points per bonus ({}) and bonus divisor ({}) settings are incompatible.",
                    js::num_to_string(p),
                    js::num_to_string(div)
                ));
            }
            r.bonus_divisor = div;
            r.bonuses_bounce_back = self.f(q, "bonusesBounceBack").truthy();
        }

        // max players
        let mp = self.num(q, "maximumPlayersPerTeam")?.unwrap_or(4.0);
        if bad_integer(mp, 1.0, 30.0) {
            return Err(format!("Scoring Rules: Invalid maximum players per team setting: {}", js::num_to_string(mp)));
        }
        r.maximum_players_per_team = mp;

        // overtime
        let min_ot = self.num(q, "minimumOvertimeQuestionCount")?.unwrap_or(1.0);
        if bad_integer(min_ot, 1.0, 100.0) {
            return Err(format!("Scoring Rules: Invalid mimimum overtime question setting: {}", js::num_to_string(min_ot)));
        }
        r.minimum_overtime_question_count = min_ot;
        r.overtime_includes_bonuses = self.f(q, "overtimeIncludesBonuses").truthy();

        // lightning
        let lc = self.num(q, "lightningCountPerTeam")?.unwrap_or(0.0);
        if bad_integer(lc, 0.0, 10.0) {
            return Err(format!("Scoring Rules: Invalid lightning rounds per team setting: {}", js::num_to_string(lc)));
        }
        r.lightning_count_per_team = if lc > 0.0 { 1.0 } else { 0.0 };
        if lc >= 1.0 {
            if let Some(ld) = self.num(q, "lightningDivisor")? {
                if bad_integer(ld, 0.0, 1000.0) {
                    return Err(format!("Scoring Rules: Invalid lightning round divisor setting: {}", js::num_to_string(ld)));
                }
                r.lightning_divisor = ld;
            }
        }
        Ok(r)
    }

    /* ---------------- registrations, teams, players ---------------- */

    fn parse_registration(&mut self, obj: &'a Value) -> R<Option<Registration>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let name = self.str_of(b, "name").map(js::trim).unwrap_or("");
        if name.is_empty() {
            return Err("This file contains a Registration object with no name.".into());
        }
        let mut reg = Registration { name: unicode_truncate(name, 300), ..Default::default() };
        reg.is_small_school = self.yf(b).is_some_and(|y| jv(y.get("isSmallSchool")).truthy());
        for t in self.iter_req(b, "teams")? {
            if let Some(id) = self.parse_team(t)? {
                reg.teams.push(id);
            }
        }
        Ok(Some(reg))
    }

    fn parse_team(&mut self, obj: &'a Value) -> R<Option<TeamId>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let raw_name = self.str_of(b, "name").unwrap_or("");
        let name = js::trim(raw_name);
        if name.is_empty() {
            return Err("This file contains a Team object with no name".into());
        }
        let name = unicode_truncate(name, 300 + 1 + 20);
        let mut players = vec![];
        for p in self.iter_req(b, "players")? {
            if let Some(pid) = self.parse_player(p, &name)? {
                players.push(pid);
            }
        }
        if players.is_empty() {
            let hint = if self.is_naqt {
                " This file appears to be from NAQT's registration system — update all rosters via the check-in process before generating the QBJ file."
            } else {
                ""
            };
            self.warnings.push(format!("Team \"{raw_name}\" has no players and was skipped.{hint}"));
            if let Some(id) = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                self.skipped_team_ids.insert(id.to_string());
            }
            return Ok(None);
        }
        let mut team = Team { name, players, ..Default::default() };
        if let Some(y) = self.yf(b) {
            team.letter = match y.get("letter") {
                None => String::new(),
                Some(l) => {
                    let s = l.as_str().ok_or("letterFromFile.trim is not a function")?;
                    let tr = js::trim(s);
                    if tr.contains(' ') {
                        return Err(format!("Team {raw_name} has an invalid letter/modifier: {tr}"));
                    }
                    unicode_truncate(tr, 20)
                }
            };
            team.is_jv = jv(y.get("isJV")).truthy();
            team.is_ug = jv(y.get("isUG")).truthy();
            team.is_d2 = jv(y.get("isD2")).truthy();
        }
        // parseTeamOverallRank
        if let Some(ranks) = self.arr(b, "ranks") {
            for r in ranks {
                let Some(rb) = self.base(r)? else { continue };
                if rb.get("ranking").map(ref_str).as_deref() != Some("Ranking_Overall") {
                    continue;
                }
                let pos = self.num(rb, "position")?;
                if let Some(p) = pos.filter(|p| *p != 0.0 && !p.is_nan()) {
                    team.overall_rank = Some(p);
                }
                break;
            }
        }
        self.t.teams.push(team);
        let tid = self.t.teams.len() - 1;
        if let Some(id) = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            self.teams_by_id.insert(id.to_string(), tid);
        }
        Ok(Some(tid))
    }

    fn parse_player(&mut self, obj: &'a Value, team_name: &str) -> R<Option<PlayerId>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let name = js::trim(self.str_of(b, "name").unwrap_or(""));
        if name.is_empty() {
            // files saved after importing an old YF file can have these; skip the player rather than refuse the file
            if let Some(id) = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                self.skipped_player_ids.insert(id.to_string());
                self.warnings.push(format!("Team {team_name} contains a player with no name, who was skipped."));
                return Ok(None);
            }
            return Err(format!("Team {team_name} contains a player with no name."));
        }
        let pid = self.t.new_player(&unicode_truncate(name, 200));
        let yf = self.yf(b);
        let year_str = yf
            .and_then(|y| y.get("yearString"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| unicode_truncate(js::trim(s), 20))
            .filter(|s| !s.is_empty());
        let year_string = match year_str {
            Some(s) => s,
            None => {
                let year = self.num(b, "year")?;
                let pname = self.t.players[pid].name.clone();
                parse_player_year(year).ok_or(format!(
                    "Player {pname} on team {team_name} has an invalid Year attribute (must be between -1 and 18)"
                ))?
            }
        };
        let p = &mut self.t.players[pid];
        p.year_string = year_string;
        p.is_ug = yf.is_some_and(|y| jv(y.get("isUG")).truthy());
        p.is_d2 = yf.is_some_and(|y| jv(y.get("isD2")).truthy());
        if let Some(id) = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            self.players_by_id.insert(id.to_string(), pid);
        }
        Ok(Some(pid))
    }

    /* ---------------- phases, pools, rounds ---------------- */

    fn parse_phase_list(&mut self, ary: &'a [Value]) -> R<Vec<Phase>> {
        let mut phases: Vec<Phase> = vec![];
        let mut co_ids: Vec<Vec<Vec<Vec<String>>>> = vec![]; // per phase, per round, per match
        let mut count = 1;
        let mut last_used = 0.0;
        let mut code = 1;
        for obj in ary {
            let fallback = if count == 1 { PhaseType::Prelim } else { PhaseType::Playoff };
            let idx = phases.len();
            let Some((ph, cos)) = self.parse_phase(obj, fallback, last_used + 1.0, &code.to_string(), idx)? else {
                continue;
            };
            last_used = ph.rounds[ph.rounds.len() - 1].number;
            phases.push(ph);
            co_ids.push(cos);
            count += 1;
            code += 1;
        }
        // parseMatchCarryoverPhasesFinish
        for (pi, ph) in phases.iter_mut().enumerate() {
            for (ri, rd) in ph.rounds.iter_mut().enumerate() {
                for (mi, m) in rd.matches.iter_mut().enumerate() {
                    for id in &co_ids[pi][ri][mi] {
                        if let Some(p) = self.phases_by_id.get(id) {
                            m.carryover_phases.push(*p);
                        }
                    }
                }
            }
        }
        Ok(phases)
    }

    #[allow(clippy::type_complexity)]
    fn parse_phase(
        &mut self,
        obj: &'a Value,
        assumed: PhaseType,
        fallback_round_start: f64,
        fallback_code: &str,
        idx: PhaseId,
    ) -> R<Option<(Phase, Vec<Vec<Vec<String>>>)>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let yf = self.yf(b);
        let name = self.str_of(b, "name").unwrap_or("");
        if name.is_empty() {
            return Err("This file contains a Phase object with no name.".into());
        }
        let ptype = match yf {
            Some(y) => {
                let s = y.get("phaseType").and_then(|v| v.as_str()).unwrap_or("");
                PhaseType::from_str(s).ok_or(format!("unsupported phase type {s:?}"))?
            }
            None => assumed,
        };
        // parsePhaseRounds
        let mut rounds: Vec<(Round, Vec<Vec<String>>)> = vec![];
        if let Some(rs) = self.arr(b, "rounds") {
            let mut fb = fallback_round_start;
            for r in rs {
                if let Some(rd) = self.parse_round(r, fb)? {
                    rounds.push(rd);
                    fb += 1.0;
                }
            }
        }
        js::sort_by(&mut rounds, |a, b| a.0.number - b.0.number);
        if rounds.is_empty() {
            return Err("Cannot read properties of undefined (reading 'number') [phase with no rounds]".into());
        }
        let code = match yf {
            Some(y) => y.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            None => fallback_code.to_string(),
        };
        let first = rounds[0].0.number;
        let last = rounds[rounds.len() - 1].0.number;
        let mut ph = Phase::new(ptype, first, last, &code, Some(name));
        ph.description = self.str_of(b, "description").unwrap_or("").to_string();
        if let Some(y) = yf {
            ph.force_numeric_rounds = y.get("forceNumericRounds").and_then(|v| v.as_bool());
            if y.get("wildCardRankingMethod").and_then(|v| v.as_str()) == Some("RecordThanPPB") {
                ph.wild_card_ranking_method = WildCardRankingMethod::RecordThenPpb;
            }
            if let Some(rules) = y.get("wildCardAdvancementRules").and_then(|v| v.as_array()) {
                for r in rules {
                    ph.wild_card_rules.push(WildCardRule {
                        tier: jv(r.get("tier")).to_number(),
                        number_of_teams: jv(r.get("numberOfTeams")).to_number(),
                    });
                }
            }
        }
        // addRoundsFromFile
        if first > last {
            return Err("addRoundsFromFile: first round was greater than last round".into());
        }
        let mut cos: Vec<Vec<Vec<String>>> = ph.rounds.iter().map(|_| vec![]).collect();
        let mut i = first;
        let mut remaining: Vec<Option<(Round, Vec<Vec<String>>)>> = rounds.into_iter().map(Some).collect();
        while i <= last {
            if let Some(k) = remaining.iter().position(|r| r.as_ref().is_some_and(|r| r.0.number == i)) {
                if let Some(slot) = ph.rounds.iter().position(|r| r.number == i) {
                    let (rd, c) = remaining[k].take().unwrap();
                    ph.rounds[slot] = rd;
                    cos[slot] = c;
                } else {
                    // TS keeps the parsed round in its list; it's just never placed. Nothing to do.
                }
            }
            i += 1.0;
        }
        // parsePhasePools
        if let Some(pools) = self.arr(b, "pools") {
            for p in pools {
                if let Some(pool) = self.parse_pool(p)? {
                    ph.pools.push(pool);
                }
            }
        }
        if let Some(id) = b.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            self.phases_by_id.insert(id.to_string(), idx);
        }
        Ok(Some((ph, cos)))
    }

    fn parse_pool(&mut self, obj: &'a Value) -> R<Option<Pool>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let yf = self.yf(b);
        let name = self.str_of(b, "name").unwrap_or("");
        if name.is_empty() {
            return Err("This file contains a Pool object with no name.".into());
        }
        let mut position = self.num(b, "position")?.unwrap_or(1.0);
        if bad_integer(position, 1.0, 100.0) {
            position = 1.0;
        }
        let pts = self.arr(b, "poolTeams");
        if yf.is_none() && pts.is_none_or(|a| a.is_empty()) {
            return Err(format!("Pool {name} has no defined size."));
        }
        let size = match yf {
            Some(y) => jv(y.get("size")).to_number(),
            None => {
                let n = pts.map(|a| a.len()).unwrap_or(0) as f64;
                if n == 0.0 {
                    99.0
                } else {
                    n
                }
            }
        };
        let mut pool = Pool::new(size, position, name, false);
        if let Some(d) = self.str_of(b, "description").filter(|d| !d.is_empty()) {
            pool.description = d.to_string();
        }
        for pt in pts.into_iter().flatten() {
            let Some(pb) = self.base(pt)? else { continue };
            let team = Self::lookup(pb.get("team"), &self.teams_by_id);
            if team.is_none() && pb.get("team").is_some_and(|o| is_ref(o) && self.skipped_team_ids.contains(&ref_str(o))) {
                continue;
            }
            let team = team.ok_or(format!("Pool {name} contains a PoolTeam that doesn't refer to a valid Team."))?;
            let yfp = self.yf(pb);
            pool.pool_teams.push(PoolTeam {
                team,
                position: self.num(pb, "position")?,
                did_not_advance: yfp.and_then(|y| y.get("didNotAdvance")).and_then(|v| v.as_bool()),
            });
        }
        if let Some(y) = yf {
            pool.round_robins = jv(y.get("roundRobins")).to_number();
            pool.seeds = y.get("seeds").and_then(|v| v.as_array()).map(|a| a.iter().map(|x| jv(Some(x)).to_number()).collect()).unwrap_or_default();
            pool.has_carryover = jv(y.get("hasCarryover")).truthy();
            if let Some(rules) = y.get("autoAdvanceRules").and_then(|v| v.as_array()) {
                for r in rules {
                    pool.auto_advance_rules.push(AutoAdvanceRule {
                        tier: jv(r.get("tier")).to_number(),
                        ranks_that_advance: r
                            .get("ranksThatAdvance")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().map(|x| jv(Some(x)).to_number()).collect())
                            .unwrap_or_default(),
                        ranking_rule: r.get("rankingRule").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    });
                }
            }
        }
        Ok(Some(pool))
    }

    fn parse_round(&mut self, obj: &'a Value, fallback: f64) -> R<Option<(Round, Vec<Vec<String>>)>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let yf = self.yf(b);
        let name = self.str_of(b, "name").unwrap_or("");
        let number = match yf {
            Some(y) => jv(y.get("number")).opt_num("YfData.number")?.ok_or("round YfData has no number")?,
            None => js::parse_float(name),
        };
        if number.is_nan() {
            let mut rd = Round::new(fallback, None);
            rd.name = Some(name.to_string()).filter(|n| !n.is_empty());
            return Ok(Some((rd, vec![])));
        }
        let mut rd = Round::new(number, None);
        if let Some(packets) = self.arr(b, "packets").filter(|a| !a.is_empty()) {
            if let Some(pb) = self.base(&packets[0])? {
                rd.packet_name = self.str_of(pb, "name").unwrap_or("").to_string();
            }
        }
        if let Some(n) = yf.and_then(|y| y.get("nonNumericName")).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            rd.name = Some(n.to_string());
        }
        let mut cos = vec![];
        if let Some(ms) = self.arr(b, "matches") {
            for m in ms {
                if let Some((m, co)) = self.parse_match(m)? {
                    rd.matches.push(m);
                    cos.push(co);
                }
            }
        }
        Ok(Some((rd, cos)))
    }

    /* ---------------- matches ---------------- */

    fn parse_match(&mut self, obj: &'a Value) -> R<Option<(Match, Vec<String>)>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let yf = self.yf(b);
        let mut id_number = self.next_match_id;
        self.next_match_id += 1.0;
        if let Some(id) = b.get("id").and_then(|v| v.as_str()) {
            if let Some(n) = match_id_number(id) {
                id_number = n;
            }
        }
        let mts = self.arr(b, "matchTeams");
        let mts = match mts {
            Some(a) if a.len() >= 2 => a,
            _ => return Err("This file contains a Match object with fewer than two teams.".into()),
        };
        if mts.len() > 2 {
            return Err("This file contains a Match object with more than two teams.".into());
        }
        let mut tossups = self.num(b, "tossupsRead")?;
        if tossups.is_none() && !self.t.scoring_rules.timed {
            tossups = Some(self.t.scoring_rules.regulation_tossup_count());
        }
        let co: Vec<String> = self
            .iter_req(b, "carryoverPhases")?
            .iter()
            .filter(|v| v.is_object() && is_ref(v))
            .map(ref_str)
            .collect();
        let left = self.parse_match_team(&mts[0])?.ok_or("This file contains MatchTeam objects that couldn't be parsed")?;
        let right = self.parse_match_team(&mts[1])?.ok_or("This file contains MatchTeam objects that couldn't be parsed")?;
        let mut m = Match::new(id_number, left, right);
        m.tossups_read = tossups;
        m.overtime_tossups_read = self.num(b, "overtimeTossupsRead")?;
        m.tiebreaker = self.f(b, "tiebreaker").truthy();
        m.location = self.str_of(b, "location").map(|s| s.to_string());
        m.packets = self.str_of(b, "packets").map(|s| s.to_string());
        m.moderator = self.str_of(b, "moderator").map(|s| s.to_string());
        m.scorekeeper = self.str_of(b, "scorekeeper").map(|s| s.to_string());
        m.serial = self.str_of(b, "serial").map(|s| s.to_string());
        m.notes = self.str_of(b, "notes").map(|s| s.to_string());
        if m.is_forfeit() {
            m.tossups_read = None;
        }
        m.modal_bottom_validation = read_validation(yf.and_then(|y| y.get("otherValidation")));
        m.imported_file = yf.and_then(|y| y.get("importedFile")).and_then(|v| v.as_str()).map(|s| s.to_string());
        let names = Names { teams: &self.t.teams, players: &self.t.players };
        m.validate_all(&self.t.scoring_rules, &names);
        m.omit = m.is_error_status(&names);
        Ok(Some((m, co)))
    }

    fn parse_match_team(&mut self, obj: &'a Value) -> R<Option<MatchTeam>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let yf = self.yf(b);
        let team = Self::lookup(b.get("team"), &self.teams_by_id).ok_or("Failed to associate a MatchTeam object with a valid Team")?;
        let mut mt = MatchTeam::new(Some(team), Some(&self.t.scoring_rules), &self.t.teams);
        mt.points = self.num(b, "points")?;
        mt.forfeit_loss = self.f(b, "forfeitLoss").truthy();
        mt.bonus_bounceback_points = self.num(b, "bonusBouncebackPoints")?;
        mt.lightning_points = self.num(b, "lightningPoints")?;
        if let Some(y) = yf {
            mt.manual_bonuses_heard = jv(y.get("manualBonusesHeard")).opt_num("manualBonusesHeard")?;
            mt.manual_bonus_points = jv(y.get("manualBonusPoints")).opt_num("manualBonusPoints")?;
            mt.manual_bounceback_parts_heard = jv(y.get("manualBouncebackPartsHeard")).opt_num("manualBouncebackPartsHeard")?;
        }
        let mut mps = vec![];
        if let Some(arr) = self.arr(b, "matchPlayers") {
            for mpv in arr {
                if let Some(mp) = self.parse_match_player(mpv)? {
                    mps.push(mp);
                }
            }
        }
        mt.match_players = mps;
        if let Some(ots) = yf.and_then(|y| y.get("overTimeBuzzes")).and_then(|v| v.as_array()) {
            for ot in ots {
                if let Some((at, n)) = self.parse_answer_count(ot)? {
                    if let Some(ac) = mt.over_time_buzzes.iter_mut().find(|ac| ac.at == at) {
                        ac.number = n;
                    }
                }
            }
        }
        sort_answer_counts(&mut mt.over_time_buzzes);
        match self.f(b, "correctTossupsWithoutBonuses") {
            Jv::Undef | Jv::Null => {}
            other => {
                let ctwb = other.opt_num("correctTossupsWithoutBonuses")?.unwrap_or(f64::NAN);
                let imported = ctwb - mt.correct_tossups_without_bonuses();
                if imported > 0.0 {
                    mt.imported_non_bonus_tossups = Some(imported);
                }
            }
        }
        if mt.points.is_none() {
            let bp = or0(self.num(b, "bonusPoints")?);
            mt.calculate_total_points(bp);
        }
        mt.modal_bottom_validation = read_validation(yf.and_then(|y| y.get("validation")));
        Ok(Some(mt))
    }

    fn parse_match_player(&mut self, obj: &'a Value) -> R<Option<MatchPlayer>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        let pid = Self::lookup(b.get("player"), &self.players_by_id);
        if pid.is_none() && b.get("player").is_some_and(|o| is_ref(o) && self.skipped_player_ids.contains(&ref_str(o))) {
            let had_buzzes =
                self.arr(b, "answerCounts").is_some_and(|acs| acs.iter().any(|ac| jv(ac.get("number")).truthy()));
            if had_buzzes {
                self.warnings.push("Stats for a player with no name were dropped.".into());
            }
            return Ok(None);
        }
        let pid = pid.ok_or("Failed to associate a MatchPlayer object with a valid Player")?;
        let mut mp = MatchPlayer::new(pid, Some(&self.t.scoring_rules));
        mp.tossups_heard = drop_zero(self.num(b, "tossupsHeard")?);
        for pac in self.iter_req(b, "answerCounts")? {
            if let Some((at, n)) = self.parse_answer_count(pac)? {
                if let Some(ac) = mp.answer_counts.iter_mut().find(|ac| ac.at == at) {
                    ac.number = n;
                }
            }
        }
        sort_answer_counts(&mut mp.answer_counts);
        Ok(Some(mp))
    }

    /// parsePlayerAnswerCount -> (answer type index, count after dropZero)
    fn parse_answer_count(&mut self, obj: &'a Value) -> R<Option<(AnswerTypeId, Option<f64>)>> {
        let Some(b) = self.base(obj)? else { return Ok(None) };
        // fixModaqAnswerType
        let mut at_v = b.get("answer_type");
        if !at_v.is_some_and(|v| jv(Some(v)).truthy()) {
            if let Some(a) = b.get("answer") {
                at_v = Some(a);
            }
        }
        let at = match Self::lookup(at_v, &self.answer_types_by_id) {
            Some(i) => Some(i),
            None => {
                let v = at_v.ok_or("Cannot read properties of undefined (reading 'value')")?;
                let val = jv(v.get("value")).opt_num("answerType.value")?;
                let found = val.and_then(|val| self.t.scoring_rules.answer_types.iter().position(|a| a.value == val));
                if found.is_none() && val == Some(0.0) {
                    return Ok(None);
                }
                found
            }
        };
        let at = at.ok_or("Failed to associate a PlayerAnswerCount object with a valid AnswerType")?;
        let n = drop_zero(self.num(b, "number")?);
        Ok(Some((at, n)))
    }
}

/// tryToSetId: "Match_<n>" or "Match_<n>~..."
fn match_id_number(id: &str) -> Option<f64> {
    let rest = id.strip_prefix("Match_")?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after = &rest[digits.len()..];
    if !(after.is_empty() || after.starts_with('~')) {
        return None;
    }
    Some(js::parse_int(&digits))
}

fn read_validation(v: Option<&Value>) -> Collection {
    let mut c = Collection::default();
    for o in v.and_then(|v| v.as_array()).into_iter().flatten() {
        let type_str = o.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let status = Status::from_num(jv(o.get("status")).to_number());
        let message = o.get("message").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let suppressable = o.get("suppressable").and_then(|v| v.as_bool());
        let is_suppressed = jv(o.get("isSuppressed")).truthy();
        c.add_full(VType::from_str(&type_str), status, message, suppressable, is_suppressed, type_str);
    }
    c
}

fn parse_player_year(year: Option<f64>) -> Option<String> {
    let Some(y) = year else { return Some(String::new()) };
    if !(-1.0..=18.0).contains(&y) {
        return None;
    }
    if y == -1.0 {
        return Some(String::new());
    }
    if y.fract() != 0.0 {
        return None;
    }
    Some(PLAYER_YEAR_ABBREVS[y as usize].to_string())
}

fn collect_ref_targets(objects: &[Value], lf: bool) -> R<HashMap<String, &Value>> {
    let mut d: HashMap<String, &Value> = HashMap::new();
    fn put<'v>(d: &mut HashMap<String, &'v Value>, o: &'v Value) {
        if let Some(Value::String(id)) = o.get("id") {
            if !id.is_empty() {
                d.insert(id.clone(), o);
            }
        }
    }
    fn arr<'v>(o: &'v Value, camel: &str, lf: bool) -> &'v [Value] {
        match field(o, camel, lf) {
            Jv::Arr(a) => a,
            _ => &[],
        }
    }
    fn rules<'v>(d: &mut HashMap<String, &'v Value>, r: &'v Value, lf: bool) {
        for a in arr(r, "answerTypes", lf) {
            put(d, a);
        }
    }
    fn team<'v>(d: &mut HashMap<String, &'v Value>, t: &'v Value, lf: bool) {
        for p in arr(t, "players", lf) {
            put(d, p);
        }
        for r in arr(t, "ranks", lf) {
            put(d, r);
            if let Some(rk) = r.get("ranking") {
                put(d, rk);
            }
        }
    }
    fn mtch<'v>(d: &mut HashMap<String, &'v Value>, m: &'v Value, lf: bool) -> R<()> {
        for mt in arr(m, "matchTeams", lf) {
            put(d, mt);
            for mp in arr(mt, "matchPlayers", lf) {
                put(d, mp);
                let Jv::Arr(acs) = field(mp, "answerCounts", lf) else {
                    return Err("matchPlayer.answerCounts is not iterable".into());
                };
                for ac in acs {
                    put(d, ac);
                }
            }
        }
        Ok(())
    }
    fn phase<'v>(d: &mut HashMap<String, &'v Value>, p: &'v Value, lf: bool) -> R<()> {
        for rd in arr(p, "rounds", lf) {
            put(d, rd);
            for m in arr(rd, "matches", lf) {
                put(d, m);
                mtch(d, m, lf)?;
            }
        }
        for pool in arr(p, "pools", lf) {
            put(d, pool);
            for pt in arr(pool, "poolTeams", lf) {
                put(d, pt);
            }
        }
        Ok(())
    }
    for obj in objects {
        put(&mut d, obj);
        let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "Tournament" => {
                if let Some(site) = obj.get("tournament_site") {
                    put(&mut d, site);
                }
                if let Some(r) = obj.get("scoring_rules").filter(|v| v.is_object()) {
                    rules(&mut d, r, lf);
                    put(&mut d, r);
                }
                for reg in arr(obj, "registrations", lf) {
                    put(&mut d, reg);
                    for t in arr(reg, "teams", lf) {
                        put(&mut d, t);
                        team(&mut d, t, lf);
                    }
                }
                for p in arr(obj, "phases", lf) {
                    put(&mut d, p);
                    phase(&mut d, p, lf)?;
                }
                for rk in arr(obj, "rankings", lf) {
                    put(&mut d, rk);
                }
            }
            "ScoringRules" => rules(&mut d, obj, lf),
            "Registration" => {
                for t in arr(obj, "teams", lf) {
                    put(&mut d, t);
                    team(&mut d, t, lf);
                }
            }
            "Team" => team(&mut d, obj, lf),
            "Rank" => {
                if let Some(rk) = obj.get("ranking") {
                    put(&mut d, rk);
                }
            }
            "Phase" => phase(&mut d, obj, lf)?,
            "Round" => {
                for m in arr(obj, "matches", lf) {
                    put(&mut d, m);
                    mtch(&mut d, m, lf)?;
                }
            }
            "Pool" => {
                for pt in arr(obj, "poolTeams", lf) {
                    put(&mut d, pt);
                }
            }
            "Match" => mtch(&mut d, obj, lf)?,
            "MatchTeam" => {
                for mp in arr(obj, "matchPlayers", lf) {
                    put(&mut d, mp);
                }
            }
            _ => {}
        }
    }
    Ok(d)
}

/// Load a YF4 .yft file. `cur_version` is the app version the file must not be newer than.
pub fn load_yft(contents: &str, cur_version: &str) -> R<YftLoad> {
    let root: Value = serde_json::from_str(contents).map_err(|e| format!("This file does not contain valid JSON. ({e})"))?;
    if root.get("version").and_then(|v| v.as_str()) != Some("2.1.1") {
        return Err("unsupported qbj version".into());
    }
    let objects = match root.get("objects") {
        Some(Value::Array(a)) => a,
        _ => return Err("This file doesn't contain any tournament schema objects.".into()),
    };
    let tourn_obj = objects
        .iter()
        .find(|o| o.get("type").and_then(|v| v.as_str()) == Some("Tournament"))
        .ok_or("no Tournament object")?;
    let file_version = tourn_obj.get("YfData").and_then(|y| y.get("YfVersion")).and_then(|v| v.as_str());
    let lightning_fix = file_version.is_some_and(|v| !v.is_empty() && js::version_lt(v, "4.0.18"));
    let refs = collect_ref_targets(objects, lightning_fix)?;
    let mut p = Parser {
        refs,
        t: Tournament::new(),
        teams_by_id: HashMap::new(),
        players_by_id: HashMap::new(),
        skipped_team_ids: HashSet::new(),
        skipped_player_ids: HashSet::new(),
        phases_by_id: HashMap::new(),
        answer_types_by_id: HashMap::new(),
        warnings: vec![],
        is_naqt: false,
        lightning_fix,
        next_match_id: 1000.0,
    };
    p.parse_tournament(tourn_obj, cur_version)?;
    let mut t = p.t;
    t.conversions();
    Ok(YftLoad { tournament: t, warnings: p.warnings })
}
