import dayjs from 'dayjs';
import { expect, test } from 'vitest';
import { camelCaseToSnakeCase, earlyYftFileConversions, snakeCaseToCamelCase } from '../renderer/DataModel/CaseConversion';
import FileParser from '../renderer/DataModel/FileParsing';
import { parseOldYfFile } from '../renderer/DataModel/OldYfParsing';
import { collectRefTargets, findTournamentObject } from '../renderer/DataModel/QbjUtils2';
import Tournament, { IYftFileTournament } from '../renderer/DataModel/Tournament';

// Importing an old (YF 2.x/3.x) file, saving it, and reopening the saved file must not lose or change anything.

function isDateField(key: string) {
  return key === 'startDate' || key === 'endDate' || key === 'start_date' || key === 'end_date' || key === 'savedAtTime';
}

/** What File > Save writes (TournamentManager.generateWholeFileObj + makeJSON) */
function save(tourn: Tournament) {
  tourn.appVersion = '4.0.0';
  const whole = { version: '2.1.1', objects: [tourn.toFileObject(false, true)] };
  camelCaseToSnakeCase(whole as unknown as Record<string, unknown>);
  return JSON.stringify(whole, (key, value) => {
    if (isDateField(key)) return value ? dayjs(value).toISOString() : undefined;
    return value;
  });
}

/** What File > Open does with a .yft file (TournamentManager.openYftFile) */
function open(contents: string) {
  const obj = JSON.parse(contents, (key, value) => (isDateField(key) ? dayjs(value).toDate() : value));
  earlyYftFileConversions(obj);
  snakeCaseToCamelCase(obj);
  const parser = new FileParser(collectRefTargets(obj.objects));
  const tourn = parser.parseYftTournament(findTournamentObject(obj.objects) as IYftFileTournament, '4.0.0');
  return { tourn: tourn!, warnings: parser.warnings };
}

const player = { year: '', undergrad: false, div2: false };
const line = (tuh: number, tens: number) => ({ tuh, powers: 0, tens, negs: 0 });
const game = (team1: string, team2: string, extra: object) => ({
  round: 1,
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

/** A small old-format file with the quirks real files have: trailing spaces in names, a player with no name, a forfeit */
function oldYfFile() {
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
  const teams = [
    team('Alpha ', { 'Ann ': player, Bob: player, '': player }),
    team('Beta', { Cal: player, Dee: player }),
    team('Gamma', { Eve: player }),
  ];
  const games = [
    game('Alpha ', 'Beta', { players1: { 'Ann ': line(20, 5), Bob: line(20, 3) }, players2: { Cal: line(20, 4) } }),
    game('Gamma', 'Beta', { round: 2, forfeit: true, score1: 0, score2: 0 }),
  ];
  const settings = { powers: 'none', negs: true, bonuses: true, bonusesBounce: false, lightning: false, playersPerTeam: 4, defaultPhases: [], rptConfig: 'ACF Basic' };
  return [{ version: '3.0.0' }, {}, settings, { Prelims: ['Pool 1'] }, teams, games].map((x) => JSON.stringify(x)).join('\n');
}

function summary(tourn: Tournament) {
  return {
    teams: tourn.getListOfAllTeams().map((tm) => [tm.name, tm.players.map((p) => p.name)]),
    seeds: tourn.seeds.map((tm) => tm.name),
    matches: tourn.phases.flatMap((ph) => ph.getAllMatches()).map((m) => [m.leftTeam.team?.name, m.rightTeam.team?.name, m.tossupsRead]),
  };
}

test('oldYfImportTrimsNamesAndDropsBlankPlayers', () => {
  const t = summary(parseOldYfFile(oldYfFile()));
  expect(t.teams).toEqual([
    ['Alpha', ['Ann', 'Bob']],
    ['Beta', ['Cal', 'Dee']],
    ['Gamma', ['Eve']],
  ]);
});

test('oldYfImportForfeitHasNoTossupsRead', () => {
  const t = summary(parseOldYfFile(oldYfFile()));
  expect(t.matches[1]).toEqual(['Gamma', 'Beta', undefined]);
});

test('oldYfImportSurvivesSaveAndReopen', () => {
  const imported = parseOldYfFile(oldYfFile());
  const { tourn: reopened, warnings } = open(save(imported));
  expect(summary(reopened)).toEqual(summary(imported));
  expect(warnings).toEqual([]);
  // Player IDs carry a counter that restarts on reopen; everything else must be the same
  const noPlayerIds = (json: string) => json.replace(/(Player_[^"]*_)\d+/g, '$1N');
  expect(noPlayerIds(save(reopened))).toBe(noPlayerIds(save(imported)));
});

/** A file saved by an earlier version of the app after importing an old file with the problems above */
function savedByOldVersion() {
  const imported = parseOldYfFile(oldYfFile());
  const contents = JSON.parse(save(imported));
  const tournObj = contents.objects[0];
  const alpha = tournObj.registrations.find((r: { name: string }) => r.name === 'Alpha').teams[0];
  // untrimmed team name in its ID, and a player with no name
  alpha.id = 'Team_Alpha ';
  alpha.players.push({ id: 'Player__9999', name: '', year: -1 });
  tournObj.YfData.seeds = [{ $ref: 'Team_Alpha ' }, { $ref: 'Team_Beta' }, { $ref: 'Team_Gamma' }];
  return JSON.stringify(contents).replaceAll('"$ref":"Team_Alpha"', '"$ref":"Team_Alpha "');
}

test('reopenFileWithUntrimmedTeamIdAndBlankPlayer', () => {
  const { tourn, warnings } = open(savedByOldVersion());
  expect(summary(tourn).seeds).toEqual(['Alpha', 'Beta', 'Gamma']);
  expect(summary(tourn).teams[0]).toEqual(['Alpha', ['Ann', 'Bob']]);
  expect(warnings).toEqual(['Team Alpha contains a player with no name, who was skipped.']);
});

/** Old YF didn't require stages: games could be in none, and files could define none */
function oldYfFileWithStages(phases: object, gamePhases: string[][]) {
  const lines = oldYfFile().split('\n');
  lines[3] = JSON.stringify(phases);
  const games = JSON.parse(lines[5]);
  const extra = games.map((g: object, i: number) => ({ ...g, round: i + 3, phases: gamePhases[i] ?? [] }));
  lines[5] = JSON.stringify([...games, ...extra]);
  return lines.join('\n');
}

const matchCount = (t: Tournament) => t.phases.reduce((n, ph) => n + ph.getAllMatches().length, 0);

test('oldYfFileWithNoStagesKeepsItsGames', () => {
  const t = parseOldYfFile(oldYfFileWithStages({}, []));
  expect(t.phases.map((ph) => ph.name)).toEqual(['Prelims']);
  expect(matchCount(t)).toBe(4);
});

test('oldYfUntaggedRoundsJoinTheStageBefore', () => {
  // rounds 1-2 are tagged Prelims; rounds 3-4 aren't in any stage
  const t = parseOldYfFile(oldYfFileWithStages({ Prelims: ['Pool 1'] }, []));
  expect(t.phases.length).toBe(1);
  expect(t.phases[0].rounds.map((r) => r.number)).toEqual([1, 2, 3, 4]);
  expect(matchCount(t)).toBe(4);
});
