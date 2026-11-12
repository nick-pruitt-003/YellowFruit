//! Builds the same JSON as rust/oracle/dump.ts. Keep the two in sync field for field.

use serde_json::{json, Value};
use yf_core::js;
use yf_core::model::*;
use yf_core::stats::{AggregateStandings, Count, PhaseStandings, PlayerStats, PoolTeamStats, RoundStats};
use yf_core::Tournament;

fn num(x: f64) -> Value {
    if x.is_nan() {
        json!("NaN")
    } else if x == f64::INFINITY {
        json!("Inf")
    } else if x == f64::NEG_INFINITY {
        json!("-Inf")
    } else {
        json!(x)
    }
}

fn onum(x: Option<f64>) -> Value {
    x.map(num).unwrap_or(Value::Null)
}

fn score(mt: &MatchTeam) -> Value {
    if mt.points_null {
        Value::Null
    } else {
        onum(mt.points)
    }
}

fn counts(c: &[Count]) -> Value {
    Value::Array(c.iter().map(|c| json!([num(c.value), onum(c.number)])).collect())
}

pub fn dump_tournament(t: &mut Tournament) -> Value {
    let reg_tu = t.scoring_rules.regulation_tossup_count();
    let names = t.names();
    let r = &t.scoring_rules;
    let structure = json!({
        "name": t.name,
        "rules": {
            "answerTypes": r.answer_types.iter().map(|a| num(a.value)).collect::<Vec<_>>(),
            "timed": r.timed,
            "maxRegTu": num(r.maximum_regulation_tossup_count),
            "maxPlayers": num(r.maximum_players_per_team),
            "useBonuses": r.use_bonuses,
            "bounceBack": r.bonuses_bounce_back,
            "lightning": num(r.lightning_count_per_team),
        },
        "registrations": t.registrations.iter().map(|reg| json!({
            "name": reg.name,
            "teams": reg.teams.iter().map(|tid| {
                let tm = &t.teams[*tid];
                json!({ "name": tm.name, "letter": tm.letter, "players": tm.players.iter().map(|p| t.players[*p].name.clone()).collect::<Vec<_>>() })
            }).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "seeds": t.seeds.iter().map(|s| t.teams[*s].name.clone()).collect::<Vec<_>>(),
        "phases": t.phases.iter().map(|ph| json!({
            "name": ph.name,
            "type": ph.phase_type.as_str(),
            "code": ph.code,
            "pools": ph.pools.iter().map(|p| json!({
                "name": p.name,
                "position": num(p.position),
                "size": num(p.size),
                "teams": p.pool_teams.iter().map(|pt| t.teams[pt.team].name.clone()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "rounds": ph.rounds.iter().map(|rd| json!({
                "number": num(rd.number),
                "name": rd.name(),
                "packet": rd.packet_name,
                "matches": rd.matches.iter().map(|m| json!({
                    "left": m.left.team.map(|x| t.teams[x].name.clone()),
                    "right": m.right.team.map(|x| t.teams[x].name.clone()),
                    "score": [score(&m.left), score(&m.right)],
                    "tuh": onum(m.tossups_read),
                    "ot": onum(m.overtime_tossups_read),
                    "carryover": m.carryover_phases.iter().map(|p| t.phases[*p].name.clone()).collect::<Vec<_>>(),
                    "omit": m.omit,
                    "errors": m.error_messages(false, &names),
                    "warnings": m.warning_messages(&names),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });

    t.compile_stats(false, true);
    let in_app = json!({
        "phases": t.stats.iter().map(|ps| phase_standings(t, ps, reg_tu, false)).collect::<Vec<_>>(),
        "prelimsPlusPlayoffs": t.prelims_plus_playoff_stats.as_ref().map(|ps| phase_standings(t, ps, reg_tu, false)),
    });

    t.prelims_plus_playoff_stats = None;
    t.compile_stats(true, false);
    let report = json!({
        "phases": t.stats.iter().map(|ps| phase_standings(t, ps, reg_tu, true)).collect::<Vec<_>>(),
        "prelimsPlusPlayoffs": t.prelims_plus_playoff_stats.as_ref().map(|ps| phase_standings(t, ps, reg_tu, false)),
        "cumulative": t.cumulative_stats.as_ref().map(|a| aggregate(t, a, reg_tu)),
    });

    json!({ "structure": structure, "inApp": in_app, "report": report })
}

fn pool_team_stats(t: &Tournament, s: &PoolTeamStats, reg_tu: f64) -> Value {
    let r = &t.scoring_rules;
    json!({
        "team": t.teams[s.team].name,
        "rank": s.rank,
        "rawRank": num(s.raw_rank),
        "finalRankCalculated": onum(s.final_rank_calculated),
        "currentSeed": onum(s.current_seed),
        "advanceToTier": onum(s.advance_to_tier),
        "recordTie": s.record_tie_for_advancement,
        "ppgTie": s.ppg_tie_for_advancement,
        "record": s.record(),
        "winPct": s.win_pct_string(),
        "wlt": [num(s.wins), num(s.losses), num(s.ties)],
        "nonForfeit": num(s.non_forfeit_matches),
        "tuh": [num(s.tuh_total), num(s.tuh_regulation)],
        "points": [num(s.total_points), num(s.total_points_for_ppg)],
        "ppg": s.pts_per_reg_tuh_string(r, reg_tu),
        "pptuhRaw": num(s.pts_per_reg_tuh(r)),
        "bonus": [num(s.bonuses_heard), num(s.bonus_points)],
        "ppb": s.pts_per_bonus_string(),
        "ppbRaw": num(s.pts_per_bonus()),
        "bb": [num(s.bounce_back_points), num(s.bounce_back_parts_heard), num(s.bounce_back_parts_converted)],
        "bbPct": s.bounceback_conv_pct_string(r),
        "lightning": [num(s.lightning_points), s.lightning_pts_per_match_string()],
        "counts": counts(&s.tossup_counts),
        "matches": s.matches.len(),
    })
}

fn player_stats(t: &Tournament, p: &PlayerStats) -> Value {
    json!({
        "player": t.players[p.player].name,
        "team": t.teams[p.team].name,
        "rank": p.rank,
        "rankTie": p.rank_tie,
        "gp": js::to_fixed(p.games_played, 1),
        "gpRaw": num(p.games_played),
        "tuh": num(p.tossups_heard),
        "points": num(p.total_points()),
        "pptuh": onum(p.pptuh()),
        "counts": counts(&p.tossup_counts),
        "matches": p.matches.len(),
    })
}

fn phase_standings(t: &Tournament, ps: &PhaseStandings, reg_tu: f64, with_players: bool) -> Value {
    json!({
        "phase": t.phases[ps.phase].name,
        "anyTies": ps.any_ties_exist,
        "pools": ps.pools.iter().map(|p| json!({
            "pool": p.pool.name,
            "teams": p.pool_teams.iter().map(|s| pool_team_stats(t, s, reg_tu)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "players": if with_players { Value::Array(ps.players.iter().map(|p| player_stats(t, p)).collect()) } else { Value::Null },
    })
}

fn round_stats(t: &Tournament, r: &RoundStats) -> Value {
    json!({
        "round": num(r.round_number),
        "phase": r.phase.map(|p| t.phases[p].name.clone()),
        "games": num(r.games),
        "tuh": [num(r.tossups_heard_total), num(r.tossups_heard_regulation)],
        "points": num(r.points),
        "bonusPoints": num(r.bonus_points),
        "lightning": num(r.lightning_points),
        "bbConv": num(r.bounceback_parts_converted),
        "counts": counts(&r.tossup_counts),
        "derived": r.derived(&t.scoring_rules).iter().map(|x| num(*x)).collect::<Vec<_>>(),
    })
}

fn aggregate(t: &Tournament, a: &AggregateStandings, reg_tu: f64) -> Value {
    json!({
        "anyTies": a.any_ties_exist,
        "teams": a.team_stats.iter().map(|s| pool_team_stats(t, s, reg_tu)).collect::<Vec<_>>(),
        "players": a.players.iter().map(|p| player_stats(t, p)).collect::<Vec<_>>(),
        "rounds": a.rounds.iter().map(|r| round_stats(t, r)).collect::<Vec<_>>(),
        "total": round_stats(t, &a.total),
    })
}

#[allow(dead_code)]
fn _unused(_: &Match) {}
