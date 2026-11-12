import { expect, test } from 'vitest';
import { parseOldYfFile } from '../renderer/DataModel/OldYfParsing';
import Tournament from '../renderer/DataModel/Tournament';

// Stat report edge cases that real old files can produce

(globalThis as unknown as { window: unknown }).window = { electron: { ipcRenderer: { sendMessage() {} } } };

const player = (year = '') => ({ year, undergrad: false, div2: false });
const line = (tuh: number, tens: number) => ({ tuh, powers: 0, tens, negs: 0 });
const team = (teamName: string, roster: object) => ({
  teamName,
  teamUGStatus: false,
  teamD2Status: false,
  smallSchool: false,
  jrVarsity: false,
  rank: 0,
  roster,
  divisions: { Prelims: 'Pool 1' },
});
const game = (round: number, team1: string, team2: string, extra: object) => ({
  round,
  phases: ['Prelims'],
  tuhtot: 20,
  ottu: 0,
  team1,
  team2,
  score1: 100,
  score2: 50,
  otPwr1: 0,
  otPwr2: 0,
  otTen1: 0,
  otTen2: 0,
  otNeg1: 0,
  otNeg2: 0,
  bbPts1: 0,
  bbPts2: 0,
  tiebreaker: false,
  lightningPts1: 0,
  lightningPts2: 0,
  players1: {},
  players2: {},
  ...extra,
});

/** Two teams whose names differ only in punctuation, a player who never played, a "-1" year and a null score */
function tournament(): Tournament {
  const teams = [team("St. Mary's", { Ann: player('-1'), Bob: player() }), team('St Marys', { Cal: player(), Dee: player() })];
  const games = [
    game(1, "St. Mary's", 'St Marys', { players1: { Ann: line(20, 10) }, players2: { Cal: line(20, 5) } }),
    // null score in a game that isn't a forfeit (old YF wrote these)
    game(2, 'St Marys', "St. Mary's", { score1: null, score2: 0, players1: { Cal: line(20, 0) }, players2: { Ann: line(20, 0) } }),
  ];
  const settings = { powers: 'none', negs: true, bonuses: true, bonusesBounce: false, lightning: false, playersPerTeam: 4, defaultPhases: [], rptConfig: 'YF Defaults' };
  const file = [{ version: '3.0.0' }, {}, settings, { Prelims: ['Pool 1'] }, teams, games].map((x) => JSON.stringify(x)).join('\n');
  const t = parseOldYfFile(file);
  t.compileStats(true);
  return t;
}

test('reportSurvivesNullScore', async () => {
  const t = tournament();
  await expect(t.makeHtmlTeamDetail()).resolves.toContain('0 - 0');
  await expect(t.makeHtmlPlayerDetail()).resolves.toBeTypeOf('string');
  const scoreboard = await t.makeHtmlScoreboard();
  expect(scoreboard).not.toContain('null');
});

test('teamsWithSimilarNamesGetTheirOwnAnchors', async () => {
  const detail = await tournament().makeHtmlTeamDetail();
  expect(detail).toContain('<h2 id=StMarys>');
  expect(detail).toContain('<h2 id=StMarys-2>');
});

test('playerDetailSkipsPlayersWhoNeverPlayed', async () => {
  const detail = await tournament().makeHtmlPlayerDetail();
  expect(detail).toContain('Ann, ');
  expect(detail).not.toContain('Bob, ');
});

test('playerDetailYearWithoutTitle', async () => {
  const t = tournament();
  t.trackPlayerYear = true;
  const detail = await t.makeHtmlPlayerDetail();
  expect(detail).not.toContain('undefined');
  expect(detail).toContain('Year/Grade: -1');
});

test('playerDetailGamesPlayedWithoutTossupsRead', async () => {
  const t = tournament();
  const m = t.phases[0].rounds[0].matches[0];
  m.tossupsRead = undefined;
  m.validateAll(t.scoringRules);
  m.determineStatsValidity();
  t.compileStats(true);
  const detail = await t.makeHtmlPlayerDetail();
  expect(detail).not.toContain('Infinity');
  expect(detail).not.toContain('NaN');
});
