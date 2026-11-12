//! .yft writer: what File > Save writes (`TournamentManager.generateWholeFileObj` + `makeJSON`).
//!
//! Byte-for-byte parity with the TS app needs three things the obvious serde approach gets wrong:
//! - Key order. Each `toFileObject` builds `{ YfData, ...qbjObject }`, then `camelCaseToSnakeCase`
//!   moves every renamed key to the end of its object in the order of its rename list. So the objects
//!   here are built with camelCase keys in TS order and [`camel_to_snake`] replays that pass.
//! - Numbers. `JSON.stringify` prints `5`, not `5.0`, and writes NaN and Infinity as `null`.
//! - `undefined` fields are left out; `null` ones are kept.

use serde_json::{Map, Value};

use crate::js;
use crate::model::*;
use crate::tournament::Tournament;
use crate::validation::Collection;

/// `camelCaseToSnakeCase` assignments, in source order (duplicates included), from CaseConversion.ts
const CAMEL_TO_SNAKE: &[(&str, &str)] = &[
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
    ("tossupQuestion", "tossup_question"),
    ("tossupStartTime", "tossup_start_time"),
    ("replacementTossupQuestion", "replacement_tossup_question"),
    ("replacementTossupStartTime", "replacement_tossup_start_time"),
    ("replacementTossupQuestionMatchTeams", "replacement_tossup_question_match_teams"),
    ("bonusPoints", "bonus_points"),
    ("bonusBouncebackPoints", "bonus_bounceback_points"),
    ("lightningPoints", "lightning_points"),
    ("bonusStartTime", "bonus_start_time"),
    ("replacementBonus", "replacement_bonus"),
    ("buzzPosition", "buzz_position"),
    ("wordIndex", "word_index"),
    ("clueIndex", "clue_index"),
    ("eligibleMatchTeams", "eligible_match_teams"),
    ("controlledPoints", "controlled_points"),
    ("bouncebackPoints", "bounceback_points"),
    ("forfeitLoss", "forfeit_loss"),
    ("bonusPoints", "bonus_points"),
    ("correctTossupsWithoutBonuses", "correct_tossups_without_bonuses"),
    ("bonusBouncebackPoints", "bonus_bounceback_points"),
    ("matchPlayers", "match_players"),
    ("suppressFromStatistics", "suppress_from_statistics"),
    ("tossupsHeard", "tossups_heard"),
    ("answerCounts", "answer_counts"),
    ("answerType", "answer_type"),
    ("firstQuestion", "first_question"),
    ("questionNumber", "question_number"),
    ("externalId", "external_id"),
    ("categoryGroup", "category_group"),
];

/// The QBJ schema version YF writes
const QBJ_VERSION: &str = "2.1.1";

/// Serialize the tournament the way File > Save does. Fails only on data the TS app would write
/// differently in a way this port doesn't reproduce yet (non-ISO dates).
pub fn save_yft(t: &Tournament) -> Result<String, String> {
    let mut whole = Obj::new();
    whole.set("version", Value::String(QBJ_VERSION.into()));
    whole.set("objects", Value::Array(vec![tournament(t)?]));
    let mut v = whole.done();
    camel_to_snake(&mut v);
    let mut out = String::new();
    write_json(&v, &mut out);
    Ok(out)
}

/* ------------------------------ object builders ------------------------------ */

/// An object under construction. Leaving a key out is how `undefined` is represented.
struct Obj(Map<String, Value>);

impl Obj {
    fn new() -> Obj {
        Obj(Map::new())
    }
    fn set(&mut self, k: &str, v: Value) -> &mut Self {
        self.0.insert(k.to_string(), v);
        self
    }
    fn opt(&mut self, k: &str, v: Option<Value>) -> &mut Self {
        if let Some(v) = v {
            self.0.insert(k.to_string(), v);
        }
        self
    }
    /// `{ YfData: yfData, ...this }`
    fn with_yf_data(self, yf: Obj) -> Obj {
        let mut o = Obj::new();
        o.set("YfData", yf.done());
        for (k, v) in self.0 {
            o.0.insert(k, v);
        }
        o
    }
    fn done(self) -> Value {
        Value::Object(self.0)
    }
}

/// A JS number. NaN and ±Infinity become `null`, as JSON.stringify writes them.
fn num(x: f64) -> Value {
    serde_json::Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null)
}
fn opt_num(x: Option<f64>) -> Option<Value> {
    x.map(num)
}
fn s(x: &str) -> Value {
    Value::String(x.to_string())
}
fn opt_s(x: &Option<String>) -> Option<Value> {
    x.as_deref().map(s)
}
/// `str || undefined`
fn nonempty(x: &str) -> Option<Value> {
    (!x.is_empty()).then(|| s(x))
}
fn b(x: bool) -> Value {
    Value::Bool(x)
}
fn ref_ptr(id: String) -> Value {
    let mut o = Obj::new();
    o.set("$ref", Value::String(id));
    o.done()
}
/// JS `x || 0` for a number
fn or_zero(x: Option<f64>) -> f64 {
    match x {
        Some(n) if n != 0.0 && !n.is_nan() => n,
        _ => 0.0,
    }
}
/// JS truthiness of `a % b`
fn mod_truthy(a: f64, m: f64) -> bool {
    let r = a % m;
    r != 0.0 && !r.is_nan()
}

/* ------------------------------ model objects ------------------------------ */

fn tournament(t: &Tournament) -> Result<Value, String> {
    let mut q = Obj::new();
    q.set("name", s(if t.name.is_empty() { "unnamed tournament" } else { &t.name }));
    q.opt("startDate", date(&t.start_date)?);
    q.opt("endDate", date(&t.end_date)?);
    q.opt("questionSet", nonempty(&t.question_set));
    q.set("registrations", Value::Array(t.registrations.iter().map(|r| registration(t, r)).collect()));
    q.set("phases", Value::Array(t.phases.iter().map(|ph| phase(t, ph)).collect()));
    let mut overall = Obj::new();
    overall.set("name", s(OVERALL_RANKING)).set("id", s(&format!("Ranking_{OVERALL_RANKING}")));
    q.set("rankings", Value::Array(vec![overall.done()]));
    q.set("type", s("Tournament"));
    if !t.site_name.is_empty() {
        let mut site = Obj::new();
        site.set("name", s(&t.site_name));
        q.set("tournamentSite", site.done());
    }
    q.set("scoringRules", scoring_rules(&t.scoring_rules));

    let mut yf = Obj::new();
    yf.set("YfVersion", s(&t.app_version));
    yf.opt("standardRuleSet", opt_s(&t.standard_rule_set));
    yf.set("seeds", Value::Array(t.seeds.iter().map(|tm| ref_ptr(t.teams[*tm].id())).collect()));
    yf.set("trackPlayerYear", b(t.track_player_year));
    yf.set("trackSmallSchool", b(t.track_small_school));
    yf.set("trackJV", b(t.track_jv));
    yf.set("trackUG", b(t.track_ug));
    yf.set("trackDiv2", b(t.track_div2));
    yf.set("finalRankingsReady", b(t.finals_rankings_ready));
    yf.set("usingScheduleTemplate", b(t.using_schedule_template));
    Ok(q.with_yf_data(yf).done())
}

const OVERALL_RANKING: &str = "Overall";

/// What a date read from the file becomes on save: the reviver does `dayjs(value).toDate()` on load,
/// and the replacer writes `dayjs(value).toISOString()`.
///
/// dayjs reads a string that doesn't end in `Z` as a *local* time, so `"2026-04-17"` re-saves as local
/// midnight in UTC (`"2026-04-17T04:00:00.000Z"` on a Mac in US Eastern time). This uses the
/// machine's time zone the same way. Strings ending in `Z` go through `new Date()`; only the
/// `toISOString()` form YF writes is handled, which round-trips unchanged.
fn date(raw: &Option<String>) -> Result<Option<Value>, String> {
    let Some(d) = raw else { return Ok(None) };
    let unsupported = || format!("can't reproduce how YF re-saves the date {d:?}");
    if d.ends_with('Z') || d.ends_with('z') {
        return if is_iso_string(d) { Ok(Some(s(d))) } else { Err(unsupported()) };
    }
    let utc = dayjs_local(d).ok_or_else(unsupported)?;
    Ok(Some(s(&utc.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())))
}

/// `Date.prototype.toISOString` output for years 0000-9999
fn is_iso_string(d: &str) -> bool {
    let by = d.as_bytes();
    by.len() == 24
        && by.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            19 => *c == b'.',
            23 => *c == b'Z',
            _ => c.is_ascii_digit(),
        })
}

/// dayjs's `parseDate` for a string without a trailing `Z`: REGEX_PARSE, then
/// `new Date(y, M - 1, d || 1, h || 0, m || 0, s || 0, ms)` in local time
fn dayjs_local(d: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{Duration, Local, LocalResult, Months, NaiveDate, TimeZone};
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(
            r"^(\d{4})[-/]?(\d{1,2})?[-/]?(\d{0,2})[Tt\s]*(\d{1,2})?:?(\d{1,2})?:?(\d{1,2})?[.:]?(\d+)?$",
        )
        .expect("valid regex")
    });
    let c = re.captures(d)?;
    let n = |i: usize| c.get(i).map(|m| m.as_str()).filter(|x| !x.is_empty()).map(|x| x.parse::<i64>().unwrap_or(0));
    let mut year = n(1)?;
    // new Date(y, ...) maps years 0-99 to 1900-1999
    if (0..=99).contains(&year) {
        year += 1900;
    }
    let month0 = n(2).map(|m| m - 1).unwrap_or(0);
    let day = n(3).filter(|d| *d != 0).unwrap_or(1);
    let ms = c.get(7).map(|m| m.as_str()[..m.as_str().len().min(3)].parse::<i64>().unwrap_or(0)).unwrap_or(0);
    // MakeDay/MakeTime: out-of-range parts carry over, as in the JS Date constructor
    let base = NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, 1, 1)?;
    let base = if month0 >= 0 {
        base.checked_add_months(Months::new(u32::try_from(month0).ok()?))?
    } else {
        base.checked_sub_months(Months::new(u32::try_from(-month0).ok()?))?
    };
    let naive = base.and_hms_opt(0, 0, 0)?
        + Duration::days(day - 1)
        + Duration::hours(n(4).unwrap_or(0))
        + Duration::minutes(n(5).unwrap_or(0))
        + Duration::seconds(n(6).unwrap_or(0))
        + Duration::milliseconds(ms);
    let local = match Local.from_local_datetime(&naive) {
        LocalResult::Single(t) => t,
        // a repeated hour: JS takes the earlier one
        LocalResult::Ambiguous(earlier, _) => earlier,
        // a skipped hour: JS uses the offset from before the change, which moves the time forward
        LocalResult::None => {
            let before = Local.from_local_datetime(&(naive - Duration::days(1))).earliest()?;
            return Some((naive - Duration::seconds(i64::from(before.offset().local_minus_utc()))).and_utc());
        }
    };
    Some(local.with_timezone(&chrono::Utc))
}

fn registration(t: &Tournament, r: &Registration) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&r.name));
    q.set("teams", Value::Array(r.teams.iter().map(|tm| team(t, &t.teams[*tm])).collect()));
    let mut yf = Obj::new();
    yf.set("isSmallSchool", b(r.is_small_school));
    q.with_yf_data(yf).done()
}

fn team(t: &Tournament, tm: &Team) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&tm.name));
    q.set("players", Value::Array(tm.players.iter().map(|p| player(&t.players[*p])).collect()));
    let mut rank = Obj::new();
    rank.set("ranking", ref_ptr(format!("Ranking_{OVERALL_RANKING}")));
    rank.opt("position", opt_num(tm.overall_rank));
    q.set("ranks", Value::Array(vec![rank.done()]));
    q.set("id", s(&tm.id()));
    let mut yf = Obj::new();
    yf.set("letter", s(&tm.letter)).set("isJV", b(tm.is_jv)).set("isUG", b(tm.is_ug)).set("isD2", b(tm.is_d2));
    q.with_yf_data(yf).done()
}

fn player(p: &Player) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&p.name));
    q.opt("year", player_year(&p.year_string).map(num));
    q.set("id", s(&player_id(p)));
    let mut yf = Obj::new();
    yf.set("yearString", s(&p.year_string)).set("isUG", b(p.is_ug)).set("isD2", b(p.is_d2));
    q.with_yf_data(yf).done()
}

pub fn player_id(p: &Player) -> String {
    format!("Player_{}_{}", p.name, p.id_number)
}

/// `Player.year`: the QBJ numeric year (-1 to 18) for the free-text year string
pub fn player_year(year_string: &str) -> Option<f64> {
    if year_string.is_empty() {
        return None;
    }
    let grade = parse_non_numeric_year(year_string).unwrap_or_else(|| js::parse_int(year_string));
    if grade.is_nan() || grade.fract() != 0.0 || !(-1.0..=18.0).contains(&grade) {
        return None;
    }
    Some(grade)
}

/// `Player.parseNonNumericYear`
fn parse_non_numeric_year(text: &str) -> Option<f64> {
    let lc = text.to_lowercase();
    // /^\d+\s*(sr|sen)\b/: "5Sr." etc. is a post-senior
    let rest = lc.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() < lc.len() {
        let rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        for w in ["sen", "sr"] {
            if let Some(after) = rest.strip_prefix(w) {
                let boundary = !after.chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
                if boundary {
                    return Some(17.0);
                }
            }
        }
    }
    const SEARCH: &[(f64, &[&str])] = &[
        (13.0, &["fr"]),
        (14.0, &["so"]),
        (15.0, &["ju", "jr"]),
        (16.0, &["sen", "sr"]),
        (17.0, &["post-s", "post s", "5s", "5 s", "5th y", "5th-y", "5ths", "5th s"]),
        (18.0, &["grad"]),
    ];
    SEARCH.iter().find(|(_, pre)| pre.iter().any(|p| lc.starts_with(p))).map(|(yr, _)| *yr)
}

fn scoring_rules(r: &ScoringRules) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&r.name));
    q.set("answerTypes", Value::Array(r.answer_types.iter().map(answer_type).collect()));
    q.set("maximumRegulationTossupCount", num(r.maximum_regulation_tossup_count));
    q.set("maximumPlayersPerTeam", num(r.maximum_players_per_team));
    q.set("minimumOvertimeQuestionCount", num(r.minimum_overtime_question_count));
    q.set("overtimeIncludesBonuses", b(r.overtime_includes_bonuses));
    q.set("lightningCountPerTeam", num(r.lightning_count_per_team));
    q.set("totalDivisor", num(total_divisor(r)));
    if r.use_bonuses {
        q.set("bonusesBounceBack", b(r.bonuses_bounce_back));
        q.set("maximumBonusScore", num(r.maximum_bonus_score));
        q.set("minimumPartsPerBonus", num(r.minimum_parts_per_bonus));
        q.set("maximumPartsPerBonus", num(r.maximum_parts_per_bonus));
        q.opt("pointsPerBonusPart", opt_num(r.points_per_bonus_part));
        q.set("bonusDivisor", num(r.bonus_divisor));
    }
    if r.use_lightning_rounds() {
        q.set("lightningDivisor", num(r.lightning_divisor));
    }
    let mut yf = Obj::new();
    yf.set("timed", b(r.timed));
    yf.opt(
        "bonusTrackingMode",
        match r.bonus_tracking_mode {
            BonusTrackingMode::Automatic => None,
            BonusTrackingMode::ManualHeardAutoPoints => Some(s("ManualHeardAutoPoints")),
            BonusTrackingMode::Manual => Some(s("Manual")),
        },
    );
    q.with_yf_data(yf).done()
}

/// `ScoringRules.totalDivisor`
fn total_divisor(r: &ScoringRules) -> f64 {
    let mut divisor = 10.0;
    for at in &r.answer_types {
        if mod_truthy(at.value, 5.0) {
            return 1.0;
        }
        if mod_truthy(at.value, 10.0) {
            divisor = 5.0;
        }
    }
    if mod_truthy(r.bonus_divisor, 5.0) {
        return 1.0;
    }
    if mod_truthy(r.bonus_divisor, 10.0) {
        divisor = 5.0;
    }
    if r.lightning_count_per_team > 0.0 {
        if mod_truthy(r.lightning_divisor, 5.0) {
            return 1.0;
        }
        if mod_truthy(r.lightning_divisor, 10.0) {
            divisor = 5.0;
        }
    }
    divisor
}

fn answer_type(at: &AnswerType) -> Value {
    let mut q = Obj::new();
    q.set("value", num(at.value));
    q.opt("label", at.label.as_deref().and_then(nonempty));
    q.opt("shortLabel", at.short_label.as_deref().and_then(nonempty));
    q.set("id", s(&at.id()));
    q.done()
}

fn phase(t: &Tournament, ph: &Phase) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&ph.name));
    q.opt("description", nonempty(&ph.description));
    q.set("rounds", Value::Array(ph.rounds.iter().map(|rd| round(t, rd)).collect()));
    q.set("pools", Value::Array(ph.pools.iter().map(|p| pool(t, p)).collect()));
    q.set("id", s(&phase_id(ph)));
    let mut yf = Obj::new();
    yf.set("phaseType", s(ph.phase_type.as_str()));
    yf.set("code", s(&ph.code));
    yf.set(
        "wildCardAdvancementRules",
        Value::Array(
            ph.wild_card_rules
                .iter()
                .map(|w| {
                    let mut o = Obj::new();
                    o.set("tier", num(w.tier)).set("numberOfTeams", num(w.number_of_teams));
                    o.done()
                })
                .collect(),
        ),
    );
    yf.set(
        "wildCardRankingMethod",
        s(match ph.wild_card_ranking_method {
            WildCardRankingMethod::RankThenPpb => "RankThenPPB",
            WildCardRankingMethod::RecordThenPpb => "RecordThanPPB",
        }),
    );
    yf.opt("forceNumericRounds", ph.force_numeric_rounds.map(b));
    q.with_yf_data(yf).done()
}

fn phase_id(ph: &Phase) -> String {
    format!("Phase_{}", ph.name)
}

fn pool(t: &Tournament, p: &Pool) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&p.name));
    q.opt("description", nonempty(&p.description));
    q.set("position", num(p.position));
    q.set(
        "poolTeams",
        Value::Array(
            p.pool_teams
                .iter()
                .map(|pt| {
                    let mut o = Obj::new();
                    o.set("team", ref_ptr(t.teams[pt.team].id()));
                    o.opt("position", opt_num(pt.position));
                    let mut yf = Obj::new();
                    yf.opt("didNotAdvance", pt.did_not_advance.map(b));
                    o.with_yf_data(yf).done()
                })
                .collect(),
        ),
    );
    let mut yf = Obj::new();
    yf.set("size", num(p.size));
    yf.set("roundRobins", num(p.round_robins));
    yf.set("seeds", Value::Array(p.seeds.iter().map(|x| num(*x)).collect()));
    yf.set("hasCarryover", b(p.has_carryover));
    yf.set(
        "autoAdvanceRules",
        Value::Array(
            p.auto_advance_rules
                .iter()
                .map(|r| {
                    let mut o = Obj::new();
                    o.set("tier", num(r.tier));
                    o.set("ranksThatAdvance", Value::Array(r.ranks_that_advance.iter().map(|x| num(*x)).collect()));
                    o.set("rankingRule", s(&r.ranking_rule));
                    o.done()
                })
                .collect(),
        ),
    );
    q.with_yf_data(yf).done()
}

fn round(t: &Tournament, rd: &Round) -> Value {
    let mut q = Obj::new();
    q.set("name", s(&rd.name()));
    q.set("matches", Value::Array(rd.matches.iter().map(|m| match_obj(t, m)).collect()));
    if !rd.packet_name.is_empty() {
        let mut pk = Obj::new();
        pk.set("name", s(&rd.packet_name));
        q.set("packets", Value::Array(vec![pk.done()]));
    }
    let mut yf = Obj::new();
    yf.set("number", num(rd.number));
    yf.opt("nonNumericName", opt_s(&rd.name));
    q.with_yf_data(yf).done()
}

fn match_obj(t: &Tournament, m: &Match) -> Value {
    let mut q = Obj::new();
    q.set("matchTeams", Value::Array(vec![match_team(t, &m.left), match_team(t, &m.right)]));
    q.set(
        "carryoverPhases",
        Value::Array(m.carryover_phases.iter().map(|p| ref_ptr(phase_id(&t.phases[*p]))).collect()),
    );
    q.opt("tossupsRead", opt_num(m.tossups_read));
    q.opt("overtimeTossupsRead", opt_num(m.overtime_tossups_read));
    q.set("tiebreaker", b(m.tiebreaker));
    q.opt("location", opt_s(&m.location));
    q.opt("packets", opt_s(&m.packets));
    q.opt("moderator", opt_s(&m.moderator));
    q.opt("scorekeeper", opt_s(&m.scorekeeper));
    q.opt("serial", opt_s(&m.serial));
    q.opt("notes", opt_s(&m.notes));
    q.set("id", s(&match_id(t, m)));
    let mut yf = Obj::new();
    yf.set("otherValidation", validation(&m.modal_bottom_validation));
    yf.opt("importedFile", opt_s(&m.imported_file));
    q.with_yf_data(yf).done()
}

/// `Match.id`
pub fn match_id(t: &Tournament, m: &Match) -> String {
    let abbr = |mt: &MatchTeam| mt.team.map(|tm| link_id_abbr_name(&t.teams[tm])).unwrap_or_default();
    format!("Match_{}~{}{}", js::num_to_string(m.id_number), abbr(&m.left), abbr(&m.right))
}

/// `Team.getLinkIdAbbrName`: NFC-normalized letters, digits and marks, cut to 20 code points
fn link_id_abbr_name(tm: &Team) -> String {
    const TARGET: usize = 20;
    let name = letters_digits_marks(&tm.name);
    if name.len() <= TARGET {
        return name.into_iter().collect();
    }
    let letter = letters_digits_marks(&tm.letter);
    if letter.is_empty() || letter.len() > 10 {
        return name[..TARGET].iter().collect();
    }
    let mut out: String = name[..TARGET - letter.len()].iter().collect();
    out.extend(letter);
    out
}

/// `[...s.normalize('NFC').replace(/[^\p{L}\p{N}\p{M}]/gu, '')]`
fn letters_digits_marks(s: &str) -> Vec<char> {
    use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};
    use icu_properties::CodePointMapData;
    let gc = CodePointMapData::<GeneralCategory>::new();
    let nfc = icu_normalizer::ComposingNormalizerBorrowed::new_nfc().normalize(s);
    nfc.chars()
        .filter(|c| {
            let cat = gc.get(*c);
            GeneralCategoryGroup::Letter.contains(cat)
                || GeneralCategoryGroup::Number.contains(cat)
                || GeneralCategoryGroup::Mark.contains(cat)
        })
        .collect()
}

fn match_team(t: &Tournament, mt: &MatchTeam) -> Value {
    let mut q = Obj::new();
    q.opt("team", mt.team.map(|tm| ref_ptr(t.teams[tm].id())));
    q.set("forfeitLoss", b(mt.forfeit_loss));
    q.opt("points", if mt.points_null { Some(Value::Null) } else { opt_num(mt.points) });
    q.set("correctTossupsWithoutBonuses", num(correct_tossups_without_bonuses(mt)));
    q.opt("bonusBouncebackPoints", opt_num(mt.bonus_bounceback_points));
    q.opt("lightningPoints", opt_num(mt.lightning_points));
    q.set(
        "matchPlayers",
        Value::Array(
            mt.match_players
                .iter()
                .map(|mp| {
                    let mut o = Obj::new();
                    o.set("player", ref_ptr(player_id(&t.players[mp.player])));
                    o.set("tossupsHeard", num(or_zero(mp.tossups_heard)));
                    o.set("answerCounts", Value::Array(mp.answer_counts.iter().map(|ac| answer_count(t, ac)).collect()));
                    o.done()
                })
                .collect(),
        ),
    );
    let mut yf = Obj::new();
    yf.set("overTimeBuzzes", Value::Array(mt.over_time_buzzes.iter().map(|ac| answer_count(t, ac)).collect()));
    yf.set("validation", validation(&mt.modal_bottom_validation));
    yf.opt("manualBonusesHeard", opt_num(mt.manual_bonuses_heard));
    yf.opt("manualBonusPoints", opt_num(mt.manual_bonus_points));
    yf.opt("manualBouncebackPartsHeard", opt_num(mt.manual_bounceback_parts_heard));
    q.with_yf_data(yf).done()
}

/// `MatchTeam.getCorrectTossupsWithoutBonuses`
fn correct_tossups_without_bonuses(mt: &MatchTeam) -> f64 {
    let mut total = 0.0;
    for ac in &mt.over_time_buzzes {
        if ac.points() > 0.0 {
            total += or_zero(ac.number);
        }
    }
    total + mt.imported_non_bonus_tossups.unwrap_or(0.0)
}

fn answer_count(t: &Tournament, ac: &PlayerAnswerCount) -> Value {
    let mut o = Obj::new();
    o.set("number", num(or_zero(ac.number)));
    o.set("answerType", ref_ptr(t.scoring_rules.answer_types[ac.at].id()));
    o.done()
}

fn validation(c: &Collection) -> Value {
    Value::Array(
        c.validators
            .iter()
            .map(|v| {
                let mut o = Obj::new();
                o.set("status", num(v.status as i32 as f64));
                o.set("message", s(&v.message));
                o.set("suppressable", b(v.suppressable));
                o.set("isSuppressed", b(v.is_suppressed));
                o.set("type", s(v.type_string()));
                o.done()
            })
            .collect(),
    )
}

/* ------------------------------ key renaming and output ------------------------------ */

/// `camelCaseToSnakeCase`, applied in place to every object in the tree
pub fn camel_to_snake(v: &mut Value) {
    match v {
        Value::Object(map) => {
            // All the assignments run before any of the deletes, so a duplicated pair re-reads the
            // still-present camelCase value
            for (camel, snake) in CAMEL_TO_SNAKE {
                match map.get(*camel).cloned() {
                    // assigning to an existing key keeps its position; a new key goes at the end
                    Some(val) => {
                        map.insert(snake.to_string(), val);
                    }
                    // `obj.snake = undefined` drops the key from the output
                    None => {
                        map.shift_remove(*snake);
                    }
                }
            }
            for (camel, _) in CAMEL_TO_SNAKE {
                map.shift_remove(*camel);
            }
            for (_, child) in map.iter_mut() {
                camel_to_snake(child);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(camel_to_snake),
        _ => {}
    }
}

/// `JSON.stringify` with no indentation
fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(x) => out.push_str(if *x { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&js::num_to_string(n.as_f64().unwrap_or(f64::NAN))),
        Value::String(x) => out.push_str(&serde_json::to_string(x).expect("string serializes")),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("string serializes"));
                out.push(':');
                write_json(x, out);
            }
            out.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_pass_moves_renamed_keys_to_the_end_in_list_order() {
        let mut v = serde_json::json!({ "matchTeams": 1, "tiebreaker": false, "tossupsRead": 20, "id": "x" });
        camel_to_snake(&mut v);
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["tiebreaker", "id", "tossups_read", "match_teams"]);
    }

    #[test]
    fn numbers_print_like_js() {
        let mut out = String::new();
        write_json(&serde_json::json!([num(5.0), num(0.1 + 0.2), num(f64::NAN), num(-0.0), num(1e21)]), &mut out);
        assert_eq!(out, "[5,0.30000000000000004,null,0,1e+21]");
    }

    #[test]
    fn player_years() {
        assert_eq!(player_year("9"), Some(9.0));
        assert_eq!(player_year("Sophomore"), Some(14.0));
        assert_eq!(player_year("5Sr."), Some(17.0));
        assert_eq!(player_year("5th"), Some(5.0));
        assert_eq!(player_year("Faculty"), None);
        assert_eq!(player_year(""), None);
    }

    #[test]
    fn link_id_abbreviation() {
        let tm = Team { name: "Alpha-Beta Gamma Delta School B".into(), letter: "B".into(), ..Default::default() };
        assert_eq!(link_id_abbr_name(&tm), "AlphaBetaGammaDeltaB");
        let short = Team { name: "Zoë's Team".into(), ..Default::default() };
        assert_eq!(link_id_abbr_name(&short), "ZoësTeam");
    }
}
