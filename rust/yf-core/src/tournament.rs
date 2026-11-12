//! Port of the parts of Tournament.ts that parsing and stats need

use crate::js;
use crate::model::*;
use crate::stats::{AggregateStandings, PhaseStandings};

/// Location of a match: (phase index, round index, match index)
pub type MatchLoc = (usize, usize, usize);

#[derive(Clone, Debug, Default)]
pub struct Tournament {
    pub name: String,
    pub site_name: String,
    pub question_set: String,
    /// raw ISO strings as they appeared in the file
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub scoring_rules: ScoringRules,
    pub standard_rule_set: Option<String>,
    pub registrations: Vec<Registration>,
    pub teams: Vec<Team>,
    pub players: Vec<Player>,
    pub phases: Vec<Phase>,
    pub seeds: Vec<TeamId>,
    pub track_player_year: bool,
    pub track_small_school: bool,
    pub track_jv: bool,
    pub track_ug: bool,
    pub track_div2: bool,
    pub finals_rankings_ready: bool,
    pub using_schedule_template: bool,
    pub has_match_data: bool,
    pub app_version: String,
    /// next Player id number (Player.idCounter)
    pub next_player_id: u64,

    pub stats: Vec<PhaseStandings>,
    pub prelims_plus_playoff_stats: Option<PhaseStandings>,
    pub cumulative_stats: Option<AggregateStandings>,
}

impl Tournament {
    pub fn new() -> Self {
        Tournament { track_player_year: true, next_player_id: 1000, ..Default::default() }
    }

    pub fn names(&self) -> Names<'_> {
        Names { teams: &self.teams, players: &self.players }
    }

    pub fn new_player(&mut self, name: &str) -> PlayerId {
        let p = Player { name: name.to_string(), id_number: self.next_player_id, ..Default::default() };
        self.next_player_id += 1;
        self.players.push(p);
        self.players.len() - 1
    }

    pub fn new_team(&mut self, name: &str) -> TeamId {
        self.teams.push(Team { name: name.to_string(), ..Default::default() });
        self.teams.len() - 1
    }

    pub fn match_at(&self, loc: MatchLoc) -> &Match {
        &self.phases[loc.0].rounds[loc.1].matches[loc.2]
    }

    pub fn list_of_all_teams(&self) -> Vec<TeamId> {
        self.registrations.iter().flat_map(|r| r.teams.iter().copied()).collect()
    }

    pub fn find_team_by_name(&self, name: &str) -> Option<TeamId> {
        for reg in &self.registrations {
            if let Some(t) = reg.teams.iter().find(|t| self.teams[**t].name == name) {
                return Some(*t);
            }
        }
        None
    }

    pub fn find_team_by_id(&self, id: &str) -> Option<TeamId> {
        self.list_of_all_teams().into_iter().find(|t| self.teams[*t].id() == id)
    }

    pub fn find_registration(&self, name: &str) -> Option<usize> {
        self.registrations.iter().position(|r| r.name == name)
    }

    /// addRegistration: push, sort by localeCompare, seed its teams
    pub fn add_registration(&mut self, reg: Registration) {
        let new_teams = reg.teams.clone();
        self.registrations.push(reg);
        let mut regs = std::mem::take(&mut self.registrations);
        js::sort_by(&mut regs, |a, b| js::locale_compare(&a.name, &b.name));
        self.registrations = regs;
        // seedTeamsInRegistration for the registration we just added
        for t in new_teams {
            if !self.seeds.contains(&t) {
                self.seed_and_assign_new_team(t);
            }
        }
    }

    fn seed_and_assign_new_team(&mut self, t: TeamId) {
        self.seeds.push(t);
        let seed_no = self.seeds.len() as f64;
        let Some(pi) = self.prelim_phase() else { return };
        if self.using_schedule_template {
            // Phase.addSeededTeam
            let phase = &mut self.phases[pi];
            if let Some(pool) = phase.pools.iter_mut().find(|p| p.seeds.contains(&seed_no)) {
                pool.add_team(t);
            }
        } else {
            // Phase.addUnseededTeam
            let phase = &mut self.phases[pi];
            if let Some(pool) = phase.pools.iter_mut().find(|p| (p.pool_teams.len() as f64) < p.size) {
                pool.add_team(t);
            }
        }
    }

    pub fn prelim_phase(&self) -> Option<PhaseId> {
        self.phases.iter().position(|p| p.phase_type == PhaseType::Prelim)
    }

    pub fn full_phases(&self) -> Vec<PhaseId> {
        (0..self.phases.len()).filter(|i| self.phases[*i].is_full_phase()).collect()
    }

    pub fn prev_full_phase(&self, p: PhaseId) -> Option<PhaseId> {
        (0..p).rev().find(|i| self.phases[*i].is_full_phase())
    }

    pub fn last_full_phase(&self) -> Option<PhaseId> {
        (0..self.phases.len()).rev().find(|i| self.phases[*i].is_full_phase())
    }

    /// whichPhaseIsRoundNumberIn + find the round
    pub fn round_by_number(&self, n: f64) -> Option<(PhaseId, usize)> {
        let pi = self.phases.iter().position(|ph| ph.round_index_by_number(n).is_some())?;
        Some((pi, self.phases[pi].round_index_by_number(n)?))
    }

    pub fn find_phase_by_name(&self, name: &str) -> Option<PhaseId> {
        self.phases.iter().position(|p| p.name == name)
    }

    /// Tournament.findPoolWithTeam(team, round): pool in the phase that contains the round
    pub fn pool_with_team_in_phase(&self, t: TeamId, p: PhaseId) -> Option<usize> {
        self.phases[p].find_pool_with_team(t)
    }

    /// Matches in earlier full phases that carry over into this playoff phase
    pub fn carryover_matches(&self, playoff: PhaseId) -> Vec<MatchLoc> {
        if self.phases[playoff].phase_type != PhaseType::Playoff {
            return vec![];
        }
        let mut out = vec![];
        let mut cur = self.prev_full_phase(playoff);
        while let Some(c) = cur {
            for (ri, rd) in self.phases[c].rounds.iter().enumerate() {
                for (mi, m) in rd.matches.iter().enumerate() {
                    if m.carryover_phases.contains(&playoff) {
                        out.push((c, ri, mi));
                    }
                }
            }
            cur = self.prev_full_phase(c);
        }
        out
    }

    pub fn all_matches_in_phase(&self, p: PhaseId) -> Vec<MatchLoc> {
        let mut out = vec![];
        for (ri, rd) in self.phases[p].rounds.iter().enumerate() {
            for mi in 0..rd.matches.len() {
                out.push((p, ri, mi));
            }
        }
        out
    }

    /// `getNextFullPhase`: the next playoff phase after this one
    pub fn next_full_phase(&self, p: PhaseId) -> Option<PhaseId> {
        (p + 1..self.phases.len()).find(|i| self.phases[*i].phase_type == PhaseType::Playoff)
    }

    /// `getTiebreakerPhaseFor`: the phase right after this one, if it's a tiebreaker phase
    pub fn tiebreaker_phase_for(&self, p: PhaseId) -> Option<PhaseId> {
        let next = p + 1;
        (next < self.phases.len() && self.phases[next].phase_type == PhaseType::Tiebreaker).then_some(next)
    }

    pub fn finals_phases(&self) -> Vec<PhaseId> {
        (0..self.phases.len()).filter(|i| self.phases[*i].phase_type == PhaseType::Finals).collect()
    }

    /// `numberOfPhasesWithStats`: phases of any type with at least one match
    pub fn number_of_phases_with_stats(&self) -> usize {
        self.phases.iter().filter(|p| p.any_matches_exist()).count()
    }

    pub fn packet_names_exist(&self) -> bool {
        self.phases.iter().any(|p| p.packet_names_exist())
    }

    /// `hasAnyCarryover`: any full phase with a pool that carries over matches
    pub fn has_any_carryover(&self) -> bool {
        self.phases.iter().any(|p| p.is_full_phase() && p.has_any_carryover())
    }

    /// `findRegistrationByTeam`
    pub fn registration_for_team(&self, t: TeamId) -> Option<&Registration> {
        self.registrations.iter().find(|r| r.teams.contains(&t))
    }

    pub fn all_prelim_games_carry_over(&self) -> bool {
        let Some(pre) = self.prelim_phase() else { return false };
        if self.full_phases().len() != 2 {
            return false;
        }
        let pools = &self.phases[pre].pools;
        pools.len() == 1 && pools[0].round_robins >= 1.0
    }

    pub fn calc_has_match_data(&mut self) {
        self.has_match_data = self
            .prelim_phase()
            .is_some_and(|p| self.phases[p].rounds.iter().any(|r| !r.matches.is_empty()));
    }

    /// Set hasCarryover on pools of phases that matches were carried over to (non-YF qbj files)
    pub fn infer_carryover_status(&mut self) {
        for ph in self.full_phases() {
            let mut found: Vec<PhaseId> = vec![];
            for rd in &self.phases[ph].rounds {
                for m in &rd.matches {
                    for co in &m.carryover_phases {
                        if !found.contains(co) {
                            found.push(*co);
                        }
                    }
                }
            }
            for co in found {
                if self.phases[co].has_any_carryover() {
                    continue;
                }
                for p in &mut self.phases[co].pools {
                    p.has_carryover = true;
                }
            }
        }
    }

    /// Tournament.conversions(): fix-ups for older YF4 files
    pub fn conversions(&mut self) {
        if js::version_lt(&self.app_version, "4.0.1") {
            self.using_schedule_template = true;
        }
        if js::version_lt(&self.app_version, "4.0.12") {
            for ph in &mut self.phases {
                if !ph.is_full_phase() && ph.force_numeric_rounds == Some(true) {
                    for rd in &mut ph.rounds {
                        rd.name = None;
                    }
                }
            }
        }
    }

    /// Tournament.compileStats
    pub fn compile_stats(&mut self, full_report: bool, sort_by_final_rank: bool) {
        self.stats = vec![];
        let Some(last) = self.last_full_phase() else { return };
        let mut stats = vec![];
        for p in 0..self.phases.len() {
            if self.phases[p].is_full_phase() {
                stats.push(PhaseStandings::new(self, p, self.carryover_matches(p)));
            }
        }
        let n = stats.len();
        for (i, ps) in stats.iter_mut().enumerate() {
            ps.compile_stats(self, sort_by_final_rank && i == n - 1);
        }
        self.stats = stats;
        if self.all_prelim_games_carry_over() {
            let prelim_matches = self.prelim_phase().map(|p| self.all_matches_in_phase(p)).unwrap_or_default();
            let mut pp = PhaseStandings::new(self, last, prelim_matches);
            pp.compile_stats(self, sort_by_final_rank);
            self.prelims_plus_playoff_stats = Some(pp);
        }
        if full_report {
            let mut agg = AggregateStandings::new(self);
            if self.finals_rankings_ready {
                agg.arrange_teams_for_final_ranking(self);
            } else if self.scoring_rules.use_bonuses {
                agg.sort_teams_by_ppb();
            } else {
                agg.sort_teams_by_pptuh_with(&self.scoring_rules);
            }
            self.cumulative_stats = Some(agg);
            let mut stats = std::mem::take(&mut self.stats);
            for ps in &mut stats {
                ps.compile_individual_stats(self);
            }
            self.stats = stats;
        }
    }
}
