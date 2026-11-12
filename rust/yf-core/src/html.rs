//! Port of HTMLReports.ts: the six pages of the HTML stat report (standings, individuals,
//! scoreboard, team detail, player detail, round report). Output must match the app byte for byte,
//! so the tag helpers reproduce the TS template strings exactly, stray spaces included.
//!
//! The caller compiles stats first, as the app does: `Tournament::compile_stats(true, false)`.
//! [`generate_all`] does both.

use std::cell::RefCell;

use crate::js;
use crate::model::*;
use crate::stats::{PhaseStandings, PlayerStats, PoolStats, PoolTeamStats, RoundStats};
use crate::tournament::{MatchLoc, Tournament};
use crate::writer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Standings,
    Individuals,
    Scoreboard,
    TeamDetails,
    PlayerDetails,
    RoundReport,
}

/// `StatReportPageOrder`
pub const PAGE_ORDER: [Page; 6] =
    [Page::Standings, Page::Individuals, Page::Scoreboard, Page::TeamDetails, Page::PlayerDetails, Page::RoundReport];

impl Page {
    /// `StatReportFileNames`
    pub fn file_name(self) -> &'static str {
        match self {
            Page::Standings => "standings.html",
            Page::Individuals => "individuals.html",
            Page::Scoreboard => "games.html",
            Page::TeamDetails => "teamdetail.html",
            Page::PlayerDetails => "playerdetail.html",
            Page::RoundReport => "rounds.html",
        }
    }
    /// `StatReportPageTitles` (the top links)
    fn link_title(self) -> &'static str {
        match self {
            Page::Standings => "Standings",
            Page::Individuals => "Individuals",
            Page::Scoreboard => "Scoreboard",
            Page::TeamDetails => "Team Detail",
            Page::PlayerDetails => "Player Detail",
            Page::RoundReport => "Round Report",
        }
    }
}

/// All six pages, in `PAGE_ORDER`
#[derive(Clone, Debug)]
pub struct Pages {
    pub standings: String,
    pub individuals: String,
    pub scoreboard: String,
    pub team_details: String,
    pub player_details: String,
    pub round_report: String,
}

/// What TournamentManager.generateHtmlReport does: set the link prefix ("report" gives
/// `report_standings.html` etc.; `None` is the app's Stat Report tab), compile stats, make each page.
/// Errors where the TS would throw.
pub fn generate_all(t: &mut Tournament, prefix: Option<&str>) -> Result<Pages, String> {
    t.compile_stats(true, false);
    let mut g = HtmlReportGenerator::new(t);
    g.set_file_prefix(prefix);
    Ok(Pages {
        standings: g.standings_page()?,
        individuals: g.individuals_page()?,
        scoreboard: g.scoreboard_page()?,
        team_details: g.team_detail_page()?,
        player_details: g.player_detail_page()?,
        round_report: g.round_report_page()?,
    })
}

const LINE_BREAK: &str = "<br/>";
const MDASH: &str = "&mdash;";
const NBSP: &str = "&nbsp;";
const EMSP: &str = "&emsp;";
const NDASH: &str = "&ndash;";
const TOP_ANCHOR_ID: &str = "top";
const PHASE_COLORS: [&str; 6] = ["#03a9f430", "#4caf5030", "#ffeb3b30", "#f4433630", "#673ab730", "#ff980030"];

mod css {
    pub const HEADER_AND_DIVIDER: &str = "headerAndDivider";
    pub const STICKY_HEADER: &str = "scoreboardRoundHeader";
    pub const BOX_SCORE_ANCHOR: &str = "boxScoreAnchor";
    pub const BOX_SCORE_TITLE: &str = "boxScoreTitle";
    pub const INLINE_DIVIDER: &str = "inlineDivider";
    pub const SMALL_TEXT: &str = "smallText";
    pub const BOX_SCORE_TABLE_CONTAINER: &str = "boxScoreTable";
    pub const PSEUDO_TFOOT: &str = "pseudoTFoot";
    pub const FLOATING_TOC: &str = "floatingTOC";
    pub const FW_BELOW_1000PX: &str = "fwBelow1000px";
    pub const FW_BELOW_800PX: &str = "fwBelow800px";
}

/// `StatTypes` from StatReportDataTypes.ts (only the ones the HTML report uses)
#[derive(Clone, Copy, Debug)]
enum Stat {
    Wins,
    Losses,
    Ties,
    WinPct,
    PointsPerXTuhTmStandings,
    TuhTmStandings,
    BonusesHrd,
    BonusPts,
    Ppb,
    BbHrd,
    BbPts,
    BbPct,
    LtngPerMtch,
    Lightning,
    LtngPerTmPerGm,
    WouldAdvance,
    AdvancedTo,
    GamesPlayed,
    Tuh,
    PointsPerXTuh,
    RrPtsPerTeamPerXTuh,
    RrPowerPct,
    RrTuConvPct,
    RrNegPct,
    RrPpb,
    RrBbPct,
    RrBonusPct,
    SmallSchool,
    JuniorVarsity,
    Undergrad,
    Div2,
}

/// `columnName`
fn column_name(stat: Stat, r: &ScoringRules) -> String {
    let reg = js::num_to_string(r.regulation_tossup_count());
    match stat {
        Stat::Wins => "W".into(),
        Stat::Losses => "L".into(),
        Stat::Ties => "T".into(),
        Stat::WinPct => "Pct".into(),
        Stat::PointsPerXTuhTmStandings | Stat::PointsPerXTuh => format!("PP{reg}TUH"),
        Stat::TuhTmStandings | Stat::Tuh => "TUH".into(),
        Stat::BonusesHrd => "BHrd".into(),
        Stat::BonusPts => "BPts".into(),
        Stat::Ppb | Stat::RrPpb => "PPB".into(),
        Stat::BbHrd => "BBHrd".into(),
        Stat::BbPts => "BBPts".into(),
        Stat::BbPct | Stat::RrBbPct => "BB%".into(),
        Stat::LtngPerMtch => "Ltng/G".into(),
        Stat::Lightning => "Ltng".into(),
        Stat::LtngPerTmPerGm => "Ltng/Tm/G".into(),
        Stat::WouldAdvance => "Would Advance".into(),
        Stat::AdvancedTo => "Advanced To".into(),
        Stat::GamesPlayed => "GP".into(),
        Stat::RrPtsPerTeamPerXTuh => format!("Pts/Tm/{reg}TUH"),
        Stat::RrPowerPct => "TU Powered".into(),
        Stat::RrTuConvPct => "TU Converted".into(),
        Stat::RrNegPct => format!("Negs/Tm/{reg}TUH"),
        Stat::RrBonusPct => "Bonus %".into(),
        Stat::SmallSchool => "SS".into(),
        Stat::JuniorVarsity => "JV".into(),
        Stat::Undergrad => "UG".into(),
        Stat::Div2 => "D2".into(),
    }
}

/// `columnTooltip`
fn column_tooltip(stat: Stat, r: &ScoringRules) -> String {
    let reg = js::num_to_string(r.regulation_tossup_count());
    match stat {
        Stat::WinPct => "Win percentage".into(),
        Stat::PointsPerXTuhTmStandings => {
            if r.use_overtime_in_pptuh() {
                format!("Points per {reg} tossups heard")
            } else {
                format!("Points scored in regulation per {reg} regulation tossups heard")
            }
        }
        Stat::PointsPerXTuh => format!("Points per {reg} tossups heard"),
        Stat::TuhTmStandings => {
            if r.use_overtime_in_pptuh() { "Tossups heard" } else { "Tossups heard in regulation" }.into()
        }
        Stat::Tuh => "Tossups heard".into(),
        Stat::BonusesHrd => "Bonuses heard".into(),
        Stat::BonusPts => {
            if r.bonuses_bounce_back { "Points scored on bonuses (as the controlling team)" } else { "Points scored on bonuses" }
                .into()
        }
        Stat::Ppb => if r.bonuses_bounce_back { "Points per bonus (as the controlling team)" } else { "Points per bonus" }.into(),
        Stat::RrPpb => {
            if r.bonuses_bounce_back { "Average points per bonus for the controlling team" } else { "Points per bonus" }.into()
        }
        Stat::BbHrd => "Bonus parts that bounced back to this team".into(),
        Stat::BbPts => "Points scored on bonus bouncebacks".into(),
        Stat::BbPct | Stat::RrBbPct => "Percentage of bounceback bonus parts that were answered correctly".into(),
        Stat::LtngPerMtch => "Points earned from lightning rounds per game".into(),
        Stat::Lightning => "Lightning round points".into(),
        Stat::LtngPerTmPerGm => "Points earned from lightning rounds per team per game".into(),
        Stat::WouldAdvance => {
            "Level of the next stage of the tournament that the team would advance to, given the current standings".into()
        }
        Stat::GamesPlayed => "Games played".into(),
        Stat::RrPtsPerTeamPerXTuh => format!("Points per team per {reg} tossups heard"),
        Stat::RrPowerPct => "Percentage of tossups powered by either team".into(),
        Stat::RrTuConvPct => "Percentage of tossups answered correctly by either team".into(),
        Stat::RrNegPct => format!("Incorrect tossup interrupts per team per {reg} tossups heard"),
        Stat::RrBonusPct => {
            "Percentage of bonus parts that were answered correctly either by the controlling team or on the bounceback".into()
        }
        Stat::SmallSchool => "Small school".into(),
        Stat::JuniorVarsity => "Junior varsity".into(),
        Stat::Undergrad => "Undergrad".into(),
        Stat::Div2 => "Division 2".into(),
        Stat::Wins | Stat::Losses | Stat::Ties | Stat::AdvancedTo => String::new(),
    }
}

/* ------------------------------ tag helpers ------------------------------ */

/// `genericTag`
fn tag(name: &str, contents: &[&str]) -> String {
    format!("<{name}>\n{}\n</{name}>", contents.join("\n"))
}

/// `genericTagWithAttributes`
fn tag_attrs(name: &str, attrs: &[&str], contents: &[&str]) -> String {
    format!("<{name} {}>\n{}\n</{name}>", attrs.join(" "), contents.join("\n"))
}

/// `makeAttribute`
fn attr(name: &str, val: &str) -> String {
    format!("{name}=\"{val}\"")
}

/// `classAttribute`
fn class_attr(classes: &[&str]) -> String {
    attr("class", &classes.join(" "))
}

/// `id`
fn id_attr(val: &str) -> String {
    format!("id={val}")
}

/// `aTag`
fn a_tag(href: &str, contents: &str) -> String {
    format!("<a HREF={href}>{contents}</a>")
}

/// `tableTag`
fn table_tag(rows: &[String], width: Option<&str>, class: Option<&str>, border: Option<&str>) -> String {
    let width = width.map(|w| format!("width={w}")).unwrap_or_default();
    let class = class.map(|c| format!("class={c}")).unwrap_or_default();
    let border = border.map(|b| format!("border={b}")).unwrap_or_default();
    format!("<table {border} {class} {width}>\n{}\n</table>", rows.join("\n"))
}

/// `trTag`
fn tr_tag(cells: &[String]) -> String {
    format!("<tr>\n{}\n</tr>", cells.join("\n"))
}

/// `tableFooter`
fn table_footer(cells: &[String]) -> String {
    format!("<tr class={}>\n{}\n</tr>", css::PSEUDO_TFOOT, cells.join("\n"))
}

/// The `tdAttributes` the report uses. `width: Some("")` still prints `width=""`, as in the TS.
#[derive(Clone, Copy, Default)]
struct Td<'s> {
    right: bool,
    bold: bool,
    width: Option<&'s str>,
}

const PLAIN: Td = Td { right: false, bold: false, width: None };
const RIGHT: Td = Td { right: true, bold: false, width: None };
const BOLD: Td = Td { right: false, bold: true, width: None };

/// `tdTag`. `title` and `style` are never set by the report, so they're always empty.
fn td(a: Td, contents: &str) -> String {
    let align = if a.right { attr("align", "right") } else { String::new() };
    let width = a.width.map(|w| attr("width", w)).unwrap_or_default();
    let inner = if a.bold { tag("b", &[contents]) } else { contents.to_string() };
    format!("<td {align}   {width}>{inner}</td>")
}

fn bold_w(width: &str) -> Td<'_> {
    Td { right: false, bold: true, width: Some(width) }
}

fn bold_right_w(width: &str) -> Td<'_> {
    Td { right: true, bold: true, width: Some(width) }
}

/// `stdTdHeader`
fn header(contents: &str, right: bool, width: Option<&str>) -> String {
    td(Td { right, bold: true, width: width.filter(|w| !w.is_empty()) }, contents)
}

fn text_cell(contents: &str) -> String {
    td(PLAIN, contents)
}

fn numeric_cell(contents: &str) -> String {
    td(RIGHT, contents)
}

/// `unorderedList`
fn unordered_list(items: &[String]) -> String {
    let lis: Vec<String> = items.iter().map(|i| tag("li", &[i])).collect();
    tag("ul", &[&lis.join("\n")])
}

/// `unicodeHTML`
fn unicode_html(codepoint: &str) -> String {
    format!("&#x{codepoint};")
}

/// `madeWithYellowFruit`
fn made_with_yellow_fruit(version: Option<&str>) -> String {
    let link = "<a HREF=https://github.com/ANadig/YellowFruit/releases target=\"_blank\">YellowFruit</a>";
    let div_attrs = "class=\"html-rpt-hide-in-yft-app\" style=\"font-size:x-small; margin-top: 10px\"";
    format!("<div {div_attrs}>Made with {link} {}&nbsp;{}</div>\n", version.unwrap_or(""), unicode_html("1F34C"))
}

/// `cssSelector`
fn css_selector(selector: &str, rules: &[(&str, &str)]) -> String {
    let rules: Vec<String> = rules.iter().map(|(a, v)| format!("{a}: {v};")).collect();
    format!("{selector}{{\n{}\n}}", rules.join("\n"))
}

/// `cssResponsiveTableWidthSelector`
fn css_responsive_width(min_width_px: u32, class: &str, width_pct: u32) -> String {
    format!("@media screen and (min-width: {min_width_px}px) {{\n.{class}{{\nwidth: {width_pct}%;\n}}\n}}")
}

/// `getPageStyle`
fn page_style() -> String {
    let c = |cls: &str| format!(".{cls}");
    let parts = [
        css_selector("HTML", &[("font-family", "Roboto, sans-serif")]),
        css_selector("table", &[("font-size", "11pt"), ("border-spacing", "0"), ("border-collapse", "collapse")]),
        css_selector("td", &[("padding", "5px")]),
        css_selector("tr:nth-child(even)", &[("background-color", "#f2f2f2")]),
        css_selector(
            &c(css::HEADER_AND_DIVIDER),
            &[("display", "flex"), ("flex-direction", "row"), ("margin", "18px 0")],
        ),
        css_selector(
            &c(css::STICKY_HEADER),
            &[
                ("width", "71%"),
                ("position", "sticky"),
                ("top", "0"),
                ("background-color", "white"),
                ("padding-bottom", "10px"),
                ("margin-bottom", "-10px"),
            ],
        ),
        css_selector(&c(css::BOX_SCORE_ANCHOR), &[("padding-top", "30px")]),
        css_selector(&c(css::BOX_SCORE_TITLE), &[("width", "71%")]),
        css_selector(
            &c(css::INLINE_DIVIDER),
            &[("flex-grow", "1"), ("height", "1px"), ("background-color", "#9f9f9f"), ("align-self", "center")],
        ),
        css_selector("ul", &[("margin", "0")]),
        css_selector(&c(css::SMALL_TEXT), &[("font-size", "10pt")]),
        css_selector(&format!(".{} h2", css::HEADER_AND_DIVIDER), &[("margin", "0")]),
        css_selector(
            &c(css::BOX_SCORE_TABLE_CONTAINER),
            &[("display", "flex"), ("gap", "15px"), ("align-items", "flex-start")],
        ),
        css_selector(
            &c(css::PSEUDO_TFOOT),
            &[("border-top", "1px solid #909090"), ("background-color", "#ffffff !important")],
        ),
        css_selector(
            &c(css::FLOATING_TOC),
            &[
                ("top", "150px"),
                ("right", "35px"),
                ("position", "fixed"),
                ("padding-right", "5px"),
                ("background-color", "#cccccc"),
                ("box-shadow", "4px 4px 7px #999999"),
                ("line-height", "1.5"),
                ("z-index", "99"),
            ],
        ),
        css_selector(
            &format!(".{} ul", css::FLOATING_TOC),
            &[("list-style-type", "none"), ("padding-inline-start", "20px")],
        ),
        css_responsive_width(800, css::FW_BELOW_800PX, 60),
        css_responsive_width(1000, css::FW_BELOW_1000PX, 80),
    ];
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    tag("style", &refs)
}

/* ------------------------------ value helpers ------------------------------ */

/// `x?.toString() ?? fallback` (also `|| fallback`, since a number's string is never empty)
fn ns(x: Option<f64>, fallback: &str) -> String {
    x.map(js::num_to_string).unwrap_or_else(|| fallback.to_string())
}

/// `alphaOnly`: drop everything but Unicode letters and numbers (no normalization)
pub fn alpha_only(s: &str) -> String {
    use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};
    use icu_properties::CodePointMapData;
    let gc = CodePointMapData::<GeneralCategory>::new();
    s.chars()
        .filter(|c| {
            let cat = gc.get(*c);
            GeneralCategoryGroup::Letter.contains(cat) || GeneralCategoryGroup::Number.contains(cat)
        })
        .collect()
}

/// `Date.prototype.toDateString()` in local time for a date field as the app loads it
/// (`dayjs(value).toDate()`). "Invalid Date" if it doesn't parse.
pub fn date_string(raw: &str) -> String {
    use chrono::{DateTime, Local, Utc};
    let utc: Option<DateTime<Utc>> = if raw.ends_with('Z') || raw.ends_with('z') {
        DateTime::parse_from_rfc3339(raw).ok().map(|d| d.with_timezone(&Utc))
    } else {
        writer::dayjs_local(raw)
    };
    match utc {
        None => "Invalid Date".into(),
        Some(u) => u.with_timezone(&Local).format("%a %b %d %Y").to_string(),
    }
}

/// `Player.getYearDisplayText`
pub fn year_display_text(p: &Player) -> String {
    const TITLES: [&str; 19] = [
        "Kindergarten",
        "1st Grade",
        "2nd Grade",
        "3rd Grade",
        "4th Grade",
        "5th Grade",
        "6th Grade",
        "7th Grade",
        "8th Grade",
        "9th Grade",
        "10th Grade",
        "11th Grade",
        "12th Grade",
        "Freshman",
        "Sophomore",
        "Junior",
        "Senior",
        "Post-senior",
        "Grad Student",
    ];
    match writer::player_year(&p.year_string) {
        None => p.year_string.clone(),
        // yearTitles has no entry for -1 (PlayerYear.NotApplicable)
        Some(y) if y < 0.0 => "undefined".into(),
        Some(y) => TITLES.get(y as usize).copied().unwrap_or("undefined").to_string(),
    }
}

/// `a.toLocaleUpperCase() < b.toLocaleUpperCase()` etc: UTF-16 code unit order of the upper-cased strings
fn upper_compare(a: &str, b: &str) -> f64 {
    let (a, b) = (a.to_uppercase(), b.to_uppercase());
    match a.encode_utf16().cmp(b.encode_utf16()) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.0,
    }
}

fn find_count(counts: &[crate::stats::Count], value: f64) -> Option<f64> {
    counts.iter().find(|c| c.value == value).and_then(|c| c.number)
}

fn find_pac(counts: &[PlayerAnswerCount], value: f64) -> Option<f64> {
    counts.iter().find(|c| c.value == value).and_then(|c| c.number)
}

/* ------------------------------ the generator ------------------------------ */

/// `HtmlReportGenerator`. The TS keeps phase colours on the generator between pages; they only
/// ever depend on the tournament's full phases, so here they're computed on demand.
pub struct HtmlReportGenerator<'a> {
    t: &'a Tournament,
    file_prefix: String,
    /// First error a page hit where the TS throws
    error: RefCell<Option<String>>,
}

impl<'a> HtmlReportGenerator<'a> {
    pub fn new(t: &'a Tournament) -> Self {
        HtmlReportGenerator { t, file_prefix: String::new(), error: RefCell::new(None) }
    }

    /// `setFilePrefix`
    pub fn set_file_prefix(&mut self, prefix: Option<&str>) {
        self.file_prefix = match prefix {
            Some(p) if !p.is_empty() => format!("{p}_"),
            _ => String::new(),
        };
    }

    fn rules(&self) -> &ScoringRules {
        &self.t.scoring_rules
    }

    fn reg_tu(&self) -> f64 {
        self.t.scoring_rules.regulation_tossup_count()
    }

    fn fail(&self, e: String) {
        let mut err = self.error.borrow_mut();
        if err.is_none() {
            *err = Some(e);
        }
    }

    fn finish(&self, page: String) -> Result<String, String> {
        match self.error.borrow_mut().take() {
            Some(e) => Err(e),
            None => Ok(page),
        }
    }

    pub fn standings_page(&self) -> Result<String, String> {
        let data = self.standings_html();
        self.finish(self.html_page("Team Standings", &data, false))
    }

    pub fn individuals_page(&self) -> Result<String, String> {
        let data = self.individuals_html();
        self.finish(self.html_page("Individuals", &data, false))
    }

    pub fn scoreboard_page(&self) -> Result<String, String> {
        let data = self.scoreboard_html();
        self.finish(self.html_page("Scoreboard", &data, false))
    }

    pub fn team_detail_page(&self) -> Result<String, String> {
        let data = self.team_detail_html();
        self.finish(self.html_page("Team Detail", &data, false))
    }

    pub fn player_detail_page(&self) -> Result<String, String> {
        let data = self.player_detail_html();
        self.finish(self.html_page("Player Detail", &data, false))
    }

    pub fn round_report_page(&self) -> Result<String, String> {
        let data = self.round_report_html();
        self.finish(self.html_page("Round Report", &data, true))
    }

    /// `generateHtmlPage`
    fn html_page(&self, title: &str, data: &str, add_yf_version: bool) -> String {
        let html_header = tag("HEAD", &[&tag("title", &[title])]);
        let top_links = self.top_links();
        let main_header = tag_attrs("h1", &[&id_attr(TOP_ANCHOR_ID)], &[title]);
        let style = page_style();
        let version = add_yf_version.then_some(self.t.app_version.as_str());
        let content = tag_attrs(
            "div",
            &["style=\"font-size: 11pt; text-size-adjust: none;\""],
            &[data, &made_with_yellow_fruit(version)],
        );
        let body = tag("BODY", &[&top_links, &main_header, &style, &content]);
        tag("HTML", &[&html_header, &body])
    }

    /* ---------------- standings ---------------- */

    fn standings_html(&self) -> String {
        let t = self.t;
        if t.stats.is_empty() {
            return String::new();
        }
        let mut sections = vec![self.general_metadata()];
        if t.finals_rankings_ready {
            let h = self.header_with_divider("Final Rankings", Page::Standings, true, false);
            sections.push(format!("{h}\n{}", self.cumulative_standings_table(false)));
        }
        sections.push(self.finals_list().join("\n"));

        let mut printed_phase_count = 0;
        let mut printed_combined_playoff = false;
        let n = t.stats.len();
        for i in (0..n).rev() {
            let ps = &t.stats[i];
            let phase = &t.phases[ps.phase];
            if !phase.any_teams_assigned() {
                continue;
            }
            let combined = if i == n - 1 && t.all_prelim_games_carry_over() {
                t.prelims_plus_playoff_stats.as_ref()
            } else {
                None
            };
            if let Some(pp) = combined {
                let h = self.header_with_divider(
                    &format!("{}: Total", phase.name),
                    Page::Standings,
                    !t.finals_rankings_ready && printed_phase_count == 1,
                    false,
                );
                sections.push(format!("{h}\n{}", self.standings_for_one_phase(pp)));
                printed_combined_playoff = true;
            }
            printed_phase_count += 1;
            let title = if combined.is_some() { format!("{} Only", phase.name) } else { phase.name.clone() };
            let h = self.header_with_divider(
                &title,
                Page::Standings,
                !t.finals_rankings_ready && printed_phase_count == 1,
                false,
            );
            sections.push(format!("{h}\n{}", self.standings_for_one_phase(ps)));
        }

        if t.number_of_phases_with_stats() > 1 && !t.finals_rankings_ready && !printed_combined_playoff {
            let h = self.header_with_divider("Cumulative", Page::Standings, false, false);
            sections.push(format!("{h}\n{}", self.cumulative_standings_table(true)));
        }
        sections.join("\n")
    }

    /// `generalMetaData`
    fn general_metadata(&self) -> String {
        let t = self.t;
        let mut segs = vec![];
        if !t.name.is_empty() {
            segs.push(tag("span", &[&t.name]));
        }
        if let Some(sd) = &t.start_date {
            let s_date = date_string(sd);
            match &t.end_date {
                Some(ed) => segs.push(tag("span", &[&format!("{s_date} {NDASH} {}", date_string(ed))])),
                None => segs.push(tag("span", &[&s_date])),
            }
        }
        if !t.site_name.is_empty() {
            segs.push(tag("span", &[&t.site_name]));
        }
        if !t.question_set.is_empty() {
            segs.push(tag("span", &[&format!("Question set: {}", t.question_set)]));
        }
        segs.join(&tag("span", &[&format!("{EMSP}|{EMSP}")]))
    }

    fn standings_for_one_phase(&self, ps: &PhaseStandings) -> String {
        let next = self.t.next_full_phase(ps.phase);
        let tb = self.t.tiebreaker_phase_for(ps.phase);
        let tables: Vec<String> = ps
            .pools
            .iter()
            .map(|pool| {
                let h = if ps.pools.len() > 1 { tag("h3", &[&pool.pool.name]) } else { String::new() };
                format!("{h}\n{}", self.one_standings_table(pool, ps.any_ties_exist, tb, next))
            })
            .collect();
        tables.join("\n") + LINE_BREAK
    }

    fn one_standings_table(&self, pool: &PoolStats, any_ties: bool, tb: Option<PhaseId>, next: Option<PhaseId>) -> String {
        let mut rows = vec![self.standings_header(any_ties, next, false, false)];
        for ts in &pool.pool_teams {
            rows.push(self.standings_row(ts, Some(&pool.pool), any_ties, next, false, false));
        }
        let full_width = match next {
            None => true,
            Some(n) => !self.t.phases[n].any_teams_assigned() && !self.t.using_schedule_template,
        };
        let class = full_width.then_some(css::FW_BELOW_1000PX);
        format!("{}\n{}", table_tag(&rows, None, class, None), self.tb_or_finals_match_list(tb, Some(&pool.pool)))
    }

    fn cumulative_standings_table(&self, omit_rank: bool) -> String {
        let Some(stats) = &self.t.cumulative_stats else { return String::new() };
        let mut rows = vec![self.standings_header(stats.any_ties_exist, None, true, omit_rank)];
        for ts in &stats.team_stats {
            rows.push(self.standings_row(ts, None, stats.any_ties_exist, None, true, omit_rank));
        }
        table_tag(&rows, None, Some(css::FW_BELOW_1000PX), None)
    }

    fn standings_header(&self, any_ties: bool, next: Option<PhaseId>, cumulative: bool, omit_rank: bool) -> String {
        let t = self.t;
        let r = self.rules();
        let mut cells = vec![];
        if !omit_rank {
            cells.push(header("Rank", false, Some("3%")));
        }
        cells.push(td(bold_w(if cumulative { "" } else { "25%" }), "Team"));
        if t.track_small_school {
            cells.push(header(&self.abbr(Stat::SmallSchool), false, Some("3%")));
        }
        if t.track_jv {
            cells.push(header(&self.abbr(Stat::JuniorVarsity), false, Some("3%")));
        }
        if t.track_ug {
            cells.push(header(&self.abbr(Stat::Undergrad), false, Some("3%")));
        }
        if t.track_div2 {
            cells.push(header(&self.abbr(Stat::Div2), false, Some("3%")));
        }
        cells.push(header(&self.abbr(Stat::Wins), true, Some("4%")));
        cells.push(header(&self.abbr(Stat::Losses), true, Some("3%")));
        if any_ties {
            cells.push(header(&self.abbr(Stat::Ties), true, Some("3%")));
        }
        if !cumulative {
            cells.push(header(&self.abbr(Stat::WinPct), true, Some("7%")));
        }
        cells.push(header(&self.abbr(Stat::PointsPerXTuhTmStandings), true, Some("8%")));
        self.push_tossup_value_headers(&mut cells);
        cells.push(header(&self.abbr(Stat::TuhTmStandings), true, Some("6%")));
        if r.use_bonuses {
            cells.push(header(&self.abbr(Stat::Ppb), true, Some("7%")));
        }
        if r.bonuses_bounce_back {
            cells.push(header(&self.abbr(Stat::BbPct), true, Some("6%")));
        }
        if r.use_lightning_rounds() {
            cells.push(header(&self.abbr(Stat::LtngPerMtch), true, None));
        }
        if let Some(n) = next {
            if t.phases[n].any_teams_assigned() {
                cells.push(header(&self.abbr(Stat::AdvancedTo), false, None));
            } else if t.using_schedule_template {
                cells.push(header(&self.abbr(Stat::WouldAdvance), false, None));
            }
        }
        tr_tag(&cells)
    }

    fn standings_row(
        &self,
        ts: &PoolTeamStats,
        pool: Option<&Pool>,
        any_ties: bool,
        next: Option<PhaseId>,
        cumulative: bool,
        omit_rank: bool,
    ) -> String {
        let t = self.t;
        let r = self.rules();
        let team = &t.teams[ts.team];
        let mut cells = vec![];
        if !omit_rank {
            cells.push(td(PLAIN, &ts.rank));
        }
        cells.push(text_cell(&self.team_detail_link(ts.team, true)));
        if t.track_small_school {
            let ss = t.registration_for_team(ts.team).is_some_and(|reg| reg.is_small_school);
            cells.push(td(PLAIN, if ss { "SS" } else { "" }));
        }
        if t.track_jv {
            cells.push(td(PLAIN, if team.is_jv { "JV" } else { "" }));
        }
        if t.track_ug {
            cells.push(td(PLAIN, if team.is_ug { "UG" } else { "" }));
        }
        if t.track_div2 {
            cells.push(td(PLAIN, if team.is_d2 { "D2" } else { "" }));
        }
        cells.push(td(RIGHT, &js::num_to_string(ts.wins)));
        cells.push(td(RIGHT, &js::num_to_string(ts.losses)));
        if any_ties {
            cells.push(td(RIGHT, &js::num_to_string(ts.ties)));
        }
        if !cumulative {
            let pct = ts.win_pct();
            cells.push(td(RIGHT, &if pct.is_nan() { MDASH.to_string() } else { js::to_fixed(pct, 3) }));
        }
        let ppg = if ts.correct_tuh(r) == 0.0 {
            MDASH.to_string()
        } else {
            js::to_fixed(ts.pts_per_reg_tuh(r) * self.reg_tu(), 1)
        };
        cells.push(td(RIGHT, &ppg));
        for at in &r.answer_types {
            cells.push(td(RIGHT, &ns(find_count(&ts.tossup_counts, at.value), "0")));
        }
        cells.push(td(RIGHT, &js::num_to_string(ts.correct_tuh(r))));
        if r.use_bonuses {
            let ppb = ts.pts_per_bonus();
            cells.push(td(RIGHT, &if ppb.is_nan() { MDASH.to_string() } else { js::to_fixed(ppb, 2) }));
        }
        if r.bonuses_bounce_back {
            let bb = ts.bounceback_conv_pct_string(r);
            cells.push(td(RIGHT, if bb == "-" { MDASH } else { &bb }));
        }
        if r.use_lightning_rounds() {
            cells.push(td(RIGHT, &ts.lightning_pts_per_match_string()));
        }
        if let Some(n) = next {
            if t.phases[n].any_teams_assigned() {
                cells.push(td(PLAIN, &self.definite_advancement(ts, pool, n)));
            } else if t.using_schedule_template {
                cells.push(td(PLAIN, &provisional_advancement(ts)));
            }
        }
        tr_tag(&cells)
    }

    /// `definiteAdvancementTierDisplay`
    fn definite_advancement(&self, ts: &PoolTeamStats, pool: Option<&Pool>, next: PhaseId) -> String {
        let did_not_advance = pool
            .and_then(|p| p.pool_teams.iter().find(|pt| pt.team == ts.team))
            .is_some_and(|pt| pt.did_not_advance == Some(true));
        if did_not_advance {
            return "None".into();
        }
        let phase = &self.t.phases[next];
        phase.find_pool_with_team(ts.team).map(|p| phase.pools[p].name.clone()).unwrap_or_default()
    }

    fn finals_list(&self) -> Vec<String> {
        let mut list: Vec<String> = self
            .t
            .finals_phases()
            .into_iter()
            .map(|ph| {
                let ml = self.tb_or_finals_match_list(Some(ph), None);
                if ml.is_empty() {
                    return String::new();
                }
                format!("{}\n{ml}", self.header_with_divider(&self.t.phases[ph].name, Page::Standings, false, false))
            })
            .collect();
        list.reverse();
        list
    }

    fn tb_or_finals_match_list(&self, phase: Option<PhaseId>, pool: Option<&Pool>) -> String {
        let Some(pi) = phase else { return String::new() };
        let ph = &self.t.phases[pi];
        let relevant = |m: &Match| match pool {
            None => true,
            Some(p) => m.left.team.is_some_and(|t| p.includes_team(t)) || m.right.team.is_some_and(|t| p.includes_team(t)),
        };
        let matches: Vec<&Match> = ph.rounds.iter().flat_map(|r| r.matches.iter()).filter(|m| relevant(m)).collect();
        if matches.is_empty() {
            return String::new();
        }
        let is_tb = ph.phase_type == PhaseType::Tiebreaker;
        let title = if is_tb { tag("span", &["Tiebreakers:"]) } else { String::new() };
        let items: Vec<String> = matches
            .iter()
            .map(|m| format!("{} ({})", m.winner_loser_string(&self.t.teams), self.scoreboard_match_link(m, "box")))
            .collect();
        let list = unordered_list(&items);
        if is_tb {
            return tag_attrs("div", &[&class_attr(&[css::SMALL_TEXT])], &[&title, &list]);
        }
        tag("div", &[&title, &list])
    }

    /* ---------------- individuals ---------------- */

    fn individuals_html(&self) -> String {
        let t = self.t;
        if t.stats.is_empty() {
            return String::new();
        }
        let mut sections = vec![];
        let mut printed = 0;
        for ps in t.stats.iter().rev() {
            if !ps.players.iter().any(|p| p.tossups_heard > 0.0) {
                continue;
            }
            printed += 1;
            let h = self.header_with_divider(&t.phases[ps.phase].name, Page::Individuals, printed == 1, false);
            sections.push(format!("{h}\n{}", self.individuals_table(&ps.players, false)));
        }
        if t.number_of_phases_with_stats() > 1 {
            if let Some(cum) = &t.cumulative_stats {
                let h = self.header_with_divider("All Games", Page::Individuals, false, false);
                sections.push(format!("{h}\n{}", self.individuals_table(&cum.players, true)));
            }
        }
        sections.join("\n")
    }

    fn individuals_table(&self, players: &[PlayerStats], skip_rank: bool) -> String {
        let mut rows = vec![self.individuals_header(skip_rank)];
        for ps in players {
            if ps.tossups_heard == 0.0 {
                continue;
            }
            rows.push(self.individuals_row(ps, skip_rank));
        }
        table_tag(&rows, None, Some(css::FW_BELOW_1000PX), None)
    }

    fn individuals_header(&self, skip_rank: bool) -> String {
        let t = self.t;
        let mut cells = vec![];
        if !skip_rank {
            cells.push(td(BOLD, "Rank"));
        }
        cells.push(header("Player", false, None));
        if t.track_player_year {
            cells.push(header("Year/Grade", false, None));
        }
        if t.track_ug {
            cells.push(header(&self.abbr(Stat::Undergrad), false, None));
        }
        if t.track_div2 {
            cells.push(header(&self.abbr(Stat::Div2), false, None));
        }
        cells.push(header("Team", false, None));
        cells.push(header(&self.abbr(Stat::GamesPlayed), true, None));
        self.push_tossup_value_headers(&mut cells);
        cells.push(header(&self.abbr(Stat::Tuh), true, None));
        cells.push(header(&self.abbr(Stat::PointsPerXTuh), true, None));
        tr_tag(&cells)
    }

    fn individuals_row(&self, ps: &PlayerStats, skip_rank: bool) -> String {
        let t = self.t;
        let player = &t.players[ps.player];
        let mut cells = vec![];
        if !skip_rank {
            cells.push(td(PLAIN, &format!("{}{}", ps.rank, if ps.rank_tie { "=" } else { "" })));
        }
        cells.push(text_cell(&self.player_detail_link(ps.player, ps.team)));
        if t.track_player_year {
            cells.push(td(PLAIN, &player.year_string));
        }
        if t.track_ug {
            cells.push(td(PLAIN, if player.is_ug { "UG" } else { "" }));
        }
        if t.track_div2 {
            cells.push(td(PLAIN, if player.is_d2 { "D2" } else { "" }));
        }
        cells.push(text_cell(&self.team_detail_link(ps.team, false)));
        cells.push(td(RIGHT, &js::to_fixed(ps.games_played, 1)));
        for at in &self.rules().answer_types {
            cells.push(td(RIGHT, &ns(find_count(&ps.tossup_counts, at.value), "0")));
        }
        cells.push(td(RIGHT, &js::num_to_string(ps.tossups_heard)));
        cells.push(td(RIGHT, &self.pptuh_string(ps)));
        tr_tag(&cells)
    }

    fn pptuh_string(&self, ps: &PlayerStats) -> String {
        match ps.pptuh() {
            Some(p) => js::to_fixed(p * self.reg_tu(), 2),
            None => MDASH.to_string(),
        }
    }

    /* ---------------- scoreboard ---------------- */

    fn scoreboard_html(&self) -> String {
        let mut rounds = vec![];
        let mut toc = vec![];
        for phase in &self.t.phases {
            let mut phase_has_games = false;
            for round in &phase.rounds {
                if round.matches.is_empty() {
                    continue;
                }
                rounds.push(self.one_round_of_box_scores(round, phase));
                if !phase_has_games {
                    phase_has_games = true;
                    if phase.uses_numeric_rounds() {
                        toc.push(phase.name.clone());
                    } else {
                        toc.push(self.scoreboard_round_link(round, &phase.name));
                    }
                }
                if phase.uses_numeric_rounds() {
                    toc.push(format!("{NBSP}{NBSP}{}", self.scoreboard_round_link(round, &round.display_name(true))));
                }
            }
        }
        let toc = tag_attrs("div", &[&class_attr(&[css::FLOATING_TOC])], &[&unordered_list(&toc)]);
        format!("{toc}\n{}", rounds.join("\n"))
    }

    fn one_round_of_box_scores(&self, round: &Round, phase: &Phase) -> String {
        let mut segs = vec![];
        if round.number != 1.0 {
            segs.push("<br /><br />".to_string());
        }
        let mut title = round.display_name(false);
        if phase.is_full_phase() {
            title += &format!(" - {}", phase.name);
        }
        segs.push(tag_attrs("div", &[&id_attr(&round_link_id(round))], &[]));
        segs.push(self.header_with_divider(&title, Page::Scoreboard, round.number == 1.0, true));
        for m in &round.matches {
            segs.push(self.box_score(m));
        }
        tag("div", &[&segs.join("\n")])
    }

    fn box_score(&self, m: &Match) -> String {
        let t = self.t;
        let r = self.rules();
        let mut segs = vec![];
        segs.push(tag_attrs(
            "div",
            &[&id_attr(&writer::match_id(t, m)), &class_attr(&[css::BOX_SCORE_ANCHOR])],
            &[],
        ));
        segs.push(tag_attrs("h3", &[&class_attr(&[css::BOX_SCORE_TITLE])], &[&m.score_string(&t.teams)]));
        if m.is_forfeit() {
            return segs.join("\n");
        }
        if !m.carryover_phases.is_empty() {
            let names: Vec<&str> = m.carryover_phases.iter().map(|p| t.phases[*p].name.as_str()).collect();
            segs.push(tag("p", &[&format!("Carries over to: {}", names.join(", "))]));
        }
        let mut tu_read = format!("Tossups read: {}", ns(m.tossups_read, "unknown"));
        if let Some(ot) = m.overtime_tossups_read.filter(|x| *x != 0.0 && !x.is_nan()) {
            tu_read += &format!(" ({} in OT)", js::num_to_string(ot));
        }
        segs.push(tag("p", &[&tu_read]));
        let left = self.box_score_table_one_team(&m.left);
        let right = self.box_score_table_one_team(&m.right);
        segs.push(tag_attrs("div", &[&class_attr(&[css::BOX_SCORE_TABLE_CONTAINER])], &[&left, &right]));
        if r.use_bonuses || r.use_lightning_rounds() {
            segs.push("<br />".to_string());
        }
        if r.use_bonuses {
            segs.push(self.box_score_bonus_table(m));
        }
        if r.bonuses_bounce_back {
            segs.push(self.box_score_bounceback_table(m));
        }
        if r.use_lightning_rounds() {
            segs.push(self.box_score_lightning_table(m));
        }
        segs.join("\n")
    }

    fn truncated_team_name(&self, mt: &MatchTeam, len: usize) -> String {
        mt.team.map(|tm| self.t.teams[tm].truncated_name(len)).unwrap_or_default()
    }

    fn box_score_table_one_team(&self, mt: &MatchTeam) -> String {
        let r = self.rules();
        let mut header_cells = vec![header(&self.truncated_team_name(mt, 25), false, None), header("TUH", true, None)];
        self.push_tossup_value_headers(&mut header_cells);
        header_cells.push(header("Tot", true, Some("8%")));
        let mut rows = vec![tr_tag(&header_cells)];
        for mp in mt.active_match_players() {
            let mut cells = vec![td(PLAIN, &self.t.players[mp.player].name), td(RIGHT, &ns(mp.tossups_heard, "0"))];
            for at in &r.answer_types {
                cells.push(td(RIGHT, &ns(find_pac(&mp.answer_counts, at.value), "0")));
            }
            cells.push(td(RIGHT, &js::num_to_string(mp.points())));
            rows.push(tr_tag(&cells));
        }
        let mut cells = vec![td(BOLD, "Total"), td(PLAIN, "")];
        let counts = mt.answer_counts();
        let bold_right = Td { right: true, bold: true, width: None };
        for at in &r.answer_types {
            cells.push(td(bold_right, &ns(find_pac(&counts, at.value), "0")));
        }
        cells.push(td(bold_right, &js::num_to_string(mt.tossup_points())));
        rows.push(table_footer(&cells));
        table_tag(&rows, Some("35%"), None, None)
    }

    fn box_score_bonus_table(&self, m: &Match) -> String {
        let head = tr_tag(&[
            td(bold_w("40%"), "Bonuses"),
            td(bold_right_w("20%"), "Heard"),
            td(bold_right_w("20%"), "Pts"),
            td(bold_right_w("20%"), "PPB"),
        ]);
        let row = |mt: &MatchTeam| {
            let [pts, heard, ppb] = mt.bonus_stats(self.rules());
            tr_tag(&[
                td(PLAIN, &self.truncated_team_name(mt, 25)),
                td(RIGHT, &heard),
                td(RIGHT, &pts),
                td(RIGHT, &ppb),
            ])
        };
        table_tag(&[head, row(&m.left), row(&m.right)], None, None, None)
    }

    fn box_score_bounceback_table(&self, m: &Match) -> String {
        let head = tr_tag(&[
            td(bold_w("40%"), "Bouncebacks"),
            td(bold_right_w("20%"), "Parts Heard"),
            td(bold_right_w("20%"), "Pts"),
            td(bold_right_w("20%"), "Success%"),
        ]);
        let row = |side: Side| {
            let [heard, rate] = m.bounceback_stats_string(side, self.rules());
            let mt = m.side(side);
            tr_tag(&[
                td(PLAIN, &self.truncated_team_name(mt, 25)),
                td(RIGHT, &heard),
                td(RIGHT, &ns(mt.bonus_bounceback_points, "0")),
                td(RIGHT, &format!("{rate}%")),
            ])
        };
        table_tag(&[head, row(Side::Left), row(Side::Right)], None, None, None)
    }

    fn box_score_lightning_table(&self, m: &Match) -> String {
        let head = tr_tag(&[td(bold_w("70%"), "Lightning Round"), td(bold_right_w("30%"), "Pts")]);
        let row = |mt: &MatchTeam| {
            let name = mt.team.map(|tm| self.t.teams[tm].name.clone()).unwrap_or_default();
            tr_tag(&[td(PLAIN, &name), td(RIGHT, &ns(mt.lightning_points, "0"))])
        };
        table_tag(&[head, row(&m.left), row(&m.right)], None, None, None)
    }

    /* ---------------- team detail ---------------- */

    fn team_detail_html(&self) -> String {
        let t = self.t;
        let Some(cum) = &t.cumulative_stats else { return String::new() };
        let mut list: Vec<&PoolTeamStats> = cum.team_stats.iter().collect();
        js::sort_by(&mut list, |a, b| upper_compare(&t.teams[a.team].name, &t.teams[b.team].name));
        list.iter().map(|ts| self.team_detail_one_team(ts)).collect::<Vec<_>>().join("\n")
    }

    fn team_detail_one_team(&self, ts: &PoolTeamStats) -> String {
        let team = &self.t.teams[ts.team];
        let attrs = self.team_attribute_string(ts.team);
        let id = id_attr(&alpha_only(&team.name));
        let mut h2_attrs = vec![id.as_str()];
        if !attrs.is_empty() {
            h2_attrs.push("style=\"margin-bottom:0\"");
        }
        let mut segs = vec![tag_attrs("h2", &h2_attrs, &[&team.name])];
        if !attrs.is_empty() {
            segs.push(tag_attrs("div", &["style=\"margin-bottom:15px; font-style: italic\""], &[&attrs]));
        }
        segs.push(self.team_detail_match_table(ts));
        segs.push("<br />".to_string());
        segs.push(self.team_detail_player_table(ts.team));
        segs.join("\n")
    }

    fn team_attribute_string(&self, tid: TeamId) -> String {
        let t = self.t;
        let team = &t.teams[tid];
        let mut a = vec![];
        if t.track_small_school && t.registration_for_team(tid).is_some_and(|r| r.is_small_school) {
            a.push("Small School");
        }
        if t.track_jv && team.is_jv {
            a.push("Junior Varsity");
        }
        if t.track_ug && team.is_ug {
            a.push("Undergraduate");
        }
        if t.track_div2 && team.is_d2 {
            a.push("Division II");
        }
        a.join(", ")
    }

    fn team_detail_match_table(&self, ts: &PoolTeamStats) -> String {
        let omit = self.t.phases.len() < 2;
        let mut rows = vec![self.team_detail_match_table_header(omit)];
        for (loc, side) in &ts.matches {
            rows.push(self.team_detail_match_table_row(*loc, *side, omit));
        }
        rows.push(self.team_detail_match_table_footer(ts, omit));
        table_tag(&rows, Some("100%"), None, None)
    }

    fn team_detail_match_table_header(&self, omit_phase: bool) -> String {
        let r = self.rules();
        let mut cells = vec![td(bold_w("5%"), "Round")];
        if !omit_phase {
            cells.push(td(bold_w("12%"), "Stage"));
        }
        if !omit_phase && self.t.has_any_carryover() {
            cells.push(td(bold_w("8%"), "Carried To"));
        }
        cells.push(header("Opponent", false, None));
        cells.push(header("", false, None));
        cells.push(header("Score", false, None));
        self.push_tossup_value_headers(&mut cells);
        cells.push(header(&self.abbr(Stat::Tuh), true, None));
        if r.use_bonuses {
            cells.push(header(&self.abbr(Stat::BonusesHrd), true, None));
            cells.push(header(&self.abbr(Stat::BonusPts), true, None));
            cells.push(header(&self.abbr(Stat::Ppb), true, None));
        }
        if r.bonuses_bounce_back {
            cells.push(header(&self.abbr(Stat::BbHrd), true, None));
            cells.push(header(&self.abbr(Stat::BbPts), true, None));
            cells.push(header(&self.abbr(Stat::BbPct), true, None));
        }
        if r.use_lightning_rounds() {
            cells.push(header(&self.abbr(Stat::Lightning), true, None));
        }
        if self.t.packet_names_exist() {
            cells.push(header("Packet", false, None));
        }
        tr_tag(&cells)
    }

    /// The Round, Stage, Opponent, result and Score cells that both detail pages start a row with
    fn detail_row_start(&self, loc: MatchLoc, side: Side, omit_phase: bool, carryover_col: bool) -> Vec<String> {
        let t = self.t;
        let m = t.match_at(loc);
        let phase = &t.phases[loc.0];
        let round = &phase.rounds[loc.1];
        let mut cells = vec![];
        cells.push(text_cell(&if phase.uses_numeric_rounds() { js::num_to_string(round.number) } else { String::new() }));
        if !omit_phase {
            cells.push(self.colored_phase_cell(&[loc.0]));
        }
        if carryover_col {
            cells.push(self.colored_phase_cell(&m.carryover_phases));
        }
        let opp = m.side(side.other()).team.expect("checked by the caller");
        cells.push(text_cell(&self.team_detail_link(opp, false)));
        cells.push(text_cell(m.result_display(side)));
        let score = m.score_only(side, true).unwrap_or_else(|e| {
            self.fail(e);
            String::new()
        });
        cells.push(text_cell(&self.scoreboard_match_link(m, &score)));
        cells
    }

    fn team_detail_match_table_row(&self, loc: MatchLoc, side: Side, omit_phase: bool) -> String {
        let t = self.t;
        let r = self.rules();
        let m = t.match_at(loc);
        let mt = m.side(side);
        if mt.team.is_none() || m.side(side.other()).team.is_none() {
            return tr_tag(&[]);
        }
        let forf = m.is_forfeit();
        let carry = !omit_phase && t.has_any_carryover();
        let mut cells = self.detail_row_start(loc, side, omit_phase, carry);
        let counts = mt.answer_counts();
        for at in &r.answer_types {
            if forf {
                cells.push(numeric_cell(""));
                continue;
            }
            cells.push(numeric_cell(&ns(find_pac(&counts, at.value), "0")));
        }
        cells.push(numeric_cell(&ns(m.tossups_read, "")));
        let blank_if_forf = |s: String| if forf { String::new() } else { s };
        if r.use_bonuses {
            let [pts, heard, ppb] = mt.bonus_stats(r);
            cells.push(numeric_cell(&blank_if_forf(heard)));
            cells.push(numeric_cell(&blank_if_forf(pts)));
            cells.push(numeric_cell(&blank_if_forf(ppb)));
        }
        if r.bonuses_bounce_back {
            let [heard, rate] = m.bounceback_stats_string(side, r);
            cells.push(numeric_cell(&blank_if_forf(heard)));
            cells.push(numeric_cell(&blank_if_forf(ns(mt.bonus_bounceback_points, "0"))));
            cells.push(numeric_cell(&blank_if_forf(format!("{rate}%"))));
        }
        if r.use_lightning_rounds() {
            cells.push(numeric_cell(&blank_if_forf(ns(mt.lightning_points, "0"))));
        }
        if t.packet_names_exist() {
            cells.push(text_cell(&t.phases[loc.0].rounds[loc.1].packet_name));
        }
        tr_tag(&cells)
    }

    fn team_detail_match_table_footer(&self, ts: &PoolTeamStats, omit_phase: bool) -> String {
        let t = self.t;
        let r = self.rules();
        let mut cells = vec![text_cell("")];
        if !omit_phase {
            cells.push(text_cell(""));
        }
        if !omit_phase && t.has_any_carryover() {
            cells.push(text_cell(""));
        }
        cells.push(header("Total", false, None));
        cells.push(header(&ts.record(), false, None));
        cells.push(header("", false, None));
        for at in &r.answer_types {
            cells.push(header(&ns(find_count(&ts.tossup_counts, at.value), "0"), true, None));
        }
        cells.push(header(&js::num_to_string(ts.tuh_total), true, None));
        if r.use_bonuses {
            let ppb = ts.pts_per_bonus();
            let ppb_str = if ppb.is_nan() { MDASH.to_string() } else { js::to_fixed(ppb, 2) };
            cells.push(header(&js::num_to_string(ts.bonuses_heard), true, None));
            cells.push(header(&js::num_to_string(ts.bonus_points), true, None));
            cells.push(header(&ppb_str, true, None));
        }
        if r.bonuses_bounce_back {
            cells.push(header(&js::num_to_string(ts.bounce_back_parts_heard), true, None));
            cells.push(header(&js::num_to_string(ts.bounce_back_points), true, None));
            cells.push(header(&ts.bounceback_conv_pct_string(r), true, None));
        }
        if r.use_lightning_rounds() {
            cells.push(header(&js::num_to_string(ts.lightning_points), true, None));
        }
        if t.packet_names_exist() {
            cells.push(header("", false, None));
        }
        table_footer(&cells)
    }

    fn team_detail_player_table(&self, team: TeamId) -> String {
        let Some(cum) = &self.t.cumulative_stats else { return String::new() };
        let on_team: Vec<&PlayerStats> = cum.players.iter().filter(|p| p.team == team).collect();
        if on_team.is_empty() {
            return String::new();
        }
        let mut rows = vec![self.team_detail_player_table_header()];
        for ps in on_team {
            if ps.tossups_heard == 0.0 {
                continue;
            }
            rows.push(self.team_detail_player_table_row(ps));
        }
        table_tag(&rows, None, Some(css::FW_BELOW_800PX), None)
    }

    fn team_detail_player_table_header(&self) -> String {
        let t = self.t;
        let mut cells = vec![header("Player", false, None)];
        if t.track_player_year {
            cells.push(header("Year/Grade", false, None));
        }
        if t.track_ug {
            cells.push(header(&self.abbr(Stat::Undergrad), false, None));
        }
        if t.track_div2 {
            cells.push(header(&self.abbr(Stat::Div2), false, None));
        }
        cells.push(header(&self.abbr(Stat::GamesPlayed), true, None));
        self.push_tossup_value_headers(&mut cells);
        cells.push(header(&self.abbr(Stat::Tuh), true, Some("10%")));
        cells.push(header(&self.abbr(Stat::PointsPerXTuh), true, Some("12%")));
        tr_tag(&cells)
    }

    fn team_detail_player_table_row(&self, ps: &PlayerStats) -> String {
        let t = self.t;
        let player = &t.players[ps.player];
        let mut cells = vec![text_cell(&self.player_detail_link(ps.player, ps.team))];
        if t.track_player_year {
            cells.push(text_cell(&player.year_string));
        }
        if t.track_ug {
            cells.push(text_cell(if player.is_ug { "UG" } else { "" }));
        }
        if t.track_div2 {
            cells.push(text_cell(if player.is_d2 { "D2" } else { "" }));
        }
        cells.push(numeric_cell(&js::to_fixed(ps.games_played, 1)));
        for at in &self.rules().answer_types {
            cells.push(numeric_cell(&ns(find_count(&ps.tossup_counts, at.value), "0")));
        }
        cells.push(numeric_cell(&js::num_to_string(ps.tossups_heard)));
        cells.push(numeric_cell(&self.pptuh_string(ps)));
        tr_tag(&cells)
    }

    /* ---------------- player detail ---------------- */

    fn player_detail_html(&self) -> String {
        let t = self.t;
        let Some(cum) = &t.cumulative_stats else { return String::new() };
        let mut list: Vec<&PlayerStats> = cum.players.iter().collect();
        js::sort_by(&mut list, |a, b| {
            let c = upper_compare(&t.teams[a.team].name, &t.teams[b.team].name);
            if c != 0.0 {
                return c;
            }
            upper_compare(&t.players[a.player].name, &t.players[b.player].name)
        });
        list.iter().map(|ps| self.player_detail_one_player(ps)).collect::<Vec<_>>().join("\n")
    }

    fn player_detail_one_player(&self, ps: &PlayerStats) -> String {
        let t = self.t;
        let player = &t.players[ps.player];
        let team = &t.teams[ps.team];
        let attrs = self.player_attribute_string(player);
        let id = id_attr(&player_detail_link_id(player, team));
        let mut h2_attrs = vec![id.as_str()];
        if !attrs.is_empty() {
            h2_attrs.push("style=\"margin-bottom:0\"");
        }
        let mut segs = vec![tag_attrs("h2", &h2_attrs, &[&format!("{}, {}", player.name, team.name)])];
        if !attrs.is_empty() {
            segs.push(tag_attrs("div", &["style=\"margin-bottom:15px; font-style: italic\""], &[&attrs]));
        }
        segs.push(self.player_detail_table(ps));
        segs.join("\n")
    }

    fn player_attribute_string(&self, player: &Player) -> String {
        let t = self.t;
        let mut a = vec![];
        if t.track_ug && player.is_ug {
            a.push("Undergraduate".to_string());
        }
        if t.track_div2 && player.is_d2 {
            a.push("Division II".to_string());
        }
        if t.track_player_year && !player.year_string.is_empty() {
            a.push(format!("Year/Grade: {}", year_display_text(player)));
        }
        a.join(", ")
    }

    fn player_detail_table(&self, ps: &PlayerStats) -> String {
        let omit = self.t.phases.len() < 2;
        let mut rows = vec![self.player_detail_table_header(omit)];
        for (loc, side, mpi) in &ps.matches {
            rows.push(self.player_detail_table_row(*loc, *side, *mpi, omit));
        }
        rows.push(self.player_detail_table_footer(ps, omit));
        table_tag(&rows, None, Some(css::FW_BELOW_1000PX), None)
    }

    fn player_detail_table_header(&self, omit_phase: bool) -> String {
        let mut cells = vec![td(bold_w("5%"), "Round")];
        if !omit_phase {
            cells.push(td(bold_w("15%"), "Stage"));
        }
        cells.push(header("Opponent", false, None));
        cells.push(header("", false, None));
        cells.push(header("Score", false, None));
        cells.push(header(&self.abbr(Stat::GamesPlayed), true, None));
        self.push_tossup_value_headers(&mut cells);
        cells.push(header(&self.abbr(Stat::Tuh), true, None));
        cells.push(header("Pts", true, None));
        tr_tag(&cells)
    }

    fn player_detail_table_row(&self, loc: MatchLoc, side: Side, mpi: usize, omit_phase: bool) -> String {
        let m = self.t.match_at(loc);
        if m.side(side).team.is_none() || m.side(side.other()).team.is_none() {
            return tr_tag(&[]);
        }
        let mp = &m.side(side).match_players[mpi];
        let forf = m.is_forfeit();
        let mut cells = self.detail_row_start(loc, side, omit_phase, false);
        let gp = nn0(mp.tossups_heard) / nn0(m.tossups_read);
        cells.push(numeric_cell(&if forf { String::new() } else { js::to_fixed(gp, 1) }));
        for at in &self.rules().answer_types {
            if forf {
                cells.push(numeric_cell(""));
                continue;
            }
            cells.push(numeric_cell(&ns(find_pac(&mp.answer_counts, at.value), "0")));
        }
        cells.push(numeric_cell(&if forf { String::new() } else { ns(mp.tossups_heard, "") }));
        cells.push(numeric_cell(&if forf { String::new() } else { js::num_to_string(mp.points()) }));
        tr_tag(&cells)
    }

    fn player_detail_table_footer(&self, ps: &PlayerStats, omit_phase: bool) -> String {
        let mut cells = vec![text_cell("")];
        if !omit_phase {
            cells.push(text_cell(""));
        }
        cells.push(header("Total", false, None));
        cells.push(text_cell(""));
        cells.push(text_cell(""));
        cells.push(header(&js::to_fixed(ps.games_played, 1), true, None));
        for at in &self.rules().answer_types {
            cells.push(header(&ns(find_count(&ps.tossup_counts, at.value), "0"), true, None));
        }
        cells.push(header(&js::num_to_string(ps.tossups_heard), true, None));
        cells.push(header(&js::num_to_string(ps.total_points()), true, None));
        table_footer(&cells)
    }

    /* ---------------- round report ---------------- */

    fn round_report_html(&self) -> String {
        let t = self.t;
        let Some(cum) = &t.cumulative_stats else { return String::new() };
        if cum.rounds.is_empty() {
            return String::new();
        }
        let omit = t.number_of_phases_with_stats() < 2;
        let mut rows = vec![self.round_report_header(omit)];
        for rs in &cum.rounds {
            rows.push(self.round_report_row(rs, omit));
        }
        rows.push(self.round_report_footer(&cum.total, omit));
        table_tag(&rows, None, None, None)
    }

    fn round_report_header(&self, omit_phase: bool) -> String {
        let r = self.rules();
        let w = Some(if omit_phase { "10%" } else { "9%" });
        let mut cells = vec![header("Round", false, Some("10%"))];
        if !omit_phase {
            cells.push(header("Stage", false, Some("15%")));
        }
        cells.push(header("Games", true, Some("10%")));
        cells.push(header(&self.abbr(Stat::RrPtsPerTeamPerXTuh), true, w));
        if has_powers(r) {
            cells.push(header(&self.abbr(Stat::RrPowerPct), true, w));
        }
        cells.push(header(&self.abbr(Stat::RrTuConvPct), true, w));
        if has_negs(r) {
            cells.push(header(&self.abbr(Stat::RrNegPct), true, w));
        }
        if r.use_bonuses {
            cells.push(header(&self.abbr(Stat::RrPpb), true, w));
        }
        if r.bonuses_bounce_back {
            cells.push(header(&self.abbr(Stat::RrBbPct), true, w));
            cells.push(header(&self.abbr(Stat::RrBonusPct), true, w));
        }
        if r.use_lightning_rounds() {
            cells.push(header(&self.abbr(Stat::LtngPerTmPerGm), true, w));
        }
        if self.t.packet_names_exist() {
            cells.push(header("Packet", false, None));
        }
        tr_tag(&cells)
    }

    /// The stat cells of a round report row, given how to make each cell
    fn round_stat_cells(&self, rs: &RoundStats, cell: impl Fn(&str) -> String) -> Vec<String> {
        let r = self.rules();
        let d = rs.derived(r);
        let mut cells = vec![cell(&js::num_to_string(rs.games)), cell(&js::to_fixed(d[0], 1))];
        if has_powers(r) {
            cells.push(cell(&format!("{}%", js::to_fixed(d[1], 0))));
        }
        cells.push(cell(&format!("{}%", js::to_fixed(d[2], 0))));
        if has_negs(r) {
            cells.push(cell(&js::to_fixed(d[3], 1)));
        }
        if r.use_bonuses {
            cells.push(cell(&js::to_fixed(d[4], 2)));
        }
        if r.bonuses_bounce_back {
            cells.push(cell(&format!("{}%", js::to_fixed(d[5], 0))));
            cells.push(cell(&format!("{}%", js::to_fixed(d[6], 0))));
        }
        if r.use_lightning_rounds() {
            cells.push(cell(&js::to_fixed(d[7], 1)));
        }
        cells
    }

    fn round_report_row(&self, rs: &RoundStats, omit_phase: bool) -> String {
        let t = self.t;
        let phase = rs.phase.map(|p| &t.phases[p]);
        let round = match (phase, rs.round) {
            (Some(ph), Some(ri)) => &ph.rounds[ri],
            _ => unreachable!("round report rows always have a round"),
        };
        let mut cells = vec![];
        let numeric = phase.is_some_and(|p| p.uses_numeric_rounds());
        cells.push(text_cell(&if numeric {
            self.scoreboard_round_link(round, &js::num_to_string(round.number))
        } else {
            String::new()
        }));
        if !omit_phase {
            match phase {
                Some(ph) if !ph.uses_numeric_rounds() => {
                    cells.push(text_cell(&self.scoreboard_round_link(&ph.rounds[0], &ph.name)));
                }
                _ => cells.push(match rs.phase {
                    Some(p) => self.colored_phase_cell(&[p]),
                    None => self.colored_phase_cell(&[]),
                }),
            }
        }
        cells.extend(self.round_stat_cells(rs, numeric_cell));
        if t.packet_names_exist() {
            cells.push(text_cell(&round.packet_name));
        }
        tr_tag(&cells)
    }

    fn round_report_footer(&self, total: &RoundStats, omit_phase: bool) -> String {
        let mut cells = vec![header("Total", false, None)];
        if !omit_phase {
            let name = total.phase.map(|p| self.t.phases[p].name.clone()).unwrap_or_default();
            cells.push(header(&name, false, None));
        }
        cells.extend(self.round_stat_cells(total, |s| header(s, true, None)));
        if self.t.packet_names_exist() {
            cells.push(header("", false, None));
        }
        table_footer(&cells)
    }

    /* ---------------- shared pieces ---------------- */

    fn push_tossup_value_headers(&self, cells: &mut Vec<String>) {
        for at in &self.rules().answer_types {
            cells.push(header(&js::num_to_string(at.value), true, Some("5%")));
        }
    }

    fn abbr(&self, stat: Stat) -> String {
        let text = column_name(stat, self.rules());
        let tooltip = column_tooltip(stat, self.rules());
        if tooltip.is_empty() {
            return text;
        }
        tag_attrs("abbr", &[&format!("title=\"{tooltip}\"")], &[&text])
    }

    /// `getTopLinks`
    fn top_links(&self) -> String {
        let cells: Vec<String> =
            PAGE_ORDER.iter().map(|p| td(PLAIN, &a_tag(&self.file_name_for_link(*p), p.link_title()))).collect();
        table_tag(&[tr_tag(&cells)], Some("100%"), None, Some("0"))
    }

    fn scoreboard_round_link(&self, round: &Round, text: &str) -> String {
        a_tag(&format!("{}#{}", self.file_name_for_link(Page::Scoreboard), round_link_id(round)), text)
    }

    fn scoreboard_match_link(&self, m: &Match, text: &str) -> String {
        a_tag(&format!("{}#{}", self.file_name_for_link(Page::Scoreboard), writer::match_id(self.t, m)), text)
    }

    fn team_detail_link(&self, team: TeamId, dont_truncate: bool) -> String {
        let tm = &self.t.teams[team];
        let text = if dont_truncate { tm.name.clone() } else { tm.truncated_name(35) };
        a_tag(&format!("{}#{}", self.file_name_for_link(Page::TeamDetails), alpha_only(&tm.name)), &text)
    }

    fn player_detail_link(&self, player: PlayerId, team: TeamId) -> String {
        let (p, tm) = (&self.t.players[player], &self.t.teams[team]);
        a_tag(&format!("{}#{}", self.file_name_for_link(Page::PlayerDetails), player_detail_link_id(p, tm)), &p.name)
    }

    fn file_name_for_link(&self, page: Page) -> String {
        format!("{}{}", self.file_prefix, page.file_name())
    }

    /// `headerWithDivider`
    fn header_with_divider(&self, text: &str, page: Page, no_top_link: bool, sticky: bool) -> String {
        let h = tag("h2", &[&format!("{text}{NBSP}")]);
        let divider = tag_attrs("div", &[&class_attr(&[css::INLINE_DIVIDER])], &[]);
        let attrs = if sticky {
            class_attr(&[css::HEADER_AND_DIVIDER, css::STICKY_HEADER])
        } else {
            class_attr(&[css::HEADER_AND_DIVIDER])
        };
        if no_top_link {
            return tag_attrs("div", &[&attrs], &[&h, &divider]);
        }
        let top = a_tag(
            &format!("{}#{TOP_ANCHOR_ID}", self.file_name_for_link(page)),
            &tag_attrs("span", &[&class_attr(&[css::SMALL_TEXT])], &[&format!("{}Top", unicode_html("2191"))]),
        );
        tag_attrs("div", &[&attrs], &[&h, &divider, &tag("span", &[NBSP]), &top])
    }

    /// Colour of a phase in the Stage columns: by position among the full phases (`assignPhaseColors`)
    fn phase_color(&self, p: PhaseId) -> &'static str {
        self.t.full_phases().iter().position(|fp| *fp == p).and_then(|i| PHASE_COLORS.get(i)).copied().unwrap_or("")
    }

    /// `coloredPhaseCell`
    fn colored_phase_cell(&self, phases: &[PhaseId]) -> String {
        if phases.is_empty() {
            return text_cell("");
        }
        let mut colors = vec![];
        for p in phases {
            let c = self.phase_color(*p);
            if !c.is_empty() {
                colors.push(c);
            }
            if colors.len() == 2 {
                break;
            }
        }
        let text: Vec<&str> = phases.iter().map(|p| self.t.phases[*p].name.as_str()).collect();
        let style = match colors.len() {
            0 => String::new(),
            1 => format!("background-color:{}", colors[0]),
            _ => format!("background-image: linear-gradient(to bottom right, {} 50%, {} 51%)", colors[0], colors[1]),
        };
        tag_attrs("td", &[&attr("style", &style)], &[&text.join(", ")])
    }
}

/// `provisionalAdvancementTierDisplay`
fn provisional_advancement(ts: &PoolTeamStats) -> String {
    if ts.record_tie_for_advancement {
        return unicode_html("2754");
    }
    match ts.advance_to_tier {
        None => MDASH.to_string(),
        Some(t) => format!("Tier {}", js::num_to_string(t)),
    }
}

fn round_link_id(round: &Round) -> String {
    format!("Round-{}", js::num_to_string(round.number))
}

fn player_detail_link_id(p: &Player, tm: &Team) -> String {
    format!("{}-{}", alpha_only(&tm.name), alpha_only(&p.name))
}

fn has_powers(r: &ScoringRules) -> bool {
    r.answer_types.iter().any(|a| a.is_power())
}

fn has_negs(r: &ScoringRules) -> bool {
    r.answer_types.iter().any(|a| a.is_neg())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn td_spacing_matches_the_template() {
        assert_eq!(td(PLAIN, "x"), "<td    >x</td>");
        assert_eq!(td(RIGHT, "5"), "<td align=\"right\"   >5</td>");
        assert_eq!(header("Rank", false, Some("3%")), "<td    width=\"3%\"><b>\nRank\n</b></td>");
        // an empty width is dropped by stdTdHeader but printed by tdTag
        assert_eq!(header("Team", false, Some("")), "<td    ><b>\nTeam\n</b></td>");
        assert_eq!(td(bold_w(""), "Team"), "<td    width=\"\"><b>\nTeam\n</b></td>");
    }

    #[test]
    fn table_and_tag_spacing() {
        assert_eq!(table_tag(&[], None, None, None), "<table   >\n\n</table>");
        assert_eq!(table_tag(&["<tr>".into()], Some("100%"), None, Some("0")), "<table border=0  width=100%>\n<tr>\n</table>");
        assert_eq!(tag_attrs("div", &["id=x"], &[]), "<div id=x>\n\n</div>");
        assert_eq!(tr_tag(&[]), "<tr>\n\n</tr>");
    }

    #[test]
    fn alpha_only_keeps_unicode_letters_and_digits() {
        assert_eq!(alpha_only("St. Mary's A-2"), "StMarysA2");
        assert_eq!(alpha_only("Łódź Ñ"), "ŁódźÑ");
        // combining marks are dropped (unlike Team.getLinkIdAbbrName)
        assert_eq!(alpha_only("e\u{301}"), "e");
    }

    #[test]
    fn upper_compare_uses_utf16_order() {
        assert_eq!(upper_compare("abc", "ABD"), -1.0);
        assert_eq!(upper_compare("b", "A"), 1.0);
        assert_eq!(upper_compare("x", "X"), 0.0);
        // U+FF21 (fullwidth A) sorts after U+10000 by code point but before it in UTF-16
        assert_eq!(upper_compare("\u{FF21}", "\u{10000}"), 1.0);
        assert_eq!(upper_compare("ab", "abc"), -1.0);
    }

    #[test]
    fn year_display() {
        let p = |y: &str| Player { year_string: y.into(), ..Default::default() };
        assert_eq!(year_display_text(&p("12")), "12th Grade");
        assert_eq!(year_display_text(&p("Jr.")), "Junior");
        assert_eq!(year_display_text(&p("-1")), "undefined");
        assert_eq!(year_display_text(&p("Faculty")), "Faculty");
    }

    #[test]
    fn date_strings() {
        assert_eq!(date_string("not a date"), "Invalid Date");
        // a date-only string is local midnight, so it shows as that day in any time zone
        assert_eq!(date_string("2026-04-17"), "Fri Apr 17 2026");
    }

    #[test]
    fn match_strings_with_null_scores() {
        let teams = vec![
            Team { name: "Alpha".into(), ..Default::default() },
            Team { name: "Beta".into(), ..Default::default() },
        ];
        let mk = |l: Option<f64>, r: Option<f64>| {
            let mut m = Match::new(1.0, MatchTeam::new(Some(0), None, &teams), MatchTeam::new(Some(1), None, &teams));
            m.left.points = l;
            m.right.points = r;
            m
        };
        let m = mk(Some(250.0), Some(-15.0));
        assert_eq!(m.score_string(&teams), "Alpha 250, Beta -15");
        assert_eq!(m.winner_loser_string(&teams), "Alpha def. Beta");
        assert_eq!(m.score_only(Side::Right, true).unwrap(), "(-15) - 250");
        // null === null, but null !== 0, and null > 0 is false
        let mut m = mk(Some(0.0), Some(0.0));
        assert_eq!(m.winner_loser_string(&teams), "Alpha tied Beta");
        m.left.points_null = true;
        assert_eq!(m.winner_loser_string(&teams), "Beta def. Alpha");
        assert_eq!(m.score_string(&teams), "Alpha null, Beta 0");
        assert!(m.score_only(Side::Left, false).is_err());
        m.right.points_null = true;
        assert_eq!(m.winner_loser_string(&teams), "Alpha tied Beta");
        let mut m = mk(Some(100.0), None);
        assert_eq!(m.score_string(&teams), "Alpha vs. Beta");
        assert_eq!(m.winner_loser_string(&teams), "");
        assert_eq!(m.score_only(Side::Left, false).unwrap(), "100 - ");
        m.left.forfeit_loss = true;
        assert_eq!(m.score_string(&teams), "Beta defeats Alpha by forfeit");
        assert_eq!(m.score_only(Side::Left, false).unwrap(), "Forfeit");
    }

    #[test]
    fn file_prefix() {
        let t = Tournament::new();
        let mut g = HtmlReportGenerator::new(&t);
        g.set_file_prefix(Some("report"));
        assert_eq!(g.file_name_for_link(Page::Scoreboard), "report_games.html");
        g.set_file_prefix(Some(""));
        assert_eq!(g.file_name_for_link(Page::Scoreboard), "games.html");
        g.set_file_prefix(None);
        assert_eq!(g.file_name_for_link(Page::RoundReport), "rounds.html");
    }
}
