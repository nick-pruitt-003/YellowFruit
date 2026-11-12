//! The read-only pages. Each mirrors a page of the TS app (`src/renderer/Components/`), showing
//! values computed by yf-core so the numbers match the app's.

use eframe::egui::{self, Color32, RichText, Ui};
use yf_core::js;
use yf_core::model::{Match, PhaseType};
use yf_core::stats::PhaseStandings;
use yf_core::Tournament;

use crate::OpenFile;

/// YellowFruit yellow (the `primary` colour in `src/renderer/YfTheme.ts`)
const YELLOW: Color32 = Color32::from_rgb(0xFF, 0xC1, 0x07);

pub fn apply_theme(ctx: &egui::Context) {
    // the TS app uses a light theme only
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| {
        s.visuals.selection.bg_fill = YELLOW;
        s.visuals.selection.stroke.color = Color32::BLACK;
        s.visuals.hyperlink_color = Color32::from_rgb(0xB2, 0x87, 0x04);
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
    });
}

fn card(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    egui::Frame::group(ui.style()).inner_margin(12.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(title).heading());
        ui.add_space(4.0);
        body(ui);
    });
    ui.add_space(10.0);
}

/// A right-aligned cell of fixed width, so number columns line up without stretching the grid
fn num_cell(ui: &mut Ui, text: impl Into<egui::WidgetText>) {
    let h = ui.spacing().interact_size.y;
    ui.allocate_ui_with_layout(egui::vec2(52.0, h), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

fn or_dash(s: &str) -> &str {
    if s.trim().is_empty() {
        "—"
    } else {
        s
    }
}

// ---------- General ----------

pub fn general(ui: &mut Ui, f: &OpenFile) {
    let t = &f.tournament;
    card(ui, "Tournament", |ui| {
        egui::Grid::new("general").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
            let row = |ui: &mut Ui, k: &str, v: &str| {
                ui.label(RichText::new(k).strong());
                ui.label(or_dash(v));
                ui.end_row();
            };
            row(ui, "Name", &t.name);
            row(ui, "Site", &t.site_name);
            row(ui, "Question set", &t.question_set);
            row(ui, "Start date", date_only(t.start_date.as_deref()));
            row(ui, "End date", date_only(t.end_date.as_deref()));
            row(ui, "Teams", &t.teams.len().to_string());
            row(ui, "Games", &match_count(t).to_string());
        });
    });
    let r = &t.scoring_rules;
    card(ui, "Rules", |ui| {
        egui::Grid::new("rules").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
            let row = |ui: &mut Ui, k: &str, v: String| {
                ui.label(RichText::new(k).strong());
                ui.label(v);
                ui.end_row();
            };
            row(ui, "Rule set", or_dash(t.standard_rule_set.as_deref().unwrap_or(&r.name)).to_string());
            row(
                ui,
                "Tossups",
                if r.timed { "Timed rounds".into() } else { format!("{} per game", js::num_to_string(r.maximum_regulation_tossup_count)) },
            );
            let values: Vec<String> = r.answer_types.iter().map(|a| a.label()).collect();
            row(ui, "Tossup values", values.join(", "));
            row(
                ui,
                "Bonuses",
                if !r.use_bonuses {
                    "None".into()
                } else if r.bonuses_bounce_back {
                    "Yes, with bouncebacks".into()
                } else {
                    "Yes".into()
                },
            );
            row(ui, "Players per team", js::num_to_string(r.maximum_players_per_team));
            if r.lightning_count_per_team > 0.0 {
                row(ui, "Lightning rounds", js::num_to_string(r.lightning_count_per_team));
            }
        });
    });
    card(ui, "Stages", |ui| {
        if t.phases.is_empty() {
            ui.label("No stages");
        }
        egui::Grid::new("stages").num_columns(4).striped(true).spacing([24.0, 4.0]).show(ui, |ui| {
            for h in ["Stage", "Type", "Rounds", "Pools"] {
                ui.label(RichText::new(h).strong());
            }
            ui.end_row();
            for p in &t.phases {
                ui.label(&p.name);
                ui.label(p.phase_type.as_str());
                ui.label(match (p.rounds.first(), p.rounds.last()) {
                    (Some(a), Some(b)) if a.number == b.number => js::num_to_string(a.number),
                    (Some(a), Some(b)) => format!("{}–{}", js::num_to_string(a.number), js::num_to_string(b.number)),
                    _ => "—".into(),
                });
                ui.label(p.pools.len().to_string());
                ui.end_row();
            }
        });
    });
}

/// "2025-12-14T05:00:00.000Z" -> "2025-12-14"
fn date_only(d: Option<&str>) -> &str {
    let d = d.unwrap_or("");
    d.get(..10).filter(|x| x.as_bytes().get(4) == Some(&b'-')).unwrap_or(d)
}

fn match_count(t: &Tournament) -> usize {
    t.phases.iter().flat_map(|p| &p.rounds).map(|r| r.matches.len()).sum()
}

// ---------- Teams ----------

pub fn teams(ui: &mut Ui, t: &Tournament) {
    card(ui, &format!("Teams ({})", t.teams.len()), |ui| {
        let any_ss = t.registrations.iter().any(|r| r.is_small_school);
        egui::Grid::new("teams").num_columns(3).striped(true).spacing([24.0, 4.0]).show(ui, |ui| {
            ui.label(RichText::new("Team").strong());
            ui.label(RichText::new("Players").strong());
            ui.label(RichText::new("").strong());
            ui.end_row();
            for reg in &t.registrations {
                for &tid in &reg.teams {
                    let team = &t.teams[tid];
                    ui.label(&team.name);
                    let names: Vec<&str> = team.players.iter().map(|&p| t.players[p].name.as_str()).collect();
                    ui.label(names.join(", "));
                    let mut tags = vec![];
                    if any_ss && reg.is_small_school {
                        tags.push("SS");
                    }
                    if team.is_jv {
                        tags.push("JV");
                    }
                    if team.is_ug {
                        tags.push("UG");
                    }
                    if team.is_d2 {
                        tags.push("D2");
                    }
                    ui.label(tags.join(" "));
                    ui.end_row();
                }
            }
        });
    });
}

// ---------- Standings (TeamsPageStandingsView) ----------

pub fn standings(ui: &mut Ui, t: &Tournament) {
    let full = t.full_phases();
    if full.is_empty() {
        ui.label("No stages, so no standings.");
        return;
    }
    let r = &t.scoring_rules;
    let reg_tu = r.regulation_tossup_count();
    for p in full {
        let next = t.next_full_phase(p);
        let stats: Option<&PhaseStandings> = if next.is_none() && t.all_prelim_games_carry_over() {
            t.prelims_plus_playoff_stats.as_ref()
        } else {
            t.stats.iter().find(|s| s.phase == p)
        };
        let Some(stats) = stats else { continue };
        card(ui, &t.phases[p].name, |ui| {
            for (pi, pool) in stats.pools.iter().enumerate() {
                let mut headers = vec!["", pool.pool.name.as_str(), "W", "L"];
                if stats.any_ties_exist {
                    headers.push("T");
                }
                headers.push("Pct");
                let pp = format!("PP{}", js::num_to_string(reg_tu));
                headers.push(&pp);
                if r.use_bonuses {
                    headers.push("PPB");
                }
                let rank_col = if next.is_none() {
                    Some("Final Rank")
                } else if t.using_schedule_template {
                    Some("Seed")
                } else {
                    None
                };
                headers.extend(rank_col);
                if next.is_some() {
                    headers.push("Advance To");
                }
                egui::Grid::new(("pool", p, pi)).num_columns(headers.len()).striped(true).spacing([20.0, 4.0]).show(ui, |ui| {
                    for (i, h) in headers.iter().enumerate() {
                        let txt = RichText::new(*h).strong();
                        if i >= 2 && !matches!(*h, "Advance To" | "Final Rank" | "Seed") {
                            num_cell(ui, txt);
                        } else {
                            ui.label(txt);
                        }
                    }
                    ui.end_row();
                    for pt in &pool.pool_teams {
                        let num = |ui: &mut Ui, s: String| num_cell(ui, s);
                        ui.label(&pt.rank);
                        ui.label(&t.teams[pt.team].name);
                        num(ui, js::num_to_string(pt.wins));
                        num(ui, js::num_to_string(pt.losses));
                        if stats.any_ties_exist {
                            num(ui, js::num_to_string(pt.ties));
                        }
                        num(ui, pt.win_pct_string());
                        num(ui, pt.pts_per_reg_tuh_string(r, reg_tu));
                        if r.use_bonuses {
                            num(ui, pt.pts_per_bonus_string());
                        }
                        match (next, rank_col) {
                            (None, Some(_)) => {
                                let rank = t.teams[pt.team].overall_rank.filter(|x| *x != 0.0).or(pt.final_rank_calculated);
                                ui.label(rank.map(js::num_to_string).unwrap_or_default());
                            }
                            (Some(_), Some(_)) => {
                                ui.label(pt.current_seed.map(js::num_to_string).unwrap_or_default());
                            }
                            _ => {}
                        }
                        if let Some(np) = next {
                            let pool_name = t.pool_with_team_in_phase(pt.team, np).map(|i| t.phases[np].pools[i].name.clone());
                            ui.label(pool_name.unwrap_or_default());
                        }
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
            }
        });
    }
}

// ---------- Games ----------

pub fn games(ui: &mut Ui, t: &Tournament) {
    let names = t.names();
    let total = match_count(t);
    if total == 0 {
        ui.label("No games entered.");
        return;
    }
    for phase in &t.phases {
        if phase.rounds.iter().all(|r| r.matches.is_empty()) {
            continue;
        }
        let title = if phase.phase_type == PhaseType::Prelim || phase.phase_type == PhaseType::Playoff {
            phase.name.clone()
        } else {
            format!("{} ({})", phase.name, phase.phase_type.as_str())
        };
        card(ui, &title, |ui| {
            for round in &phase.rounds {
                if round.matches.is_empty() {
                    continue;
                }
                let force = phase.force_numeric_rounds.unwrap_or(false);
                egui::CollapsingHeader::new(format!("{} ({} games)", round.display_name(force), round.matches.len()))
                    .id_salt((&phase.name, round.number.to_bits()))
                    .default_open(true)
                    .show(ui, |ui| {
                        for m in &round.matches {
                            match_row(ui, t, m, &names);
                        }
                    });
            }
        });
    }
}

fn match_row(ui: &mut Ui, t: &Tournament, m: &Match, names: &yf_core::model::Names) {
    let errors = m.error_messages(true, names);
    let warnings = m.warning_messages(names);
    ui.horizontal(|ui| {
        // a coloured dot: egui's emoji glyphs for check/cross ignore text colour
        let (color, what) = if !errors.is_empty() {
            (Color32::from_rgb(0xC6, 0x28, 0x28), "Error")
        } else if !warnings.is_empty() {
            (Color32::from_rgb(0xEF, 0x6C, 0x00), "Warning")
        } else {
            (Color32::from_rgb(0x2E, 0x7D, 0x32), "OK")
        };
        let msgs: Vec<String> = errors.iter().chain(&warnings).cloned().collect();
        let (rect, r) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.0, color);
        r.on_hover_text(if msgs.is_empty() { what.to_string() } else { format!("{what}:\n{}", msgs.join("\n")) });
        ui.label(m.score_string(&t.teams));
        if m.omit {
            ui.label(RichText::new("(not counted in stats)").italics().weak());
        }
    });
}
