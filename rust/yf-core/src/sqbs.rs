//! SQBS export: port of SqbsFileGeneration.ts (File > Export > SQBS)

use crate::js;
use crate::model::*;
use crate::tournament::Tournament;
use crate::writer::match_id;

/// A round, by (phase index, round index)
type RoundLoc = (usize, usize);

/// Export the given stages (`generateFile(phases)`); `Err` holds the message the app shows instead of a file
pub fn generate_sqbs(t: &Tournament, phases: &[PhaseId]) -> Result<String, String> {
    let g = Gen::new(t, phases);
    g.validate()?;
    let mut out = Out::default();
    g.add_team_list(&mut out);
    g.add_game_list(&mut out);
    g.add_misc_settings(&mut out);
    g.add_divisions(&mut out);
    g.add_tossup_point_values(&mut out);
    g.add_packets(&mut out);
    g.add_exhibition_statuses(&mut out);
    Ok(out.0)
}

/// `addLine`: lines joined with "\n" (an empty first line is swallowed, as in the TS)
#[derive(Default)]
struct Out(String);

impl Out {
    fn line(&mut self, l: impl AsRef<str>) {
        if self.0.is_empty() {
            self.0 = l.as_ref().to_string();
        } else {
            self.0.push('\n');
            self.0.push_str(l.as_ref());
        }
    }
    fn num(&mut self, x: f64) {
        self.line(js::num_to_string(x));
    }
}

struct Gen<'a> {
    t: &'a Tournament,
    phases: Vec<PhaseId>,
    team_list: Vec<TeamId>,
    use_divisions: bool,
    /// tossup answer types SQBS can hold (not 0-point ones), as (id, value)
    answer_types: Vec<(AnswerTypeId, f64)>,
    match_list: Vec<(RoundLoc, Vec<&'a Match>)>,
}

impl<'a> Gen<'a> {
    fn new(t: &'a Tournament, phases: &[PhaseId]) -> Gen<'a> {
        let phases = phases.to_vec();
        // buildCarryoverSearchPhaseList: earlier full stages whose games may carry into an exported stage
        let carryover_search: Vec<PhaseId> = (0..t.phases.len())
            .filter(|ph| t.phases[*ph].is_full_phase() && !phases.contains(ph))
            .filter(|ph| phases.iter().any(|o| t.phases[*o].has_any_carryover() && ph < o))
            .collect();
        let answer_types = t
            .scoring_rules
            .answer_types
            .iter()
            .enumerate()
            .filter(|(_, at)| at.value != 0.0)
            .map(|(i, at)| (i, at.value))
            .collect();
        let team_list = t.registrations.iter().flat_map(|r| r.teams.iter().copied()).collect();
        let use_divisions = phases.len() == 1 && t.phases[phases[0]].pools.len() > 1;
        let mut match_list = vec![];
        for ph in &phases {
            for (ri, rd) in t.phases[*ph].rounds.iter().enumerate() {
                match_list.push(((*ph, ri), rd.matches.iter().collect()));
            }
            for co in &carryover_search {
                for (ri, rd) in t.phases[*co].rounds.iter().enumerate() {
                    let ms: Vec<&Match> = rd.matches.iter().filter(|m| m.carryover_phases.contains(ph)).collect();
                    if !ms.is_empty() {
                        match_list.push(((*co, ri), ms));
                    }
                }
            }
        }
        Gen { t, phases, team_list, use_divisions, answer_types, match_list }
    }

    fn rules(&self) -> &ScoringRules {
        &self.t.scoring_rules
    }

    /// `validateExportability`
    fn validate(&self) -> Result<(), String> {
        if self.answer_types.len() > 4 {
            return Err("SQBS doesn't support formats with more than 4 tossup point values".into());
        }
        for ph in &self.phases {
            for rd in &self.t.phases[*ph].rounds {
                for m in &rd.matches {
                    if too_many_players(&m.left) || too_many_players(&m.right) {
                        return Err("SQBS doesn't support matches that involve more than 8 players on one team".into());
                    }
                }
            }
        }
        Ok(())
    }

    fn add_team_list(&self, out: &mut Out) {
        out.num(self.team_list.len() as f64);
        for tm in &self.team_list {
            let team = &self.t.teams[*tm];
            out.num(team.players.len() as f64 + 1.0);
            out.line(&team.name);
            for p in &team.players {
                let pl = &self.t.players[*p];
                let mut name = pl.name.clone();
                if !pl.year_string.is_empty() && self.t.track_player_year {
                    name.push_str(&format!(" ({})", pl.year_string));
                }
                out.line(name);
            }
        }
    }

    fn add_game_list(&self, out: &mut Out) {
        let total: usize = self.match_list.iter().map(|(_, ms)| ms.iter().filter(|m| !is_double_forfeit(m)).count()).sum();
        out.num(total as f64);
        for (rd, ms) in &self.match_list {
            for m in ms {
                self.add_one_match(out, m, *rd);
            }
        }
    }

    fn add_misc_settings(&self, out: &mut Out) {
        let r = self.rules();
        out.num(if r.use_bonuses { 1.0 } else { 0.0 });
        out.num(if r.bonuses_bounce_back { 3.0 } else { 1.0 });
        let powers_or_negs = r.answer_types.iter().any(|a| a.is_power() || a.is_neg());
        out.num(if powers_or_negs { 3.0 } else { 2.0 });
        out.num(if r.lightning_count_per_team > 0.0 { 1.0 } else { 0.0 });
        // tossups heard, packet names, all validation warnings, then every report and the stat key on;
        // no custom stylesheet
        for x in [1.0, 3.0, 254.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0] {
            out.num(x);
        }
        out.num(if self.use_divisions { 1.0 } else { 0.0 });
        out.num(1.0); // sort by record, then ppg
        out.line(&self.t.name);
        for _ in 0..4 {
            out.line(""); // FTP configuration
        }
        out.num(1.0);
        for suffix in [
            "_rounds.html",
            "_standings.html",
            "_individuals.html",
            "_games.html",
            "_teamdetail.html",
            "_playerdetail.html",
            "_statkey.html",
        ] {
            out.line(suffix);
        }
        out.line(""); // style sheet file
    }

    fn add_divisions(&self, out: &mut Out) {
        if !self.use_divisions {
            out.num(0.0);
            out.num(self.team_list.len() as f64);
            for _ in &self.team_list {
                out.num(-1.0);
            }
            return;
        }
        let ph = &self.t.phases[self.phases[0]];
        out.num(ph.pools.len() as f64);
        for p in &ph.pools {
            out.line(&p.name);
        }
        out.num(self.team_list.len() as f64);
        for tm in &self.team_list {
            out.num(ph.find_pool_with_team(*tm).map(|i| i as f64).unwrap_or(-1.0));
        }
    }

    fn add_tossup_point_values(&self, out: &mut Out) {
        for i in 0..4 {
            out.num(self.answer_types.get(i).map(|a| a.1).unwrap_or(0.0));
        }
    }

    /// `addPackets`: one packet name per distinct round in the match list, in first-seen order
    fn add_packets(&self, out: &mut Out) {
        let mut rounds: Vec<RoundLoc> = vec![];
        for (rd, _) in &self.match_list {
            if !rounds.contains(rd) {
                rounds.push(*rd);
            }
        }
        out.num(rounds.len() as f64);
        for (pi, ri) in rounds {
            out.line(&self.t.phases[pi].rounds[ri].packet_name);
        }
    }

    fn add_exhibition_statuses(&self, out: &mut Out) {
        out.num(self.team_list.len() as f64);
        for _ in &self.team_list {
            out.num(0.0);
        }
    }

    fn add_one_match(&self, out: &mut Out, m: &Match, (pi, ri): RoundLoc) {
        if is_double_forfeit(m) {
            return;
        }
        let r = self.rules();
        // the first team is always the winner of a forfeit in SQBS
        let (lw, rw) = if m.left.forfeit_loss { (Side::Right, Side::Left) } else { (Side::Left, Side::Right) };
        let (lt, rt) = (m.side(lw), m.side(rw));
        let (Some(lteam), Some(rteam)) = (lt.team, rt.team) else { return };
        let forfeit = m.is_forfeit();
        fn heard(mt: &MatchTeam) -> Vec<&MatchPlayer> {
            mt.match_players.iter().filter(|mp| mp.tossups_heard.unwrap_or(0.0) > 0.0).collect()
        }
        let (lps, rps) = (heard(lt), heard(rt));

        out.line(match_id(self.t, m));
        out.num(self.team_id(lteam));
        out.num(self.team_id(rteam));
        out.num(if forfeit { -1.0 } else { lt.points.unwrap_or(0.0) });
        out.num(if forfeit { -1.0 } else { rt.points.unwrap_or(0.0) });
        out.num(if forfeit { 0.0 } else { m.tossups_read.unwrap_or(0.0) });
        out.num(self.t.phases[pi].rounds[ri].number);
        if r.use_bonuses && !r.bonuses_bounce_back {
            out.num(lt.bonuses_heard(r));
            out.num(lt.bonus_points());
            out.num(rt.bonuses_heard(r));
            out.num(rt.bonus_points());
        } else if r.bonuses_bounce_back {
            for (mt, side) in [(lt, lw), (rt, rw)] {
                out.num(10000.0 * m.bounceback_parts_heard(side, r) + mt.bonuses_heard(r));
                out.num(10000.0 * mt.bonus_bounceback_points.unwrap_or(0.0) + mt.bonus_points());
            }
        } else {
            for _ in 0..4 {
                out.num(0.0);
            }
        }
        out.num(if m.overtime_tossups_read.unwrap_or(0.0) > 0.0 { 1.0 } else { 0.0 });
        out.num(self.ot_tossups_converted(lt));
        out.num(self.ot_tossups_converted(rt));
        out.num(if forfeit { 1.0 } else { 0.0 });
        out.num(lt.lightning_points.unwrap_or(0.0));
        out.num(rt.lightning_points.unwrap_or(0.0));
        for i in 0..8 {
            self.add_match_player(out, &lps, i, m, lteam);
            self.add_match_player(out, &rps, i, m, rteam);
        }
    }

    fn add_match_player(&self, out: &mut Out, mps: &[&MatchPlayer], i: usize, m: &Match, team: TeamId) {
        let Some(mp) = mps.get(i) else {
            for x in [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] {
                out.num(x);
            }
            return;
        };
        let pid = self.t.teams[team].players.iter().position(|p| *p == mp.player).map(|i| i as f64).unwrap_or(-1.0);
        out.num(pid);
        // games played
        out.line(js::to_fixed(mp.tossups_heard.unwrap_or(0.0) / m.tossups_read.unwrap_or(0.0), 2));
        for k in 0..4 {
            match self.answer_types.get(k) {
                None => out.num(0.0),
                Some((_, value)) => {
                    let n = mp.answer_counts.iter().find(|ac| ac.value == *value).and_then(|ac| ac.number);
                    out.num(n.unwrap_or(0.0));
                }
            }
        }
        out.num(mp.points());
    }

    /// In SQBS, overtime tossups only count when overtime has no bonuses
    fn ot_tossups_converted(&self, mt: &MatchTeam) -> f64 {
        let r = self.rules();
        if r.overtime_includes_bonuses || !r.use_bonuses {
            return 0.0;
        }
        mt.correct_tossups_without_bonuses()
    }

    fn team_id(&self, tm: TeamId) -> f64 {
        self.team_list.iter().position(|t| *t == tm).map(|i| i as f64).unwrap_or(-1.0)
    }
}

/// SQBS can't store a match where more than 8 players on one team heard tossups
fn too_many_players(mt: &MatchTeam) -> bool {
    mt.match_players.iter().filter(|mp| mp.tossups_heard.unwrap_or(0.0) > 0.0).count() > 8
}

fn is_double_forfeit(m: &Match) -> bool {
    m.left.forfeit_loss && m.right.forfeit_loss
}
