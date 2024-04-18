/* eslint-disable class-methods-use-this */
/* eslint-disable no-useless-concat */
import { StatReportPages, StatReportPageOrder, StatReportFileNames } from '../Enums';
import { LeftOrRight, NullObjects } from '../Utils/UtilTypes';
import { Match } from './Match';
import { MatchPlayer } from './MatchPlayer';
import { MatchTeam } from './MatchTeam';
import { Phase, PhaseTypes } from './Phase';
import { Player } from './Player';
import { Pool } from './Pool';
import { Round } from './Round';
import {
  PhaseStandings,
  PlayerDetailMatchResult,
  PlayerStats,
  PoolStats,
  PoolTeamStats,
  RoundStats,
  TeamDetailMatchResult,
} from './StatSummaries';
import { Team } from './Team';
// eslint-disable-next-line import/no-cycle
import Tournament from './Tournament';

export default class HtmlReportGenerator {
  tournament: Tournament;

  constructor(tourn: Tournament) {
    this.tournament = tourn;
  }

  // Public functions for generating entire pages

  generateStandingsPage() {
    return this.generateHtmlPage('Team Standings', this.getStandingsHtml());
  }

  generateIndividualsPage() {
    return this.generateHtmlPage('Individuals', this.getIndividualsHtml());
  }

  generateScoreboardPage() {
    return this.generateHtmlPage('Scoreboard', this.getScoreboardHtml());
  }

  generateTeamDetailPage() {
    return this.generateHtmlPage('Team Detail', this.getTeamDetailHtml());
  }

  generatePlayerDetailPage() {
    return this.generateHtmlPage('Player Detail', this.getPlayerDetailHtml());
  }

  generateRoundReportPage() {
    return this.generateHtmlPage('Round Report', this.getRoundReportHtml());
  }

  /**
   * The entire contents of one html document
   * @param title Title of the top header of the page
   * @param data  html contents of the page's data
   */
  private generateHtmlPage(title: string, data: string) {
    const htmlHeader = getHtmlHeader(title);
    const topLinks = getTopLinks();
    const mainHeader = genericTagWithAttributes('h1', [id(topAnchorID)], title);
    const style = getPageStyle();

    const body = genericTag('BODY', topLinks, mainHeader, style, data, madeWithYellowFruit());
    return genericTag('HTML', htmlHeader, body);
  }

  /** The actual data of the Standings page */
  private getStandingsHtml() {
    const sections: string[] = [];
    const prelims = this.tournament.stats[0];
    if (!prelims) return '';

    sections.push(this.generalMetaData());

    if (this.tournament.finalRankingsReady) {
      const header = headerWithDivider('Final Rankings', true);
      sections.push(`${header}\n${this.cumulativeStandingsTable()}`);
    }

    sections.push(this.finalsList().join('\n'));

    let printedPhaseCount = 0;
    for (let i = this.tournament.stats.length - 1; i >= 0; i--) {
      const phaseStats = this.tournament.stats[i];
      if (!phaseStats.phase.anyTeamsAssigned()) continue;

      printedPhaseCount++;
      const header = headerWithDivider(
        phaseStats.phase.name,
        !this.tournament.finalRankingsReady && printedPhaseCount === 1,
      );
      sections.push(`${header}\n${this.standingsForOnePhase(phaseStats)}`);
    }

    if (printedPhaseCount > 1 && !this.tournament.finalRankingsReady) {
      const header = headerWithDivider('Cumulative');
      sections.push(`${header}\n${this.cumulativeStandingsTable(true)}`);
    }
    return sections.join('\n');
  }

  private generalMetaData() {
    const firstLineSegments: string[] = [];
    if (this.tournament.name) firstLineSegments.push(genericTag('span', this.tournament.name));
    if (this.tournament.startDate !== NullObjects.nullDate) {
      const sDate = this.tournament.startDate.toDateString();
      if (this.tournament.endDate !== NullObjects.nullDate) {
        firstLineSegments.push(genericTag('span', `${sDate} ${ndash} ${this.tournament.endDate.toDateString()}`));
      } else {
        firstLineSegments.push(genericTag('span', sDate));
      }
    }
    if (this.tournament.tournamentSite.name) {
      firstLineSegments.push(genericTag('span', this.tournament.tournamentSite.name));
    }
    if (this.tournament.questionSet) {
      firstLineSegments.push(genericTag('span', `Question set: ${this.tournament.questionSet}`));
    }
    return firstLineSegments.join(genericTag('span', `${emsp}|${emsp}`));
  }

  private standingsForOnePhase(phaseStandings: PhaseStandings) {
    const tables: string[] = [];
    const nextPhase = this.tournament.getNextFullPhase(phaseStandings.phase);
    const tbPhase = this.tournament.getTiebreakerPhaseFor(phaseStandings.phase);

    for (const onePool of phaseStandings.pools) {
      const header = phaseStandings.pools.length > 1 ? genericTag('h3', onePool.pool.name) : '';
      tables.push(`${header}\n${this.oneStandingsTable(onePool, phaseStandings.anyTiesExist, tbPhase, nextPhase)}`);
    }
    return tables.join('\n') + lineBreak;
  }

  private oneStandingsTable(poolStats: PoolStats, anyTiesExist: boolean, tbPhase?: Phase, nextPhase?: Phase) {
    const rows = [this.standingsHeader(anyTiesExist, nextPhase)];
    for (const teamStats of poolStats.poolTeams) {
      rows.push(this.standingsRow(teamStats, anyTiesExist, nextPhase));
    }
    return `${tableTag(rows, undefined, '100%')}\n${this.tiebreakerList(tbPhase, poolStats.pool)}`;
  }

  private cumulativeStandingsTable(omitRank: boolean = false) {
    const stats = this.tournament.cumulativeStats;
    if (!stats) return '';
    const rows = [this.standingsHeader(stats.anyTiesExist, undefined, true, omitRank)];
    for (const teamStats of stats.teamStats) {
      rows.push(this.standingsRow(teamStats, stats.anyTiesExist, undefined, true, omitRank));
    }
    return tableTag(rows, undefined, '75%');
  }

  private standingsHeader(
    anyTiesExist: boolean,
    nextPhase?: Phase,
    cumulative: boolean = false,
    omitRank: boolean = false,
  ) {
    const cells: string[] = [];
    if (!omitRank) cells.push(tdTag({ bold: true, width: '3%' }, 'Rank'));
    cells.push(tdTag({ bold: true, width: cumulative ? '' : '20%' }, 'Team'));
    if (this.tournament.trackSmallSchool) cells.push(tdTag({ bold: true }, 'SS'));
    if (this.tournament.trackJV) cells.push(tdTag({ bold: true }, 'JV'));
    if (this.tournament.trackUG) cells.push(tdTag({ bold: true }, 'UG'));
    if (this.tournament.trackDiv2) cells.push(tdTag({ bold: true }, 'D2'));
    cells.push(tdTag({ bold: true, align: 'right', width: '3%' }, 'W'));
    cells.push(tdTag({ bold: true, align: 'right', width: '3%' }, 'L'));
    if (anyTiesExist) cells.push(tdTag({ bold: true, align: 'right', width: '3%' }, 'T'));
    if (!cumulative) cells.push(tdTag({ bold: true, align: 'right' }, 'Pct'));
    cells.push(
      tdTag({ bold: true, align: 'right', width: '8%' }, `PP${this.tournament.scoringRules.regulationTossupCount}TUH`),
    );
    this.pushTossupValueHeaders(cells);
    cells.push(tdTag({ bold: true, align: 'right' }, 'TUH'));
    cells.push(tdTag({ bold: true, align: 'right' }, 'PPB'));
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(tdTag({ bold: true, align: 'right' }, 'BB%'));
    }
    if (nextPhase?.anyTeamsAssigned()) {
      cells.push(tdTag({ bold: true }, 'Advanced To'));
    } else if (nextPhase) {
      cells.push(tdTag({ bold: true }, 'Would Advance'));
    }

    return trTag(cells);
  }

  private standingsRow(
    teamStats: PoolTeamStats,
    anyTiesExist: boolean,
    nextPhase?: Phase,
    cumulative: boolean = false,
    omitRank: boolean = false,
  ) {
    const cells: string[] = [];
    if (!omitRank && cumulative) cells.push(tdTag({}, teamStats.team.getOverallRankString()));
    else if (!omitRank && !cumulative) cells.push(tdTag({}, teamStats.rank));

    cells.push(textCell(teamDetailLink(teamStats.team)));
    if (this.tournament.trackSmallSchool) {
      const isSS = this.tournament.findRegistrationByTeam(teamStats.team)?.isSmallSchool;
      cells.push(tdTag({}, isSS ? 'SS' : ''));
    }
    if (this.tournament.trackJV) cells.push(tdTag({}, teamStats.team.isJV ? 'JV' : ''));
    if (this.tournament.trackUG) cells.push(tdTag({}, teamStats.team.isUG ? 'UG' : ''));
    if (this.tournament.trackDiv2) cells.push(tdTag({}, teamStats.team.isD2 ? 'D2' : ''));
    cells.push(tdTag({ align: 'right' }, teamStats.wins.toString()));
    cells.push(tdTag({ align: 'right' }, teamStats.losses.toString()));
    if (anyTiesExist) cells.push(tdTag({ align: 'right' }, teamStats.ties.toString()));

    if (!cumulative) {
      const pct = teamStats.getWinPct();
      const pctStr = Number.isNaN(pct) ? mDashHtml : pct.toFixed(3).toString();
      cells.push(tdTag({ align: 'right' }, pctStr));
    }

    const ppgStr =
      teamStats.totalPointsForPPG === 0
        ? mDashHtml
        : (teamStats.getPtsPerRegTuh() * this.tournament.scoringRules.regulationTossupCount).toFixed(1);
    cells.push(tdTag({ align: 'right' }, ppgStr));

    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = teamStats.tossupCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(tdTag({ align: 'right' }, answerCount?.number?.toString() || '0'));
    });
    cells.push(tdTag({ align: 'right' }, teamStats.tuhRegulation.toString()));

    if (this.tournament.scoringRules.useBonuses) {
      const ppb = teamStats.getPtsPerBonus();
      const ppbStr = Number.isNaN(ppb) ? mDashHtml : ppb.toFixed(2);
      cells.push(tdTag({ align: 'right' }, ppbStr));
    }

    if (this.tournament.scoringRules.bonusesBounceBack) {
      const bbConv = teamStats.getBouncebackConvPctString();
      const bbConvStr = bbConv === '-' ? mDashHtml : bbConv;
      cells.push(tdTag({ align: 'right' }, bbConvStr));
    }

    if (nextPhase?.anyTeamsAssigned()) {
      cells.push(tdTag({}, this.definiteAdvancementTierDisplay(teamStats, nextPhase)));
    } else if (nextPhase) {
      cells.push(tdTag({}, this.provisionalAdvancementTierDisplay(teamStats)));
    }

    return trTag(cells);
  }

  private provisionalAdvancementTierDisplay(teamStats: PoolTeamStats) {
    if (teamStats.recordTieForAdvancement) return unicodeHTML('2754');
    if (teamStats.advanceToTier === undefined) return mDashHtml;
    return `Tier ${teamStats.advanceToTier}`;
  }

  private definiteAdvancementTierDisplay(teamStats: PoolTeamStats, nextPhase: Phase) {
    if (teamStats.poolTeam.didNotAdvance) return 'None';
    return nextPhase.findPoolWithTeam(teamStats.team)?.name || '';
  }

  private finalsList() {
    return this.tournament.getFinalsPhases().map((ph) => {
      const matchList = this.tiebreakerList(ph);
      if (matchList === '') return '';
      return `${headerWithDivider(ph.name)}\n${matchList}`;
    });
  }

  private tiebreakerList(tbOrFinalsPhase: Phase | undefined, pool?: Pool) {
    if (!tbOrFinalsPhase) return '';
    const matches = tbOrFinalsPhase.getMatchesForPool(pool);
    if (matches.length === 0) return '';
    const title = tbOrFinalsPhase.phaseType === PhaseTypes.Tiebreaker ? genericTag('span', 'Tiebreakers:') : '';
    const list = unorderedList(matches.map((m) => m.getWinnerLoserString()));
    return genericTagWithAttributes('div', [classAttribute(cssClasses.smallText)], title, list);
  }

  /** The actual data of the individuals page */
  private getIndividualsHtml() {
    const sections: string[] = [];
    const prelims = this.tournament.stats[0];
    if (!prelims) return '';

    const prelimsHeader = headerWithDivider(prelims.phase.name, true);
    sections.push(`${prelimsHeader}\n${this.individualsTable(prelims.players)}`);

    if (this.tournament.stats.length > 1 && this.tournament.cumulativeStats) {
      const aggregateHeader = headerWithDivider('All Games');
      sections.push(`${aggregateHeader}\n${this.individualsTable(this.tournament.cumulativeStats.players, true)}`);
    }

    return sections.join('\n');
  }

  private individualsTable(players: PlayerStats[], skipRankCol: boolean = false) {
    const rows = [this.individualsHeader(skipRankCol)];
    for (const playerStats of players) {
      if (playerStats.tossupsHeard === 0) continue;
      rows.push(this.individualsRow(playerStats, skipRankCol));
    }
    return tableTag(rows, undefined, '80%');
  }

  private individualsHeader(skipRankCol: boolean = false) {
    const cells: string[] = [];
    if (!skipRankCol) cells.push(tdTag({ bold: true, width: '3%' }, 'Rank'));
    cells.push(stdTdHeader('Player'));
    if (this.tournament.trackPlayerYear) cells.push(stdTdHeader('Year/Grade'));
    if (this.tournament.trackUG) cells.push(stdTdHeader('UG'));
    if (this.tournament.trackDiv2) cells.push(stdTdHeader('D2'));
    cells.push(stdTdHeader('Team'));
    cells.push(stdTdHeader('GP', true));
    this.pushTossupValueHeaders(cells);
    cells.push(stdTdHeader('TUH', true));
    cells.push(stdTdHeader(`PP${this.tournament.scoringRules.regulationTossupCount}TUH`, true));

    return trTag(cells);
  }

  private individualsRow(playerStats: PlayerStats, skipRankCol: boolean = false) {
    const cells: string[] = [];
    if (!skipRankCol) cells.push(tdTag({}, playerStats.rank + (playerStats.rankTie ? '=' : '')));
    cells.push(textCell(playerDetailLink(playerStats.player, playerStats.team)));
    if (this.tournament.trackPlayerYear) cells.push(tdTag({}, playerStats.player.yearString));
    if (this.tournament.trackUG) cells.push(tdTag({}, playerStats.player.isUG ? 'UG' : ''));
    if (this.tournament.trackDiv2) cells.push(tdTag({}, playerStats.player.isD2 ? 'D2' : ''));
    cells.push(textCell(teamDetailLink(playerStats.team)));
    cells.push(tdTag({ align: 'right' }, playerStats.gamesPlayed.toFixed(1)));
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = playerStats.tossupCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(tdTag({ align: 'right' }, answerCount?.number?.toString() || '0'));
    });
    cells.push(tdTag({ align: 'right' }, playerStats.tossupsHeard.toString()));

    const pptuh = playerStats.getPptuh();
    cells.push(
      tdTag(
        { align: 'right' },
        pptuh !== undefined ? (pptuh * this.tournament.scoringRules.regulationTossupCount).toFixed(2) : mDashHtml,
      ),
    );

    return trTag(cells);
  }

  getScoreboardHtml() {
    const rounds: string[] = [];
    const tocItems: string[] = [];
    for (const phase of this.tournament.phases) {
      let phaseHasGames = false;
      for (const round of phase.rounds) {
        if (round.matches.length > 0) {
          rounds.push(this.oneRoundOfBoxScores(round, phase));
          if (!phaseHasGames) {
            phaseHasGames = true;
            if (phase.usesNumericRounds()) {
              tocItems.push(phase.name);
            } else {
              tocItems.push(scoreboardRoundLink(round, phase.name));
            }
          }
          if (phase.usesNumericRounds()) {
            tocItems.push(`${nbsp}${nbsp}${scoreboardRoundLink(round, round.displayName(true))}`);
          }
        }
      }
    }
    const toc = genericTagWithAttributes('div', [classAttribute(cssClasses.floatingTOC)], unorderedList(tocItems));
    return `${toc}\n${rounds.join('\n')}`;
  }

  private oneRoundOfBoxScores(round: Round, phase: Phase) {
    const segments: string[] = [];
    if (round.number !== 1) segments.push('<br /><br />');
    let title = round.displayName(false);
    if (phase.isFullPhase()) title += ` - ${phase.name}`;
    segments.push(genericTagWithAttributes('div', [id(roundLinkId(round))]));
    segments.push(headerWithDivider(title, round.number === 1));
    for (const match of round.matches) {
      segments.push(this.boxScore(match));
    }
    return segments.join('\n');
  }

  private boxScore(match: Match) {
    const segments: string[] = [];
    segments.push(genericTagWithAttributes('div', [id(matchLinkId(match))]));
    segments.push(genericTag('h3', match.getScoreString()));
    if (match.isForfeit()) return segments.join('\n');

    if (match.carryoverPhases.length > 0) {
      segments.push(
        genericTagWithAttributes(
          'p',
          [classAttribute(cssClasses.smallText)],
          `Carries over to: ${match.carryoverPhases.map((ph) => ph.name).join(', ')}`,
        ),
      );
    }
    let tuReadStr = `Tossups read: ${match.tossupsRead ?? 'unknown'}`;
    if (match.overtimeTossupsRead) tuReadStr += ` (${match.overtimeTossupsRead} in OT)`;
    segments.push(genericTagWithAttributes('p', [classAttribute(cssClasses.smallText)], tuReadStr));

    const leftTable = this.boxScoreTableOneTeam(match.leftTeam);
    const rightTable = this.boxScoreTableOneTeam(match.rightTeam);
    segments.push(
      genericTagWithAttributes('div', [classAttribute(cssClasses.boxScoreTableContainer)], leftTable, rightTable),
    );

    if (this.tournament.scoringRules.useBonuses) {
      segments.push('<br />');
      segments.push(this.boxScoreBonusTable(match));
    }
    if (this.tournament.scoringRules.bonusesBounceBack) segments.push(this.boxScoreBouncebackTable(match));
    return segments.join('\n');
  }

  private boxScoreTableOneTeam(matchTeam: MatchTeam) {
    const rows = [this.boxScoreTableHeader(matchTeam.team?.name ?? '')];
    for (const mp of matchTeam.getActiveMatchPlayers()) {
      rows.push(this.boxScoreOneMatchPlayer(mp));
    }
    rows.push(this.boxScoreTotalRow(matchTeam));
    return tableTag(rows, undefined, '35%');
  }

  private boxScoreTableHeader(teamName: string) {
    const cells: string[] = [];
    cells.push(tdTag({ bold: true }, teamName));
    cells.push(tdTag({ bold: true, align: 'right' }, 'TUH'));
    this.pushTossupValueHeaders(cells);
    cells.push(tdTag({ bold: true, align: 'right' }, 'Tot'));
    return trTag(cells);
  }

  private boxScoreOneMatchPlayer(matchPlayer: MatchPlayer) {
    const cells: string[] = [];
    cells.push(tdTag({}, matchPlayer.player.name));
    cells.push(tdTag({ align: 'right' }, matchPlayer.tossupsHeard?.toString() ?? '0'));
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = matchPlayer.answerCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(tdTag({ align: 'right' }, answerCount?.number?.toString() ?? '0'));
    });
    cells.push(tdTag({ align: 'right' }, matchPlayer.points.toString()));
    return trTag(cells);
  }

  private boxScoreTotalRow(matchTeam: MatchTeam) {
    const cells = [tdTag({ bold: true }, 'Total')];
    cells.push(tdTag({}, ''));
    const answerCounts = matchTeam.getAnswerCounts();
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = answerCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(tdTag({ bold: true, align: 'right' }, answerCount?.number?.toString() ?? '0'));
    });
    cells.push(tdTag({ bold: true, align: 'right' }, matchTeam.getTossupPoints().toString()));
    return tableFooter(cells);
  }

  private boxScoreBonusTable(match: Match) {
    const rows: string[] = [];
    rows.push(this.boxScoreBonusTableHeader());
    rows.push(this.boxScoreBonusTableRow(match.leftTeam));
    rows.push(this.boxScoreBonusTableRow(match.rightTeam));
    return tableTag(rows, undefined, '50%');
  }

  private boxScoreBonusTableHeader() {
    const cells: string[] = [];
    cells.push(tdTag({ bold: true, width: '40%' }, 'Bonuses'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'Heard'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'Pts'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'PPB'));
    return trTag(cells);
  }

  private boxScoreBonusTableRow(matchTeam: MatchTeam) {
    const cells: string[] = [];
    const [pts, heard, ppb] = matchTeam.getBonusStats(this.tournament.scoringRules);
    cells.push(tdTag({}, matchTeam.team?.name ?? ''));
    cells.push(tdTag({ align: 'right' }, heard));
    cells.push(tdTag({ align: 'right' }, pts));
    cells.push(tdTag({ align: 'right' }, ppb));
    return trTag(cells);
  }

  private boxScoreBouncebackTable(match: Match) {
    const rows: string[] = [];
    rows.push(this.boxScoreBouncebackTableHeader());
    rows.push(this.boxScoreBouncebackTableRow(match, 'left'));
    rows.push(this.boxScoreBouncebackTableRow(match, 'right'));
    return tableTag(rows, undefined, '50%');
  }

  private boxScoreBouncebackTableHeader() {
    const cells: string[] = [];
    cells.push(tdTag({ bold: true, width: '40%' }, 'Bouncebacks'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'Parts Heard'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'Pts'));
    cells.push(tdTag({ bold: true, align: 'right', width: '20%' }, 'Success%'));
    return trTag(cells);
  }

  private boxScoreBouncebackTableRow(match: Match, whichTeam: LeftOrRight) {
    const cells: string[] = [];
    const [heard, rate] = match.getBouncebackStatsString(whichTeam, this.tournament.scoringRules);
    const matchTeam = match.getMatchTeam(whichTeam);
    cells.push(tdTag({}, matchTeam.team?.name ?? ''));
    cells.push(tdTag({ align: 'right' }, heard));
    cells.push(tdTag({ align: 'right' }, matchTeam.bonusBouncebackPoints?.toString() ?? '0'));
    cells.push(tdTag({ align: 'right' }, `${rate}%`));
    return trTag(cells);
  }

  private getTeamDetailHtml() {
    if (!this.tournament.cumulativeStats) return '';

    const teamListCopy = this.tournament.cumulativeStats.teamStats.slice();
    teamListCopy.sort((a, b) => {
      const aName = a.team.name.toLocaleUpperCase();
      const bName = b.team.name.toLocaleUpperCase();
      if (aName < bName) return -1;
      if (aName > bName) return 1;
      return 0;
    });

    return teamListCopy.map((tStats) => this.teamDetailOneTeam(tStats)).join('\n');
  }

  private teamDetailOneTeam(teamStats: PoolTeamStats) {
    const segments = [];
    segments.push(genericTagWithAttributes('h2', [id(teamDetailLinkId(teamStats.team))], teamStats.team.name));
    segments.push(this.teamDetailMatchTable(teamStats));
    segments.push('<br />');
    segments.push(this.teamDetailPlayerTable(teamStats.team));
    return segments.join('\n');
  }

  private teamDetailMatchTable(teamStats: PoolTeamStats) {
    const omitPhaseCol = this.tournament.getFullPhases().length < 2;
    const rows = [this.teamDetailMatchTableHeader(omitPhaseCol)];
    for (const result of teamStats.matches) {
      rows.push(this.teamDetailMatchTableRow(result, omitPhaseCol));
    }
    rows.push(this.teamDetailMatchTableFooter(teamStats, omitPhaseCol));
    return tableTag(rows, undefined, '100%');
  }

  private teamDetailMatchTableHeader(omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(tdTag({ bold: true, width: '5%' }, 'Round'));
    if (!omitPhase) cells.push(tdTag({ bold: true, width: '12%' }, 'Stage'));
    if (!omitPhase && this.tournament.hasAnyCarryover()) {
      cells.push(tdTag({ bold: true, width: '8%' }, 'Carried To'));
    }
    cells.push(stdTdHeader('Opponent'));
    cells.push(stdTdHeader('')); // win/loss
    cells.push(stdTdHeader('Score'));
    this.pushTossupValueHeaders(cells);
    cells.push(stdTdHeader('TUH', true));
    if (this.tournament.scoringRules.useBonuses) {
      cells.push(stdTdHeader('BHrd', true));
      cells.push(stdTdHeader('BPts', true));
      cells.push(stdTdHeader('PPB', true));
    }
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(stdTdHeader('BBHrd', true));
      cells.push(stdTdHeader('BBPts', true));
      cells.push(stdTdHeader('BB%', true));
    }
    return trTag(cells);
  }

  private teamDetailMatchTableRow(result: TeamDetailMatchResult, omitPhase: boolean = false) {
    const cells: string[] = [];
    const matchTeam = result.match.getMatchTeam(result.whichTeam);
    const opponent = result.match.getOpponent(result.whichTeam);
    const forf = result.match.isForfeit();
    if (!matchTeam.team || !opponent.team) return trTag([]);

    cells.push(textCell(result.phase?.usesNumericRounds() ? result.round.number.toString() : ''));
    if (!omitPhase) cells.push(textCell(result.phase?.name ?? ''));
    if (!omitPhase && this.tournament.hasAnyCarryover()) {
      cells.push(textCell(result.match.carryoverPhases.map((ph) => ph.name).join(', ')));
    }
    cells.push(textCell(teamDetailLink(opponent.team)));
    cells.push(textCell(result.match.getResultDisplay(result.whichTeam)));
    cells.push(textCell(scoreboardMatchLink(result.match, result.match.getScoreOnly(result.whichTeam))));

    const answerCounts = matchTeam.getAnswerCounts();
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      if (forf) {
        cells.push(numericCell(''));
        return;
      }
      const answerCount = answerCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(numericCell(answerCount?.number?.toString() ?? '0'));
    });

    cells.push(numericCell(result.match.tossupsRead?.toString() ?? ''));
    if (this.tournament.scoringRules.useBonuses) {
      const [pts, heard, ppb] = matchTeam.getBonusStats(this.tournament.scoringRules);
      cells.push(numericCell(forf ? '' : heard));
      cells.push(numericCell(forf ? '' : pts));
      cells.push(numericCell(forf ? '' : ppb));
    }
    if (this.tournament.scoringRules.bonusesBounceBack) {
      const [heard, rate] = result.match.getBouncebackStatsString(result.whichTeam, this.tournament.scoringRules);
      cells.push(numericCell(forf ? '' : heard));
      cells.push(numericCell(forf ? '' : matchTeam.bonusBouncebackPoints?.toString() ?? '0'));
      cells.push(numericCell(forf ? '' : `${rate}%`));
    }
    return trTag(cells);
  }

  private teamDetailMatchTableFooter(teamStats: PoolTeamStats, omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(textCell('')); // round no.
    if (!omitPhase) cells.push(textCell(''));
    if (!omitPhase && this.tournament.hasAnyCarryover()) cells.push(textCell('')); // carryover phases
    cells.push(stdTdHeader('Total')); // below Opponent column
    cells.push(stdTdHeader(teamStats.getRecord()));
    cells.push(stdTdHeader(''));
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = teamStats.tossupCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(stdTdHeader(answerCount?.number?.toString() || '0', true));
    });
    cells.push(stdTdHeader(teamStats.tuhTotal?.toString() ?? '0', true));
    if (this.tournament.scoringRules.useBonuses) {
      const ppb = teamStats.getPtsPerBonus();
      const ppbStr = Number.isNaN(ppb) ? mDashHtml : ppb.toFixed(2);
      cells.push(stdTdHeader(teamStats.bonusesHeard?.toString() ?? '0', true));
      cells.push(stdTdHeader(teamStats.bonusPoints?.toString() ?? '0', true));
      cells.push(stdTdHeader(ppbStr, true));
    }
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(stdTdHeader(teamStats.bounceBackPartsHeard?.toString() ?? '0', true));
      cells.push(stdTdHeader(teamStats.bounceBackPartsHeard?.toString() ?? '0', true));
      cells.push(stdTdHeader(teamStats.getBouncebackConvPctString(), true));
    }
    return tableFooter(cells);
  }

  private teamDetailPlayerTable(team: Team) {
    const playersOnTeam = this.tournament.cumulativeStats?.players.filter((plSt) => plSt.team === team);
    if (!playersOnTeam || playersOnTeam.length === 0) return '';

    const rows = [this.teamDetailPlayerTableHeader()];
    for (const plSt of playersOnTeam) {
      rows.push(this.teamDetailPlayerTableRow(plSt));
    }
    return tableTag(rows, undefined, '70%');
  }

  private teamDetailPlayerTableHeader() {
    const cells: string[] = [];
    cells.push(stdTdHeader('Player'));
    if (this.tournament.trackPlayerYear) cells.push(stdTdHeader('Year/Grade'));
    if (this.tournament.trackUG) cells.push(stdTdHeader('UG'));
    if (this.tournament.trackDiv2) cells.push(stdTdHeader('D2'));
    cells.push(stdTdHeader('GP', true));
    this.pushTossupValueHeaders(cells);
    cells.push(stdTdHeader('TUH', true));
    cells.push(stdTdHeader(`PP${this.tournament.scoringRules.regulationTossupCount}TUH`, true));
    return trTag(cells);
  }

  private teamDetailPlayerTableRow(playerStats: PlayerStats) {
    const cells: string[] = [];
    cells.push(textCell(playerDetailLink(playerStats.player, playerStats.team)));
    if (this.tournament.trackPlayerYear) cells.push(textCell(playerStats.player.yearString));
    if (this.tournament.trackUG) cells.push(textCell(playerStats.player.isUG ? 'UG' : ''));
    if (this.tournament.trackDiv2) cells.push(textCell(playerStats.player.isD2 ? 'D2' : ''));
    cells.push(numericCell(playerStats.gamesPlayed.toFixed(1)));
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = playerStats.tossupCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(numericCell(answerCount?.number?.toString() || '0'));
    });
    cells.push(numericCell(playerStats.tossupsHeard.toString()));

    const pptuh = playerStats.getPptuh();
    cells.push(
      numericCell(
        pptuh !== undefined ? (pptuh * this.tournament.scoringRules.regulationTossupCount).toFixed(2) : mDashHtml,
      ),
    );

    return trTag(cells);
  }

  private getPlayerDetailHtml() {
    if (!this.tournament.cumulativeStats) return '';

    const playerListCopy = this.tournament.cumulativeStats.players.slice();
    playerListCopy.sort((a, b) => {
      const aTeam = a.team.name.toLocaleUpperCase();
      const bTeam = b.team.name.toLocaleUpperCase();
      if (aTeam < bTeam) return -1;
      if (aTeam > bTeam) return 1;
      const aPlName = a.player.name.toLocaleUpperCase();
      const bPlName = b.player.name.toLocaleUpperCase();
      if (aPlName < bPlName) return -1;
      if (aPlName > bPlName) return 1;
      return 0;
    });

    return playerListCopy.map((pStats) => this.playerDetailOnePlayer(pStats)).join('\n');
  }

  private playerDetailOnePlayer(playerStats: PlayerStats) {
    const segments = [];
    const anchorId = id(playerDetailLinkId(playerStats.player, playerStats.team));
    segments.push(genericTagWithAttributes('h2', [anchorId], `${playerStats.player.name}, ${playerStats.team.name}`));
    segments.push(this.playerDetailTable(playerStats));
    return segments.join('\n');
  }

  private playerDetailTable(playerStats: PlayerStats) {
    const omitPhaseCol = this.tournament.getFullPhases().length < 2;
    const rows = [this.playerDetailTableHeader(omitPhaseCol)];
    for (const result of playerStats.matches) {
      rows.push(this.playerDetailTableRow(result, omitPhaseCol));
    }
    rows.push(this.playerDetailTableFooter(playerStats, omitPhaseCol));
    return tableTag(rows, undefined, '80%');
  }

  private playerDetailTableHeader(omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(tdTag({ bold: true, width: '5%' }, 'Round'));
    if (!omitPhase) cells.push(tdTag({ bold: true, width: '15%' }, 'Stage'));
    cells.push(stdTdHeader('Opponent'));
    cells.push(stdTdHeader('')); // win/loss
    cells.push(stdTdHeader('Score'));
    cells.push(stdTdHeader('GP', true));
    this.pushTossupValueHeaders(cells);
    cells.push(stdTdHeader('TUH', true));
    cells.push(stdTdHeader('Pts', true));
    return trTag(cells);
  }

  private playerDetailTableRow(result: PlayerDetailMatchResult, omitPhase: boolean = false) {
    const cells: string[] = [];
    const matchTeam = result.match.getMatchTeam(result.whichTeam);
    const opponent = result.match.getOpponent(result.whichTeam);
    const forf = result.match.isForfeit();
    if (!matchTeam.team || !opponent.team) return trTag([]);

    cells.push(textCell(result.phase?.usesNumericRounds() ? result.round.number.toString() : ''));
    if (!omitPhase) cells.push(textCell(result.phase?.name ?? ''));
    cells.push(textCell(teamDetailLink(opponent.team)));
    cells.push(textCell(result.match.getResultDisplay(result.whichTeam)));
    cells.push(textCell(scoreboardMatchLink(result.match, result.match.getScoreOnly(result.whichTeam))));
    const gamesPlayed = (result.matchPlayer.tossupsHeard ?? 0) / (result.match.tossupsRead ?? 0);
    cells.push(numericCell(forf ? '' : gamesPlayed.toFixed(1)));

    this.tournament.scoringRules.answerTypes.forEach((at) => {
      if (forf) {
        cells.push(numericCell(''));
        return;
      }
      const answerCount = result.matchPlayer.answerCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(numericCell(answerCount?.number?.toString() ?? '0'));
    });

    cells.push(numericCell(forf ? '' : result.matchPlayer.tossupsHeard?.toString() ?? ''));
    cells.push(numericCell(forf ? '' : result.matchPlayer.points.toString()));
    return trTag(cells);
  }

  private playerDetailTableFooter(playerStats: PlayerStats, omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(textCell('')); // round
    if (!omitPhase) cells.push(textCell(''));
    cells.push(stdTdHeader('Total')); // underneath opponent col
    cells.push(textCell('')); // result
    cells.push(textCell('')); // score
    cells.push(stdTdHeader(playerStats.gamesPlayed.toFixed(1), true));
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      const answerCount = playerStats.tossupCounts.find((ac) => ac.answerType.value === at.value);
      cells.push(stdTdHeader(answerCount?.number?.toString() || '0', true));
    });
    cells.push(stdTdHeader(playerStats.tossupsHeard.toString(), true));
    cells.push(stdTdHeader(playerStats.getTotalPoints().toString(), true));
    return tableFooter(cells);
  }

  private getRoundReportHtml() {
    if (!this.tournament.cumulativeStats) return '';

    const omitPhaseCol = this.tournament.getFullPhases().length < 2;
    const rows = [this.roundReportTableHeader(omitPhaseCol)];
    for (const roundStats of this.tournament.cumulativeStats.rounds) {
      rows.push(this.roundReportTableRow(roundStats, omitPhaseCol));
    }
    rows.push(this.roundReportTableFooter(this.tournament.cumulativeStats.roundReportTotalStats, omitPhaseCol));
    return tableTag(rows, undefined, '100%');
  }

  private roundReportTableHeader(omitPhase: boolean = false) {
    const cells: string[] = [];
    const columnWidth = omitPhase ? '11%' : '10%';
    cells.push(stdTdHeader('Round'));
    if (!omitPhase) cells.push(stdTdHeader('Stage', false, '15%'));
    cells.push(stdTdHeader('Games', true, '10%'));
    cells.push(stdTdHeader(`Pts/Team/${this.tournament.scoringRules.regulationTossupCount}TUH`, true, columnWidth));
    if (this.tournament.scoringRules.hasPowers()) cells.push(stdTdHeader('TU Powered', true, columnWidth));
    cells.push(stdTdHeader('TU Converted', true, columnWidth));
    if (this.tournament.scoringRules.hasNegs()) {
      cells.push(stdTdHeader(`Negs/Team/${this.tournament.scoringRules.regulationTossupCount}TUH`, true, columnWidth));
    }
    if (this.tournament.scoringRules.useBonuses) cells.push(stdTdHeader('PPB', true, columnWidth));
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(stdTdHeader('BB%', true, columnWidth));
      cells.push(stdTdHeader('Bonus%', true, columnWidth));
    }
    return trTag(cells);
  }

  private roundReportTableRow(stats: RoundStats, omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(
      textCell(stats.phase?.usesNumericRounds() ? scoreboardRoundLink(stats.round, stats.round.number.toString()) : ''),
    );
    if (!omitPhase) {
      if (stats.phase && !stats.phase.usesNumericRounds()) {
        cells.push(textCell(scoreboardRoundLink(stats.phase.rounds[0], stats.phase.name)));
      } else {
        cells.push(textCell(stats.phase?.name ?? ''));
      }
    }
    cells.push(numericCell(stats.games.toString()));
    cells.push(numericCell(stats.getPointsPerXTuh().toFixed(1)));
    if (this.tournament.scoringRules.hasPowers()) cells.push(numericCell(`${stats.getPowerPct().toFixed(0)}%`));
    cells.push(numericCell(`${stats.getTossupConversionPct().toFixed(0)}%`));
    if (this.tournament.scoringRules.hasNegs()) {
      cells.push(numericCell(stats.getNegsPerXTuh().toFixed(1)));
    }
    if (this.tournament.scoringRules.useBonuses) cells.push(numericCell(stats.getPointsPerBonus().toFixed(2)));
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(numericCell(`${stats.getBounceBackConvPct().toFixed(0)}%`));
      cells.push(numericCell(`${stats.getTotalBonusConvPct().toFixed(0)}%`));
    }
    return trTag(cells);
  }

  private roundReportTableFooter(tournTotals: RoundStats, omitPhase: boolean = false) {
    const cells: string[] = [];
    cells.push(stdTdHeader('Total'));
    if (!omitPhase) cells.push(stdTdHeader(tournTotals.phase?.name ?? ''));
    cells.push(stdTdHeader(tournTotals.games.toString(), true));
    cells.push(stdTdHeader(tournTotals.getPointsPerXTuh().toFixed(1), true));
    if (this.tournament.scoringRules.hasPowers()) {
      cells.push(stdTdHeader(`${tournTotals.getPowerPct().toFixed(0)}%`, true));
    }
    cells.push(stdTdHeader(`${tournTotals.getTossupConversionPct().toFixed(0)}%`, true));
    if (this.tournament.scoringRules.hasNegs()) {
      cells.push(stdTdHeader(tournTotals.getNegsPerXTuh().toFixed(1), true));
    }
    if (this.tournament.scoringRules.useBonuses) {
      cells.push(stdTdHeader(tournTotals.getPointsPerBonus().toFixed(2), true));
    }
    if (this.tournament.scoringRules.bonusesBounceBack) {
      cells.push(stdTdHeader(`${tournTotals.getBounceBackConvPct().toFixed(0)}%`, true));
      cells.push(stdTdHeader(`${tournTotals.getTotalBonusConvPct().toFixed(0)}%`, true));
    }
    return tableFooter(cells);
  }

  private pushTossupValueHeaders(cells: string[]) {
    this.tournament.scoringRules.answerTypes.forEach((at) => {
      cells.push(stdTdHeader(at.value.toString() || '??', true));
    });
  }
}

const lineBreak = '<br/>';
const mDashHtml = '&mdash;';
const nbsp = '&nbsp;';
const emsp = '&emsp;';
const ndash = '&ndash;';

const StatReportPageTitles = {
  [StatReportPages.Standings]: 'Standings',
  [StatReportPages.Individuals]: 'Individuals',
  [StatReportPages.Scoreboard]: 'Scoreboard',
  [StatReportPages.TeamDetails]: 'Team Detail',
  [StatReportPages.PlayerDetails]: 'Player Detail',
  [StatReportPages.RoundReport]: 'Round Report',
};

const cssClasses = {
  headerAndDivider: 'headerAndDivider',
  inlineDivider: 'inlineDivider',
  smallText: 'smallText',
  boxScoreTableContainer: 'boxScoreTable',
  pseudoTFoot: 'pseudoTFoot',
  floatingTOC: 'floatingTOC',
};

const topAnchorID = '#top';

/** id HTML attribute */
function id(val: string) {
  return `id=${val}`;
}

function scoreboardRoundLink(round: Round, text: string) {
  const href = `${StatReportFileNames[StatReportPages.Scoreboard]}#${roundLinkId(round)}`;
  return aTag(href, text);
}

function roundLinkId(round: Round) {
  return `Round-${round.number}`;
}

function scoreboardMatchLink(match: Match, text: string) {
  const href = `${StatReportFileNames[StatReportPages.Scoreboard]}#${matchLinkId(match)}`;
  return aTag(href, text);
}

function matchLinkId(match: Match) {
  return `Match-${match.id}`;
}

function teamDetailLink(team: Team) {
  const href = `${StatReportFileNames[StatReportPages.TeamDetails]}#${teamDetailLinkId(team)}`;
  return aTag(href, team.name);
}

function teamDetailLinkId(team: Team) {
  return alphaOnly(team.name);
}

function playerDetailLink(player: Player, team: Team) {
  const href = `${StatReportFileNames[StatReportPages.PlayerDetails]}#${playerDetailLinkId(player, team)}`;
  return aTag(href, player.name);
}

function playerDetailLinkId(player: Player, team: Team) {
  return `${alphaOnly(team.name)}-${alphaOnly(player.name)}`;
}

function alphaOnly(str: string) {
  return str.replace(/\W/g, '');
}

/** Header at the top of the html document */
function getHtmlHeader(pageTitle: string) {
  return genericTag('HEAD', genericTag('title', pageTitle));
}

function getPageStyle() {
  const body = cssSelector('BODY', { attr: 'font-family', val: 'Roboto, sans-serif' });
  const table = cssSelector(
    'table',
    { attr: 'font-size', val: '11pt' },
    { attr: 'border-spacing', val: '0' },
    { attr: 'border-collapse', val: 'collapse' },
  );
  const td = cssSelector('td', { attr: 'padding', val: '5px' });
  const zebra = cssSelector('tr:nth-child(even)', { attr: 'background-color', val: '#f2f2f2' });
  const headerAndDivider = cssSelector(
    `.${cssClasses.headerAndDivider}`,
    { attr: 'display', val: 'flex' },
    { attr: 'flex-direction', val: 'row' },
    { attr: 'margin', val: '18 0' },
  );
  const phaseH2 = cssSelector(`.${cssClasses.headerAndDivider} h2`, { attr: 'margin', val: '0' });
  const inlineDivider = cssSelector(
    `.${cssClasses.inlineDivider}`,
    { attr: 'flex-grow', val: '1' },
    { attr: 'height', val: '1px' },
    { attr: 'background-color', val: '#9f9f9f' },
    { attr: 'align-self', val: 'center' },
  );
  const smallText = cssSelector(`.${cssClasses.smallText}`, { attr: 'font-size', val: '10pt' });
  const ul = cssSelector('ul', { attr: 'margin', val: '0' });
  const boxScoreTableContainer = cssSelector(
    `.${cssClasses.boxScoreTableContainer}`,
    { attr: 'display', val: 'flex' },
    { attr: 'gap', val: '15px' },
    { attr: 'align-items', val: 'flex-start' },
  );
  const pseudoFooter = cssSelector(
    `.${cssClasses.pseudoTFoot}`,
    { attr: 'border-top', val: '1px solid #909090' },
    { attr: 'background-color', val: '#ffffff !important' },
  );
  const floatingTOC = cssSelector(
    `.${cssClasses.floatingTOC}`,
    { attr: 'top', val: '45px' },
    { attr: 'position', val: 'sticky' },
    { attr: 'float', val: 'right' },
    { attr: 'margin-top', val: '5px' },
    { attr: 'margin-right', val: '10px' },
    { attr: 'padding-right', val: '5px' },
    { attr: 'background-color', val: '#cccccc' },
    { attr: 'box-shadow', val: '4px 4px 7px #999999' },
    { attr: 'line-height', val: '1.5' },
  );
  const ulNoBullets = cssSelector(
    `.${cssClasses.floatingTOC} ul`,
    { attr: 'list-style-type', val: 'none' },
    { attr: 'padding-inline-start', val: '20px' },
    { attr: 'font-size', val: '11pt' },
  );
  return genericTag(
    'style',
    body,
    table,
    td,
    zebra,
    headerAndDivider,
    inlineDivider,
    ul,
    smallText,
    phaseH2,
    boxScoreTableContainer,
    pseudoFooter,
    floatingTOC,
    ulNoBullets,
  );
}

/**
 * The links at the top of every page of the report
 */
function getTopLinks(): string {
  const linkCells = [];
  for (const page of StatReportPageOrder) {
    const oneLink = aTag(StatReportFileNames[page], StatReportPageTitles[page]);
    linkCells.push(tdTag({}, oneLink));
  }

  const tr = trTag(linkCells);
  return tableTag([tr], '0', '100%');
}

/** An <a> tag for hyperlinks */
function aTag(href: string, contents: string) {
  return `<a HREF=${href}>${contents}</a>`;
}

function tableTag(trTags: string[], border?: string, width?: string) {
  const borderAttr = border !== undefined ? `border=${border}` : '';
  const widthAttr = width !== undefined ? `width=${width}` : '';
  return `<table ${borderAttr} ${widthAttr}>\n${trTags.join('\n')}\n</table>`;
}

/** A <tr> tag: table row containing <td>s */
function trTag(tdTags: string[]) {
  return `<tr>\n${tdTags.join('\n')}\n</tr>`;
}

/** A tr tag with the special footer class */
function tableFooter(tdTags: string[]) {
  return `<tr class=${cssClasses.pseudoTFoot}>\n${tdTags.join('\n')}\n</tr>`;
}

function stdTdHeader(contents: string, rightAlign?: boolean, width?: string) {
  const props: tdAttributes = { bold: true };
  if (rightAlign) props.align = 'right';
  if (width) props.width = width;
  return tdTag(props, contents);
}

function textCell(contents: string) {
  return tdTag({}, contents);
}

function numericCell(contents: string) {
  return tdTag({ align: 'right' }, contents);
}

type tdAttributes = { align?: string; bold?: boolean; title?: string; style?: string; width?: string };

/** <td> tag (table cell) */
function tdTag(attributes: tdAttributes, contents: string) {
  const align = makeAttributeFromObj(attributes, 'align');
  const title = makeAttributeFromObj(attributes, 'title');
  const style = makeAttributeFromObj(attributes, 'style');
  const width = makeAttributeFromObj(attributes, 'width');
  const innerText = attributes.bold ? genericTag('b', contents) : contents;
  return `<td ${align} ${title} ${style} ${width}>${innerText}</td>`;
}

/** Put contents inside a tag of the given type */
function genericTag(tag: string, ...contents: string[]) {
  return `<${tag}>\n${contents.join('\n')}\n</${tag}>`;
}

function genericTagWithAttributes(tag: string, attr: string[], ...contents: string[]) {
  return `<${tag} ${attr.join(' ')}>\n${contents.join('\n')}\n</${tag}>`;
}

function headerWithDivider(text: string, noTopLink: boolean = false) {
  const header = genericTag('h2', text + nbsp);
  const divider = genericTagWithAttributes('div', [classAttribute(cssClasses.inlineDivider)]);
  if (noTopLink) return genericTagWithAttributes('div', [classAttribute(cssClasses.headerAndDivider)], header, divider);

  const topLink = aTag(
    topAnchorID,
    genericTagWithAttributes('span', [classAttribute(cssClasses.smallText)], `${unicodeHTML('2191')}Top`),
  );
  return genericTagWithAttributes(
    'div',
    [classAttribute(cssClasses.headerAndDivider)],
    header,
    divider,
    genericTag('span', nbsp),
    topLink,
  );
}

function unorderedList(items: string[]) {
  const liTags = items.map((itm) => genericTag('li', itm));
  return genericTag('ul', liTags.join('\n'));
}

function makeAttributeFromObj(obj: any, attrName: string) {
  const val = obj[attrName];
  if (val === undefined) return '';
  return makeAttribute(attrName, val);
}

function makeAttribute(attrName: string, val: string) {
  return `${attrName}="${val}"`;
}

function classAttribute(className: string) {
  return makeAttribute('class', className);
}

function madeWithYellowFruit(yfVersion?: string) {
  let html = `<span style="font-size:x-small">Made with YellowFruit ${unicodeHTML('1F34C')}</span>` + '\n'; // banana emoji
  if (yfVersion) {
    html += `<span style="font-size:x-small; color:white">&nbsp;${yfVersion}</span>` + `\n`;
  }
  return html;
}

type CssRule = { attr: string; val: string };

/** Make a string out of CSS rules for one selector */
function cssSelector(selector: string, ...rules: CssRule[]) {
  const ruleStrings = rules.map((r) => `${r.attr}: ${r.val};`);
  return `${selector}{\n${ruleStrings.join('\n')}\n}`;
}

function unicodeHTML(codepoint: string) {
  return `&#x${codepoint};`;
}
