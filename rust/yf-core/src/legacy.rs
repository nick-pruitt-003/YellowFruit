//! Port of OldYfParsing.ts: YellowFruit 1.x-3.x files (six newline-separated JSON sections)

use serde_json::Value;

use crate::js;
use crate::jsval::{get, js_keys, jv, Jv};
use crate::model::*;
use crate::tournament::Tournament;

/// `isOldYftFile`
pub fn is_old_yft_file(contents: &str) -> bool {
    let line_end = contents.find(['\n', '\r', '\u{2028}', '\u{2029}']).unwrap_or(contents.len());
    let line = &contents[..line_end];
    // /^\{"version":"\d+\.\d+\.\d+"\}/
    if let Some(rest) = contents.strip_prefix("{\"version\":\"") {
        let mut it = rest.char_indices().peekable();
        let mut ok = true;
        for part in 0..3 {
            let mut n = 0;
            while let Some((_, c)) = it.peek() {
                if c.is_ascii_digit() {
                    it.next();
                    n += 1;
                } else {
                    break;
                }
            }
            if n == 0 {
                ok = false;
                break;
            }
            if part < 2 {
                if it.next().map(|x| x.1) != Some('.') {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            let tail: String = it.map(|x| x.1).take(2).collect();
            if tail == "\"}" {
                return true;
            }
        }
    }
    // /^\{.*"powers":.*\}/
    if line.starts_with('{') {
        if let Some(p) = line[1..].find("\"powers\":") {
            if line[1 + p + 9..].contains('}') {
                return true;
            }
        }
    }
    false
}

struct PlayerLine {
    tuh: Option<f64>,
    powers: Option<f64>,
    tens: Option<f64>,
    negs: Option<f64>,
}

struct OldGame {
    round: Jv<'static>,
    phases: Vec<Option<String>>,
    tuhtot: Option<f64>,
    ottu: Option<f64>,
    forfeit: bool,
    team1: Option<String>,
    team2: Option<String>,
    /// (value, was null)
    score1: (Option<f64>, bool),
    score2: (Option<f64>, bool),
    ot: [[Option<f64>; 3]; 2],
    bb: [Option<f64>; 2],
    lightning: [Option<f64>; 2],
    notes: Option<String>,
    tiebreaker: bool,
    players: [Vec<(String, PlayerLine)>; 2],
}

struct OldPlayer {
    name: String,
    year: Option<String>,
    undergrad: bool,
    div2: bool,
}

struct OldTeam<'a> {
    name: String,
    ug: bool,
    d2: bool,
    small_school: bool,
    jv: bool,
    roster: Vec<OldPlayer>,
    divisions: Option<&'a serde_json::Map<String, Value>>,
}

/// Number field: `+x` if the file predates 2.5.2 (gameConversion2x5x2), else the raw value
fn num(v: Jv, coerce: bool, field: &str) -> Result<Option<f64>, String> {
    if coerce {
        return Ok(Some(v.to_number()));
    }
    v.opt_num(field)
}

/// Scores: `null` shows up in real files and needs JS null semantics (see MatchTeam::points_null)
fn score_field(v: Jv, coerce: bool) -> Result<(Option<f64>, bool), String> {
    if !coerce {
        if let Jv::Null = v {
            return Ok((Some(0.0), true));
        }
    }
    Ok((num(v, coerce, "score")?, false))
}

/// Round numbers are compared with ===, so keep the raw value. Leak is fine: tiny, per-file.
fn static_jv(v: Jv) -> Jv<'static> {
    match v {
        Jv::Undef => Jv::Undef,
        Jv::Null => Jv::Null,
        Jv::Bool(b) => Jv::Bool(b),
        Jv::Num(n) => Jv::Num(n),
        Jv::Str(s) => Jv::Str(Box::leak(s.to_string().into_boxed_str())),
        Jv::Arr(_) | Jv::Obj(_) => Jv::Bool(true),
    }
}

pub fn parse_old_yf_file(contents: &str) -> Result<Tournament, String> {
    let pieces = file_pieces(contents);
    let parse = |i: usize| -> Result<Value, String> {
        let s = pieces.get(i).map(|s| s.as_str()).unwrap_or("");
        serde_json::from_str::<Value>(s).map_err(|e| format!("JSON parse error in section {i}: {e}"))
    };
    let meta = parse(0)?;
    let packets = parse(1)?;
    let settings = parse(2)?;
    let phases = parse(3)?;
    let teams_v = parse(4)?;
    let games_v = parse(5)?;

    let version = match get(&meta, "version") {
        Jv::Str(s) if !s.is_empty() => s.to_string(),
        _ => return Err("Unrecognized file format.".into()),
    };
    if js::version_lt("3.0.2", &version) {
        return Err("This file has an unsupported version number.".into());
    }
    let lt = |v: &str| js::version_lt(&version, v);

    // ---- settings (with settingsConversion2x5x0 when it applies) ----
    let has_default_phase = !get(&settings, "defaultPhase").is_undef();
    let s_conv = has_default_phase && lt("2.5.0");
    let (bonuses, bonuses_bounce, negs, lightning) = if s_conv {
        let b = get(&settings, "bonuses");
        let n = get(&settings, "negs");
        (
            b.as_str() != Some("none"),
            b.as_str() == Some("yesBb"),
            n.as_str() == Some("yes"),
            false,
        )
    } else {
        (
            get(&settings, "bonuses").truthy(),
            get(&settings, "bonusesBounce").truthy(),
            get(&settings, "negs").truthy(),
            get(&settings, "lightning").truthy(),
        )
    };
    let powers = get(&settings, "powers");
    let mut rules = ScoringRules { answer_types: vec![], ..Default::default() };
    if powers.as_str() == Some("20pts") {
        rules.answer_types.push(AnswerType::new(20.0));
    }
    if powers.as_str() == Some("15pts") {
        rules.answer_types.push(AnswerType::new(15.0));
    }
    rules.answer_types.push(AnswerType::new(10.0));
    if negs {
        rules.answer_types.push(AnswerType::new(-5.0));
    }
    rules.set_use_bonuses(bonuses);
    rules.bonuses_bounce_back = bonuses && bonuses_bounce;
    rules.lightning_count_per_team = if lightning { 1.0 } else { 0.0 };
    rules.maximum_players_per_team = get(&settings, "playersPerTeam").to_number();

    // ---- teams ----
    let teams_arr = teams_v.as_array().ok_or("oldYfTeams is not iterable")?;
    let mut old_teams = vec![];
    for t in teams_arr {
        let name = get(t, "teamName").as_str().ok_or("team name is not a string")?.to_string();
        let mut roster = vec![];
        match get(t, "roster") {
            Jv::Arr(a) if lt("2.1.0") => {
                // teamConversion2x1x0: array of names -> object; later keys win but keep first position
                let mut seen: Vec<String> = vec![];
                for p in a {
                    let n = match p {
                        Value::String(s) => s.clone(),
                        other => crate::jsval::jv(Some(other)).to_number().to_string(),
                    };
                    if !seen.contains(&n) {
                        seen.push(n);
                    }
                }
                for n in seen {
                    roster.push(OldPlayer { name: n, year: Some(String::new()), undergrad: false, div2: false });
                }
            }
            Jv::Obj(o) => {
                for k in js_keys(o) {
                    let d = &o[k];
                    let year = match get(d, "year") {
                        Jv::Str(s) => Some(s.to_string()),
                        Jv::Undef => None,
                        other => Some(format!("{other:?}")),
                    };
                    let (ug, d2) = if lt("2.2.0") {
                        (false, false)
                    } else {
                        (get(d, "undergrad").truthy(), get(d, "div2").truthy())
                    };
                    roster.push(OldPlayer { name: k.clone(), year, undergrad: ug, div2: d2 });
                }
            }
            _ => {}
        }
        let (ug, d2) = if lt("2.2.0") {
            (false, false)
        } else {
            (get(t, "teamUGStatus").truthy(), get(t, "teamD2Status").truthy())
        };
        let (ss, jvs) = if lt("2.3.0") {
            (false, false)
        } else {
            (get(t, "smallSchool").truthy(), get(t, "jrVarsity").truthy())
        };
        let divisions = match get(t, "divisions") {
            Jv::Obj(o) => Some(o),
            _ => None,
        };
        old_teams.push(OldTeam { name, ug, d2, small_school: ss, jv: jvs, roster, divisions });
    }

    // ---- games ----
    let coerce = lt("2.5.2");
    let games_arr = games_v.as_array().ok_or("oldYfGames is not iterable")?;
    let mut games = vec![];
    for g in games_arr {
        let round = if coerce {
            // g.round = +(g.round ?? 0)
            let r = get(g, "round");
            Jv::Num(match r {
                Jv::Undef | Jv::Null => 0.0,
                other => other.to_number(),
            })
        } else {
            static_jv(get(g, "round"))
        };
        if let Jv::Str(_) = round {
            return Err("unsupported: string round number in a post-2.5.2 file".into());
        }
        let phases = match get(g, "phases") {
            Jv::Arr(a) => a.iter().map(|v| v.as_str().map(|s| s.to_string())).collect(),
            _ => return Err("Cannot read properties of undefined (reading 'filter') [game.phases]".into()),
        };
        let lt250 = lt("2.5.0");
        let field = |k: &str| num(get(g, k), coerce, k);
        let lightning_field = |k: &str| -> Result<Option<f64>, String> {
            if lt250 {
                // gameConversion2x5x0 sets '', then 2x5x2 turns it into 0
                return Ok(Some(0.0));
            }
            field(k)
        };
        let mut players: [Vec<(String, PlayerLine)>; 2] = [vec![], vec![]];
        for (i, key) in ["players1", "players2"].iter().enumerate() {
            if let Jv::Obj(o) = get(g, key) {
                for k in js_keys(o) {
                    let line = &o[k];
                    let f = |x: &str| num(get(line, x), coerce, x);
                    players[i].push((k.clone(), PlayerLine { tuh: f("tuh")?, powers: f("powers")?, tens: f("tens")?, negs: f("negs")? }));
                }
            }
        }
        games.push(OldGame {
            round,
            phases,
            tuhtot: field("tuhtot")?,
            ottu: field("ottu")?,
            forfeit: get(g, "forfeit").truthy(),
            team1: get(g, "team1").as_str().map(|s| s.to_string()),
            team2: get(g, "team2").as_str().map(|s| s.to_string()),
            score1: score_field(get(g, "score1"), coerce)?,
            score2: score_field(get(g, "score2"), coerce)?,
            ot: [
                [field("otPwr1")?, field("otTen1")?, field("otNeg1")?],
                [field("otPwr2")?, field("otTen2")?, field("otNeg2")?],
            ],
            bb: [field("bbPts1")?, field("bbPts2")?],
            lightning: [lightning_field("lightningPts1")?, lightning_field("lightningPts2")?],
            notes: get(g, "notes").as_str().map(|s| s.to_string()),
            tiebreaker: if lt("2.4.0") { false } else { get(g, "tiebreaker").truthy() },
            players,
        });
    }

    let mut t = Tournament::new();
    t.scoring_rules = rules;
    parse_teams(&mut t, &old_teams);
    t.phases = parse_phase_structure(&t, &phases, &games, &old_teams)?;
    parse_packets(&mut t, &packets);
    parse_games(&mut t, &games);
    attributes_to_track(&mut t, &old_teams);
    Ok(t)
}

fn file_pieces(contents: &str) -> Vec<String> {
    let mut s = contents.to_string();
    for _ in 0..3 {
        if let Some(i) = s.find("divider_between_sections\n") {
            s.replace_range(i..i + "divider_between_sections\n".len(), "");
        }
    }
    let ary: Vec<String> = s.split('\n').take(6).map(|x| x.to_string()).collect();
    if ary.len() < 5 {
        let mut v = vec!["{\"version\":\"1.9.9\"}".to_string(), "{}".to_string()];
        v.extend(ary);
        return v;
    }
    ary
}

fn parse_teams(t: &mut Tournament, old: &[OldTeam]) {
    for team in old {
        // Trimmed, like the .yft loader, so team IDs survive a save and reopen
        let team_name = js::trim(&team.name);
        let (reg_name, letter) = js::team_get_name_and_letter(team_name);
        let existing = t.find_registration(&reg_name);
        let tid = t.new_team(team_name);
        {
            let tm = &mut t.teams[tid];
            tm.letter = letter;
            tm.is_jv = team.jv;
            tm.is_ug = team.ug;
            tm.is_d2 = team.d2;
        }
        for p in &team.roster {
            // the .yft loader can't take players with no name
            if js::trim(&p.name).is_empty() {
                continue;
            }
            let pid = t.new_player(js::trim(&p.name));
            let pl = &mut t.players[pid];
            pl.year_string = p.year.clone().unwrap_or_default();
            pl.is_ug = p.undergrad;
            pl.is_d2 = p.div2;
            t.teams[tid].players.push(pid);
        }
        match existing {
            Some(r) => t.registrations[r].teams.push(tid),
            None => {
                let reg = Registration { name: reg_name, teams: vec![tid], is_small_school: team.small_school };
                t.add_registration(reg);
            }
        }
    }
}

fn round_num(g: &OldGame) -> Option<f64> {
    match g.round {
        Jv::Num(n) => Some(n),
        _ => None,
    }
}

fn parse_phase_structure(t: &Tournament, phases: &Value, games: &[OldGame], old_teams: &[OldTeam]) -> Result<Vec<Phase>, String> {
    // findMaxRoundNumber
    let mut max_round = 1.0;
    for g in games {
        if g.round.truthy() {
            if let Some(r) = round_num(g) {
                if r > max_round {
                    max_round = r;
                }
            }
        }
    }
    // findPrelimPhase
    let mut prelim: Option<String> = None;
    for g in games {
        if round_num(g) == Some(1.0) && g.phases.len() == 1 {
            prelim = g.phases[0].clone();
            break;
        }
    }
    let mut ranges: Vec<PhaseRange> = vec![];
    let mut code = 0;
    let empty = serde_json::Map::new();
    let phase_obj = match jv(Some(phases)) {
        Jv::Obj(o) => o,
        _ => &empty,
    };
    for phase_name in js_keys(phase_obj) {
        if prelim.as_deref().map_or(true, |p| p.is_empty()) {
            prelim = Some(phase_name.clone());
        }
        let ptype = if Some(phase_name.as_str()) == prelim.as_deref() { PhaseType::Prelim } else { PhaseType::Playoff };
        code += 1;
        // findPhaseRoundRange
        let (mut first, mut last) = (9999.0, 0.0);
        let mut round = 0.0;
        while round <= max_round {
            let in_round: Vec<&OldGame> = games.iter().filter(|g| round_num(g) == Some(round)).collect();
            let in_phase = in_round.iter().filter(|g| g.phases.iter().any(|p| p.as_deref() == Some(phase_name))).count();
            if in_phase as f64 > in_round.len() as f64 / 2.0 {
                if round < first {
                    first = round;
                }
                if round > last {
                    last = round;
                }
            }
            round += 1.0;
        }
        if last == 0.0 {
            continue;
        }
        ranges.push(PhaseRange {
            name: phase_name.clone(),
            ptype,
            code: code.to_string(),
            first,
            last,
            divisions: phase_obj[phase_name].as_array(),
        });
    }
    cover_unassigned_rounds(&mut ranges, games, max_round, code + 1);

    let mut out = vec![];
    for r in &ranges {
        let mut phase = Phase::new(r.ptype, r.first, r.last, &r.code, Some(&r.name));
        for div in r.divisions.into_iter().flatten() {
            let mut in_div = vec![];
            for tm in old_teams {
                let Some(d) = tm.divisions else {
                    return Err("Cannot read properties of undefined (team.divisions)".into());
                };
                if d.get(&r.name).is_some_and(|v| v == div) {
                    in_div.push(tm);
                }
            }
            let name = div.as_str().unwrap_or("");
            let mut pool = Pool::new(in_div.len() as f64, 1.0, name, true);
            for tm in in_div {
                if let Some(id) = t.find_team_by_name(js::trim(&tm.name)) {
                    pool.add_team(id);
                }
            }
            phase.pools.push(pool);
        }
        if phase.pools.is_empty() {
            phase.pools.push(Pool::new(99.0, 1.0, "New Pool", true));
        }
        out.push(phase);
    }
    Ok(out)
}

/// A stage found in an old file, before its Phase is built
struct PhaseRange<'a> {
    name: String,
    ptype: PhaseType,
    code: String,
    first: f64,
    last: f64,
    divisions: Option<&'a Vec<Value>>,
}

/// `coverUnassignedRounds`: a round with games that no stage claims goes into the stage before it (or
/// after it, for rounds before the first stage); with no stages at all, one "Prelims" stage takes every round
fn cover_unassigned_rounds(ranges: &mut Vec<PhaseRange>, games: &[OldGame], max_round: f64, next_code: i32) {
    let rounds_with_games: Vec<f64> = games.iter().filter_map(round_num).filter(|r| *r > 0.0).collect();
    if rounds_with_games.is_empty() {
        return;
    }
    if ranges.is_empty() {
        ranges.push(PhaseRange {
            name: "Prelims".into(),
            ptype: PhaseType::Prelim,
            code: next_code.to_string(),
            first: 1.0,
            last: max_round,
            divisions: None,
        });
        return;
    }
    let mut round = 1.0;
    while round <= max_round {
        let unassigned =
            rounds_with_games.contains(&round) && !ranges.iter().any(|r| r.first <= round && round <= r.last);
        if unassigned {
            // reduce((a, b) => b.x > a.x ? b : a): the first one wins ties
            let mut before: Option<usize> = None;
            for (i, r) in ranges.iter().enumerate() {
                if r.last < round && before.is_none_or(|b| r.last > ranges[b].last) {
                    before = Some(i);
                }
            }
            match before {
                Some(i) => ranges[i].last = round,
                None => {
                    let mut first = 0;
                    for (i, r) in ranges.iter().enumerate() {
                        if r.first < ranges[first].first {
                            first = i;
                        }
                    }
                    ranges[first].first = round;
                }
            }
        }
        round += 1.0;
    }
}

fn parse_packets(t: &mut Tournament, packets: &Value) {
    let Jv::Obj(o) = jv(Some(packets)) else { return };
    for k in js_keys(o) {
        let n = js::parse_int(k);
        if let Some((pi, ri)) = t.round_by_number(n) {
            t.phases[pi].rounds[ri].packet_name = match &o[k] {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
        }
    }
}

fn add_team_to_arbitrary_pool(t: &mut Tournament, team: TeamId, pi: PhaseId) {
    let phase = &mut t.phases[pi];
    for pool in &mut phase.pools {
        if (pool.pool_teams.len() as f64) < pool.size {
            pool.add_team(team);
            return;
        }
    }
    let mut p = Pool::new(99.0, 1.0, "New Pool", true);
    p.add_team(team);
    phase.pools.push(p);
}

fn parse_games(t: &mut Tournament, games: &[OldGame]) {
    let mut next_match_id = 1000.0;
    for g in games {
        if !g.round.truthy() {
            continue;
        }
        let Some(rn) = round_num(g) else { continue };
        let Some((pi, ri)) = t.round_by_number(rn) else { continue };
        let main_name = t.phases[pi].name.clone();
        let carryover: Vec<PhaseId> = g
            .phases
            .iter()
            .filter(|p| p.as_deref() != Some(main_name.as_str()))
            .filter_map(|p| p.as_deref().and_then(|n| t.find_phase_by_name(n)))
            .collect();
        let (Some(lt), Some(rt)) = (
            g.team1.as_deref().and_then(|n| t.find_team_by_name(js::trim(n))),
            g.team2.as_deref().and_then(|n| t.find_team_by_name(js::trim(n))),
        ) else {
            continue;
        };
        if t.pool_with_team_in_phase(lt, pi).is_none() {
            add_team_to_arbitrary_pool(t, lt, pi);
        }
        if t.pool_with_team_in_phase(rt, pi).is_none() {
            add_team_to_arbitrary_pool(t, rt, pi);
        }

        let rules = &t.scoring_rules;
        let left = MatchTeam::new(Some(lt), Some(rules), &t.teams);
        let right = MatchTeam::new(Some(rt), Some(rules), &t.teams);
        let mut m = Match::new(next_match_id, left, right);
        next_match_id += 1.0;
        m.carryover_phases = carryover;
        // forfeits have no tossups read, as when a .yft file is opened
        m.tossups_read = if g.forfeit { None } else { g.tuhtot };
        m.overtime_tossups_read = g.ottu;
        m.tiebreaker = g.tiebreaker;
        m.notes = g.notes.clone();
        if g.forfeit {
            m.right.forfeit_loss = true;
        }
        (m.left.points, m.left.points_null) = g.score1;
        (m.right.points, m.right.points_null) = g.score2;
        m.left.bonus_bounceback_points = g.bb[0];
        m.right.bonus_bounceback_points = g.bb[1];
        m.left.lightning_points = g.lightning[0];
        m.right.lightning_points = g.lightning[1];

        for (side, team) in [(Side::Left, lt), (Side::Right, rt)] {
            let si = if side == Side::Left { 0 } else { 1 };
            for (pname, line) in &g.players[si] {
                // Team.findPlayerByName: first player with that exact name
                let Some(pid) = t.teams[team].players.iter().copied().find(|p| t.players[*p].name == js::trim(pname)) else {
                    continue;
                };
                let mt = m.side_mut(side);
                let Some(mp) = mt.match_players.iter_mut().find(|mp| mp.player == pid) else { continue };
                mp.tossups_heard = line.tuh;
                if let Some(ac) = mp.answer_counts.iter_mut().find(|ac| ac.value > 10.0) {
                    ac.add_to_count(line.powers);
                }
                if let Some(ac) = mp.answer_counts.iter_mut().find(|ac| ac.value == 10.0) {
                    ac.add_to_count(line.tens);
                }
                if let Some(ac) = mp.answer_counts.iter_mut().find(|ac| ac.value == -5.0) {
                    ac.add_to_count(line.negs);
                }
            }
            let ot = &g.ot[si];
            let buzzes = &mut m.side_mut(side).over_time_buzzes;
            if let Some(ac) = buzzes.iter_mut().find(|ac| ac.value > 10.0) {
                ac.add_to_count(ot[0]);
            }
            if let Some(ac) = buzzes.iter_mut().find(|ac| ac.value == 10.0) {
                ac.add_to_count(ot[1]);
            }
            if let Some(ac) = buzzes.iter_mut().find(|ac| ac.value == -5.0) {
                ac.add_to_count(ot[2]);
            }
        }

        let names = Names { teams: &t.teams, players: &t.players };
        m.validate_all(&t.scoring_rules, &names);
        if m.is_error_status(&names) {
            m.omit = true;
        }
        t.phases[pi].rounds[ri].matches.push(m);
    }
}

fn attributes_to_track(t: &mut Tournament, old: &[OldTeam]) {
    t.track_player_year = false;
    for tm in old {
        if tm.small_school {
            t.track_small_school = true;
        }
        if tm.jv {
            t.track_jv = true;
        }
        if tm.ug {
            t.track_ug = true;
        }
        if tm.d2 {
            t.track_div2 = true;
        }
        if !t.track_player_year && tm.roster.iter().any(|p| p.year.as_deref() != Some("")) {
            t.track_player_year = true;
        }
    }
}
