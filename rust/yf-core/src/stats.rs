//! Port of StatSummaries.ts. Operation order is kept identical to the TS so float results match bit for bit.

use crate::js;
use crate::model::*;
use crate::tournament::{MatchLoc, Tournament};

/// A PlayerAnswerCount as used in stats tallies (matched by point value, not identity)
#[derive(Clone, Debug)]
pub struct Count {
    pub value: f64,
    pub number: Option<f64>,
}

impl Count {
    fn points(&self) -> f64 {
        match self.number {
            None => 0.0,
            Some(n) => n * self.value,
        }
    }
}

fn add_answer_count(counts: &mut Vec<Count>, value: f64, number: Option<f64>) {
    match counts.iter_mut().find(|c| c.value == value) {
        None => counts.push(Count { value, number }),
        Some(tc) => match tc.number {
            None => tc.number = number,
            Some(x) => tc.number = Some(x + or0(number)),
        },
    }
}

fn sum(v: impl Iterator<Item = f64>) -> f64 {
    v.fold(0.0, |a, b| a + b)
}

#[derive(Clone, Debug)]
pub struct PoolTeamStats {
    pub team: TeamId,
    pub rank: String,
    pub raw_rank: f64,
    pub final_rank_calculated: Option<f64>,
    pub current_seed: Option<f64>,
    pub advance_to_tier: Option<f64>,
    pub record_tie_for_advancement: bool,
    pub ppg_tie_for_advancement: bool,
    pub wins: f64,
    pub losses: f64,
    pub ties: f64,
    pub non_forfeit_matches: f64,
    pub tuh_total: f64,
    pub tuh_regulation: f64,
    pub total_points: f64,
    pub total_points_for_ppg: f64,
    pub tossup_counts: Vec<Count>,
    pub bonuses_heard: f64,
    pub bonus_points: f64,
    pub bounce_back_points: f64,
    pub bounce_back_parts_heard: f64,
    pub bounce_back_parts_converted: f64,
    pub lightning_points: f64,
    /// Team Detail page entries: (match, side, phase)
    pub matches: Vec<(MatchLoc, Side)>,
}

impl PoolTeamStats {
    pub fn new(team: TeamId) -> Self {
        PoolTeamStats {
            team,
            rank: String::new(),
            raw_rank: 0.0,
            final_rank_calculated: None,
            current_seed: None,
            advance_to_tier: None,
            record_tie_for_advancement: false,
            ppg_tie_for_advancement: false,
            wins: 0.0,
            losses: 0.0,
            ties: 0.0,
            non_forfeit_matches: 0.0,
            tuh_total: 0.0,
            tuh_regulation: 0.0,
            total_points: 0.0,
            total_points_for_ppg: 0.0,
            tossup_counts: vec![],
            bonuses_heard: 0.0,
            bonus_points: 0.0,
            bounce_back_points: 0.0,
            bounce_back_parts_heard: 0.0,
            bounce_back_parts_converted: 0.0,
            lightning_points: 0.0,
            matches: vec![],
        }
    }

    pub fn record(&self) -> String {
        let wl = format!("{}-{}", js::num_to_string(self.wins), js::num_to_string(self.losses));
        if self.ties == 0.0 {
            return wl;
        }
        format!("{wl}-{}", js::num_to_string(self.ties))
    }
    pub fn win_pct(&self) -> f64 {
        let wins = self.wins + self.ties / 2.0;
        wins / (self.wins + self.losses + self.ties)
    }
    pub fn win_pct_string(&self) -> String {
        let p = self.win_pct();
        if p.is_nan() {
            return "-".into();
        }
        js::to_fixed(p, 3)
    }
    pub fn correct_tuh(&self, r: &ScoringRules) -> f64 {
        if r.use_overtime_in_pptuh() {
            self.tuh_total
        } else {
            self.tuh_regulation
        }
    }
    pub fn pts_per_reg_tuh(&self, r: &ScoringRules) -> f64 {
        self.total_points_for_ppg / self.correct_tuh(r)
    }
    pub fn pts_per_reg_tuh_string(&self, r: &ScoringRules, reg_tu: f64) -> String {
        if self.total_points_for_ppg == 0.0 {
            return "-".into();
        }
        js::to_fixed(self.pts_per_reg_tuh(r) * reg_tu, 1)
    }
    pub fn pts_per_bonus(&self) -> f64 {
        self.bonus_points / self.bonuses_heard
    }
    pub fn pts_per_bonus_string(&self) -> String {
        let p = self.pts_per_bonus();
        if p.is_nan() {
            return "-".into();
        }
        js::to_fixed(p, 2)
    }
    pub fn bounceback_conv_pct(&self, r: &ScoringRules) -> f64 {
        if !r.can_calculate_bounce_back_parts_heard() {
            return f64::NAN;
        }
        js::round((100.0 * self.bounce_back_parts_converted) / self.bounce_back_parts_heard)
    }
    pub fn bounceback_conv_pct_string(&self, r: &ScoringRules) -> String {
        let v = self.bounceback_conv_pct(r);
        if v.is_nan() {
            return "-".into();
        }
        format!("{}%", js::num_to_string(v))
    }
    pub fn lightning_pts_per_match_string(&self) -> String {
        let v = self.lightning_points / self.non_forfeit_matches;
        if v.is_nan() {
            return "-".into();
        }
        js::to_fixed(v, 1)
    }

    fn needs_tiebreaker_with(&self, other: &PoolTeamStats) -> bool {
        if self.rank != other.rank {
            return false;
        }
        if self.advance_to_tier.is_none() && other.advance_to_tier.is_none() {
            return false;
        }
        self.advance_to_tier != other.advance_to_tier || other.record_tie_for_advancement
    }

    fn add_match_team(&mut self, t: &Tournament, loc: MatchLoc, side: Side, record_match: bool) {
        let m = t.match_at(loc);
        let rules = &t.scoring_rules;
        if record_match {
            self.matches.push((loc, side));
        }
        self.tuh_total += or0(m.tossups_read);
        self.tuh_regulation += or0(m.tossups_read) - or0(m.overtime_tossups_read);
        match m.result(side) {
            Some(Wlt::Win) => self.wins += 1.0,
            Some(Wlt::Loss) => self.losses += 1.0,
            Some(Wlt::Tie) => self.ties += 1.0,
            None => {}
        }
        if m.is_forfeit() {
            return;
        }
        self.non_forfeit_matches += 1.0;
        let mt = m.side(side);
        self.total_points += or0(mt.points);
        self.total_points_for_ppg += mt.points_for_ppg(rules);
        self.bonus_points += mt.bonus_points();
        self.bonuses_heard += mt.bonuses_heard(rules);
        if rules.bonuses_bounce_back {
            let ppbp = or0(rules.points_per_bonus_part);
            let ppbp = if ppbp == 0.0 { 10.0 } else { ppbp };
            self.bounce_back_parts_converted += or0(mt.bonus_bounceback_points) / ppbp;
            self.bounce_back_parts_heard += or0(Some(m.bounceback_parts_heard(side, rules)));
            self.bounce_back_points += or0(mt.bonus_bounceback_points);
        }
        self.lightning_points += or0(mt.lightning_points);
        for mp in &mt.match_players {
            for ac in &mp.answer_counts {
                if ac.number.is_some() {
                    add_answer_count(&mut self.tossup_counts, ac.value, ac.number);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub player: PlayerId,
    pub team: TeamId,
    pub rank: String,
    pub rank_tie: bool,
    pub games_played: f64,
    pub tossup_counts: Vec<Count>,
    pub tossups_heard: f64,
    /// Player Detail page entries: (match, side, index of the MatchPlayer in that side's matchPlayers)
    pub matches: Vec<(MatchLoc, Side, usize)>,
}

impl PlayerStats {
    fn new(player: PlayerId, team: TeamId, rules: &ScoringRules) -> Self {
        PlayerStats {
            player,
            team,
            rank: String::new(),
            rank_tie: false,
            games_played: 0.0,
            tossup_counts: rules.answer_types.iter().map(|a| Count { value: a.value, number: Some(0.0) }).collect(),
            tossups_heard: 0.0,
            matches: vec![],
        }
    }
    pub fn total_points(&self) -> f64 {
        sum(self.tossup_counts.iter().map(|c| c.points()))
    }
    pub fn pptuh(&self) -> Option<f64> {
        if self.tossups_heard == 0.0 {
            return None;
        }
        Some(self.total_points() / self.tossups_heard)
    }
    fn add_match_player(&mut self, mp: &MatchPlayer, tossups_read: f64) {
        let tuh = match mp.tossups_heard {
            None => return,
            Some(x) if x == 0.0 => return,
            Some(x) => x,
        };
        self.games_played += tuh / tossups_read;
        self.tossups_heard += tuh;
        for ac in &mp.answer_counts {
            add_answer_count(&mut self.tossup_counts, ac.value, ac.number);
        }
    }
}

fn sort_players_by_pptuh(players: &mut Vec<PlayerStats>) {
    js::sort_by(players, |a, b| {
        let ap = a.pptuh().unwrap_or(-999999.0);
        let bp = b.pptuh().unwrap_or(-999999.0);
        if ap == bp {
            if ap > 0.0 {
                return b.tossups_heard - a.tossups_heard;
            }
            return a.tossups_heard - b.tossups_heard;
        }
        bp - ap
    });
}

fn find_player(players: &[PlayerStats], p: PlayerId) -> Option<usize> {
    players.iter().position(|s| s.player == p)
}

fn ppb_compare(a: &PoolTeamStats, b: &PoolTeamStats) -> f64 {
    let mut ap = a.pts_per_bonus();
    let mut bp = b.pts_per_bonus();
    if ap.is_nan() {
        ap = -9999999.0;
    }
    if bp.is_nan() {
        bp = -9999999.0;
    }
    bp - ap
}

#[derive(Clone, Debug)]
pub struct RoundStats {
    pub round_number: f64,
    /// index of the round in its phase's rounds (None for the tournament total)
    pub round: Option<usize>,
    pub phase: Option<PhaseId>,
    pub games: f64,
    pub tossups_heard_total: f64,
    pub tossups_heard_regulation: f64,
    pub points: f64,
    pub tossup_counts: Vec<Count>,
    pub bonus_points: f64,
    pub lightning_points: f64,
    pub bounceback_parts_converted: f64,
}

impl RoundStats {
    fn new(round_number: f64, phase: Option<PhaseId>, round: Option<usize>) -> Self {
        RoundStats {
            round_number,
            round,
            phase,
            games: 0.0,
            tossups_heard_total: 0.0,
            tossups_heard_regulation: 0.0,
            points: 0.0,
            tossup_counts: vec![],
            bonus_points: 0.0,
            lightning_points: 0.0,
            bounceback_parts_converted: 0.0,
        }
    }
    fn bonuses_heard(&self) -> f64 {
        sum(self.tossup_counts.iter().map(|c| if c.value > 0.0 { or0(c.number) } else { 0.0 }))
    }
    fn bonus_parts_heard(&self, r: &ScoringRules) -> f64 {
        if !r.bonuses_are_regular() {
            return f64::NAN;
        }
        self.bonuses_heard() * r.maximum_parts_per_bonus
    }
    fn bonus_parts_conv_controlling(&self, r: &ScoringRules) -> f64 {
        if !r.bonuses_are_regular() {
            return f64::NAN;
        }
        self.bonus_points / r.points_per_bonus_part.unwrap_or(10.0)
    }
    fn bounceback_parts_heard(&self, r: &ScoringRules) -> f64 {
        if !r.bonuses_are_regular() {
            return f64::NAN;
        }
        self.bonus_parts_heard(r) - self.bonus_parts_conv_controlling(r)
    }
    fn tuh_for_pptuh(&self, r: &ScoringRules) -> f64 {
        if r.use_overtime_in_pptuh() {
            self.tossups_heard_total
        } else {
            self.tossups_heard_regulation
        }
    }
    /// [pointsPerXTuh, powerPct, tossupConvPct, negsPerXTuh, ppb, bbConvPct, totalBonusConvPct, lightningPerTeamPerMatch]
    pub fn derived(&self, r: &ScoringRules) -> [f64; 8] {
        let reg = r.regulation_tossup_count();
        let total_powers = sum(self.tossup_counts.iter().map(|c| if c.value > 10.0 { nn0(c.number) } else { 0.0 }));
        let total_tus = sum(self.tossup_counts.iter().map(|c| if c.value > 0.0 { nn0(c.number) } else { 0.0 }));
        let total_negs = sum(self.tossup_counts.iter().map(|c| if c.value < 0.0 { nn0(c.number) } else { 0.0 }));
        [
            (reg * self.points) / self.tuh_for_pptuh(r) / 2.0,
            (100.0 * total_powers) / self.tossups_heard_total,
            (100.0 * total_tus) / self.tossups_heard_total,
            (reg * total_negs) / self.tuh_for_pptuh(r) / 2.0,
            self.bonus_points / self.bonuses_heard(),
            (100.0 * self.bounceback_parts_converted) / self.bounceback_parts_heard(r),
            (100.0 * (self.bonus_parts_conv_controlling(r) + self.bounceback_parts_converted)) / self.bonus_parts_heard(r),
            self.lightning_points / self.games / 2.0,
        ]
    }
    fn add_match(&mut self, m: &Match, r: &ScoringRules) {
        if m.is_forfeit() {
            return;
        }
        self.games += 1.0;
        self.tossups_heard_total += nn0(m.tossups_read);
        self.tossups_heard_regulation += match m.tossups_read {
            None => 0.0,
            Some(t) => t - nn0(m.overtime_tossups_read),
        };
        self.points += nn0(m.left.points);
        self.points += nn0(m.right.points);
        self.lightning_points += nn0(m.left.lightning_points);
        self.lightning_points += nn0(m.right.lightning_points);
        for mt in [&m.left, &m.right] {
            for mp in &mt.match_players {
                for ac in &mp.answer_counts {
                    if ac.number.is_some() {
                        add_answer_count(&mut self.tossup_counts, ac.value, ac.number);
                    }
                }
            }
        }
        self.bonus_points += m.left.bonus_points();
        self.bonus_points += m.right.bonus_points();
        if r.bonuses_are_regular() {
            let d = r.points_per_bonus_part.unwrap_or(0.0);
            self.bounceback_parts_converted += nn0(m.left.bonus_bounceback_points) / d;
            self.bounceback_parts_converted += nn0(m.right.bonus_bounceback_points) / d;
        }
    }
}

#[derive(Clone, Debug)]
pub struct AggregateStandings {
    pub team_stats: Vec<PoolTeamStats>,
    pub players: Vec<PlayerStats>,
    pub rounds: Vec<RoundStats>,
    pub total: RoundStats,
    pub any_ties_exist: bool,
}

impl AggregateStandings {
    pub fn new(t: &Tournament) -> Self {
        let rules = &t.scoring_rules;
        let mut agg = AggregateStandings {
            team_stats: t.list_of_all_teams().into_iter().map(PoolTeamStats::new).collect(),
            players: vec![],
            rounds: vec![],
            total: RoundStats::new(-1.0, None, None),
            any_ties_exist: false,
        };
        for (pi, phase) in t.phases.iter().enumerate() {
            for (ri, round) in phase.rounds.iter().enumerate() {
                let mut rs = RoundStats::new(round.number, Some(pi), Some(ri));
                for (mi, m) in round.matches.iter().enumerate() {
                    if m.omit {
                        continue;
                    }
                    let loc = (pi, ri, mi);
                    for side in [Side::Left, Side::Right] {
                        if let Some(team) = m.side(side).team {
                            if let Some(ts) = agg.team_stats.iter_mut().find(|s| s.team == team) {
                                ts.add_match_team(t, loc, side, true);
                            }
                        }
                    }
                    for side in [Side::Left, Side::Right] {
                        let mt = m.side(side);
                        let tossups_read = nn0(m.tossups_read);
                        let Some(team) = mt.team else { continue };
                        for (mpi, mp) in mt.match_players.iter().enumerate() {
                            let i = match find_player(&agg.players, mp.player) {
                                Some(i) => i,
                                None => {
                                    agg.players.push(PlayerStats::new(mp.player, team, rules));
                                    agg.players.len() - 1
                                }
                            };
                            let ps = &mut agg.players[i];
                            ps.add_match_player(mp, tossups_read);
                            // addMatchResult
                            if mp.tossups_heard.is_some_and(|x| x != 0.0) {
                                ps.matches.push((loc, side, mpi));
                            }
                        }
                    }
                    rs.add_match(m, rules);
                    agg.total.add_match(m, rules);
                }
                if rs.games > 0.0 {
                    agg.rounds.push(rs);
                }
            }
        }
        agg.any_ties_exist = agg.team_stats.iter().any(|s| s.ties > 0.0);
        sort_players_by_pptuh(&mut agg.players);
        agg
    }

    pub fn sort_teams_by_ppb(&mut self) {
        js::sort_by(&mut self.team_stats, ppb_compare);
    }

    pub fn sort_teams_by_pptuh_with(&mut self, r: &ScoringRules) {
        js::sort_by(&mut self.team_stats, |a, b| {
            let mut ap = a.pts_per_reg_tuh(r);
            let mut bp = b.pts_per_reg_tuh(r);
            if ap.is_nan() {
                ap = -9999999.0;
            }
            if bp.is_nan() {
                bp = -9999999.0;
            }
            bp - ap
        });
    }

    pub fn arrange_teams_for_final_ranking(&mut self, t: &Tournament) {
        js::sort_by(&mut self.team_stats, |a, b| {
            let ar = t.teams[a.team].overall_rank_or(9999.0);
            let br = t.teams[b.team].overall_rank_or(9999.0);
            if ar != br {
                return ar - br;
            }
            ppb_compare(a, b)
        });
        let mut last_rank = 0.0;
        for i in 0..self.team_stats.len() {
            let Some(r) = t.teams[self.team_stats[i].team].overall_rank else { continue };
            if r == last_rank {
                let s = format!("{}=", js::num_to_string(r));
                self.team_stats[i].rank = s.clone();
                self.team_stats[i - 1].rank = s;
            } else {
                self.team_stats[i].rank = js::num_to_string(r);
            }
            last_rank = r;
        }
    }
}

#[derive(Clone, Debug)]
pub struct PoolStats {
    pub pool: Pool,
    pub pool_teams: Vec<PoolTeamStats>,
}

impl PoolStats {
    fn sort_teams(&mut self, r: &ScoringRules) {
        js::sort_by(&mut self.pool_teams, |a, b| {
            let mut aw = a.win_pct();
            let mut bw = b.win_pct();
            if aw.is_nan() {
                aw = -1.0;
            }
            if bw.is_nan() {
                bw = -1.0;
            }
            if aw != bw {
                return bw - aw;
            }
            let mut ap = a.pts_per_reg_tuh(r);
            let mut bp = b.pts_per_reg_tuh(r);
            if ap.is_nan() {
                ap = -9999999.0;
            }
            if bp.is_nan() {
                bp = -9999999.0;
            }
            bp - ap
        });
    }

    fn rank_sorted_teams(&mut self, r: &ScoringRules) {
        let starting_rank = 1.0;
        let mut prev_win_pct = 2.0;
        let mut teams_so_far = 0.0;
        let mut prev_rank = starting_rank - 1.0;
        for i in 0..self.pool_teams.len() {
            teams_so_far += 1.0;
            self.pool_teams[i].raw_rank = teams_so_far;
            let this_wp = self.pool_teams[i].win_pct();
            if this_wp == prev_win_pct || (this_wp.is_nan() && f64::is_nan(prev_win_pct)) {
                let tied = format!("{}=", js::num_to_string(prev_rank));
                self.pool_teams[i].rank = tied.clone();
                if i > 0 {
                    self.pool_teams[i - 1].rank = tied;
                }
            } else {
                self.pool_teams[i].rank = js::num_to_string(teams_so_far);
                prev_rank = teams_so_far;
            }
            self.pool_teams[i].advance_to_tier = self.pool.tier_that_rank_advances_to(teams_so_far);
            self.pool_teams[i].current_seed = self.pool.seed_for_rank(teams_so_far);
            prev_win_pct = this_wp;
        }
        self.advancement_tiers_handle_ties(r);
    }

    fn advancement_tiers_handle_ties(&mut self, r: &ScoringRules) {
        let n = self.pool_teams.len();
        if n == 0 {
            return;
        }
        let mut prev = 0;
        for i in 1..n {
            if self.pool_teams[i].needs_tiebreaker_with(&self.pool_teams[prev]) {
                let same_ppg = self.pool_teams[i].pts_per_reg_tuh(r) == self.pool_teams[prev].pts_per_reg_tuh(r);
                self.pool_teams[i].record_tie_for_advancement = true;
                self.pool_teams[i].ppg_tie_for_advancement = same_ppg;
            }
            prev = i;
        }
        let mut prev = n - 1;
        for i in (0..n.saturating_sub(1)).rev() {
            if self.pool_teams[i].needs_tiebreaker_with(&self.pool_teams[prev]) {
                let same_ppg = self.pool_teams[i].pts_per_reg_tuh(r) == self.pool_teams[prev].pts_per_reg_tuh(r);
                self.pool_teams[i].record_tie_for_advancement = true;
                self.pool_teams[i].ppg_tie_for_advancement = same_ppg;
            }
            prev = i;
        }
    }

    fn find_team(&mut self, t: TeamId) -> Option<&mut PoolTeamStats> {
        self.pool_teams.iter_mut().find(|pt| pt.team == t)
    }

    fn any_ties(&self) -> bool {
        self.pool_teams.iter().any(|pt| pt.ties > 0.0)
    }

    fn calculate_final_ranks(&mut self, starting_rank: f64, step: f64) {
        if self.pool_teams.is_empty() {
            return;
        }
        self.pool_teams[0].final_rank_calculated = Some(starting_rank);
        let mut teams_so_far = step;
        for i in 1..self.pool_teams.len() {
            if self.pool_teams[i - 1].rank == self.pool_teams[i].rank {
                self.pool_teams[i].final_rank_calculated = self.pool_teams[i - 1].final_rank_calculated;
            } else {
                self.pool_teams[i].final_rank_calculated = Some(starting_rank + teams_so_far);
            }
            teams_so_far += step;
        }
    }

    fn sort_by_final_rank(&mut self, t: &Tournament) {
        js::sort_by(&mut self.pool_teams, |a, b| {
            let key = |s: &PoolTeamStats| {
                let r = t.teams[s.team].overall_rank_or(0.0);
                if r != 0.0 {
                    return r;
                }
                match s.final_rank_calculated {
                    Some(f) if f != 0.0 && !f.is_nan() => f,
                    _ => 9999.0,
                }
            };
            key(a) - key(b)
        });
    }
}

#[derive(Clone, Debug)]
pub struct PhaseStandings {
    pub phase: PhaseId,
    pub pools: Vec<PoolStats>,
    pub players: Vec<PlayerStats>,
    pub carryover_matches: Vec<MatchLoc>,
    pub any_ties_exist: bool,
}

impl PhaseStandings {
    pub fn new(t: &Tournament, phase: PhaseId, carryover: Vec<MatchLoc>) -> Self {
        let pools = t.phases[phase]
            .pools
            .iter()
            .map(|p| PoolStats {
                pool: p.clone(),
                pool_teams: p.pool_teams.iter().map(|pt| PoolTeamStats::new(pt.team)).collect(),
            })
            .collect();
        PhaseStandings { phase, pools, players: vec![], carryover_matches: carryover, any_ties_exist: false }
    }

    fn phase_matches(&self, t: &Tournament) -> Vec<MatchLoc> {
        let mut v = t.all_matches_in_phase(self.phase);
        v.extend(self.carryover_matches.iter().copied());
        v
    }

    pub fn compile_stats(&mut self, t: &Tournament, sort_by_final_rank: bool) {
        let rules = &t.scoring_rules;
        for loc in self.phase_matches(t) {
            let m = t.match_at(loc);
            if m.omit {
                continue;
            }
            for side in [Side::Left, Side::Right] {
                let Some(team) = m.side(side).team else { continue };
                for pool in &mut self.pools {
                    if let Some(pt) = pool.find_team(team) {
                        pt.add_match_team(t, loc, side, false);
                        break;
                    }
                }
            }
        }
        for pool in &mut self.pools {
            pool.sort_teams(rules);
            pool.rank_sorted_teams(rules);
            if !self.any_ties_exist && pool.any_ties() {
                self.any_ties_exist = true;
            }
        }
        self.assign_wild_card_seeds(t);
        if sort_by_final_rank {
            self.assign_final_team_ranks();
            for p in &mut self.pools {
                p.sort_by_final_rank(t);
            }
        }
    }

    fn assign_wild_card_seeds(&mut self, t: &Tournament) {
        let phase = &t.phases[self.phase];
        if !phase.has_wild_cards() {
            return;
        }
        let mut cur_seed = phase.top_wild_card_seed();
        // collect references to teams to seed, in pool order
        let mut refs: Vec<(usize, usize)> = vec![];
        for (pi, pool) in self.pools.iter().enumerate() {
            for (ti, pt) in pool.pool_teams.iter().enumerate() {
                let ok = matches!(pt.current_seed, Some(s) if s != 0.0 && !s.is_nan())
                    && cur_seed != 0.0
                    && pt.current_seed.unwrap() >= cur_seed;
                if ok {
                    refs.push((pi, ti));
                }
            }
        }
        let pools = &self.pools;
        js::sort_by(&mut refs, |a, b| {
            let a = &pools[a.0].pool_teams[a.1];
            let b = &pools[b.0].pool_teams[b.1];
            match phase.wild_card_ranking_method {
                WildCardRankingMethod::RankThenPpb => {
                    let (ar, br) = (a.raw_rank, b.raw_rank);
                    if ar != br {
                        return ar - br;
                    }
                }
                WildCardRankingMethod::RecordThenPpb => {
                    let mut aw = a.win_pct();
                    let mut bw = b.win_pct();
                    if aw.is_nan() {
                        aw = -1.0;
                    }
                    if bw.is_nan() {
                        bw = -1.0;
                    }
                    if aw != bw {
                        return bw - aw;
                    }
                }
            }
            ppb_compare(a, b)
        });
        for (pi, ti) in refs {
            let tier = phase.tier_that_wc_seed_advances_to(cur_seed);
            let pt = &mut self.pools[pi].pool_teams[ti];
            pt.current_seed = Some(cur_seed);
            pt.advance_to_tier = tier;
            cur_seed += 1.0;
        }
    }

    fn assign_final_team_ranks(&mut self) {
        let mut teams_so_far = 0.0;
        let mut tier = 1.0;
        loop {
            let in_tier: Vec<usize> = (0..self.pools.len()).filter(|i| self.pools[*i].pool.position == tier).collect();
            if in_tier.is_empty() {
                break;
            }
            let mut teams_in_tier = 0.0;
            let n = in_tier.len() as f64;
            for i in in_tier {
                self.pools[i].calculate_final_ranks(teams_so_far + 1.0, n);
                teams_in_tier += self.pools[i].pool_teams.len() as f64;
            }
            teams_so_far += teams_in_tier;
            tier += 1.0;
        }
    }

    pub fn compile_individual_stats(&mut self, t: &Tournament) {
        let rules = &t.scoring_rules;
        for loc in self.phase_matches(t) {
            let m = t.match_at(loc);
            if m.omit {
                continue;
            }
            let tossups_read = nn0(m.tossups_read);
            for mt in [&m.left, &m.right] {
                let Some(team) = mt.team else { continue };
                for mp in &mt.match_players {
                    let i = match find_player(&self.players, mp.player) {
                        Some(i) => i,
                        None => {
                            self.players.push(PlayerStats::new(mp.player, team, rules));
                            self.players.len() - 1
                        }
                    };
                    self.players[i].add_match_player(mp, tossups_read);
                }
            }
        }
        sort_players_by_pptuh(&mut self.players);
        // assignIndividualRanks
        let mut prev_rank = 0.0;
        let mut so_far = 0.0;
        let mut prev_pptuh = 9999999.0;
        for i in 0..self.players.len() {
            so_far += 1.0;
            let cur = self.players[i].pptuh().unwrap_or(-999999.0);
            if cur == prev_pptuh {
                self.players[i].rank = js::num_to_string(prev_rank);
                self.players[i].rank_tie = true;
                if i > 0 {
                    self.players[i - 1].rank_tie = true;
                }
            } else {
                self.players[i].rank = js::num_to_string(so_far);
                prev_rank = so_far;
            }
            prev_pptuh = cur;
        }
    }
}
