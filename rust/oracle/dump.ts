/**
 * TS parity oracle. Loads each tournament file with the real YellowFruit DataModel (no Electron, no React)
 * and writes a JSON dump of what the app would compute: parsed structure, per-match validation, and stats.
 * The Rust `yf-parity` tool produces the same dump shape, and `yf-parity diff` compares the two.
 *
 * Usage (from the repo root):
 *   npx tsx rust/oracle/dump.ts <out-dir> <file.yft> [<file.yft> ...]
 *   npx tsx rust/oracle/dump.ts <out-dir> --list <nul-separated-file-list>
 *
 * Output: <out-dir>/<sanitized-name>.ts.json per input file, plus <out-dir>/manifest.ts.json.
 * Dumps contain real team and player names, so keep <out-dir> out of git (rust/target/ is ignored).
 */

/* eslint-disable no-console */
import fs from 'fs';
import path from 'path';
import dayjs from 'dayjs';

// DataModel code only touches `window` in a few UI helpers, but stub it so nothing explodes at call time.
(globalThis as unknown as { window: unknown }).window = {
  electron: { ipcRenderer: { sendMessage() {}, on() {}, once() {}, invoke() {} }, getPlatform: () => 'darwin' },
};

const DM = '../../src/renderer/DataModel';

async function main() {
  const [outDir, ...rest] = process.argv.slice(2);
  // `--list <file>` reads NUL-separated paths (what `find -print0` writes), so the run is one process
  const files =
    rest[0] === '--list' ? fs.readFileSync(rest[1], 'utf8').split('\0').filter((f) => f.length > 0) : rest;
  if (!outDir || files.length === 0) {
    console.error('usage: tsx rust/oracle/dump.ts <out-dir> <file> [...]');
    process.exit(2);
  }
  fs.mkdirSync(outDir, { recursive: true });

  const { parseOldYfFile, isOldYftFile } = await import(`${DM}/OldYfParsing`);
  const { earlyYftFileConversions, snakeCaseToCamelCase } = await import(`${DM}/CaseConversion`);
  const { collectRefTargets, findTournamentObject } = await import(`${DM}/QbjUtils2`);
  const { qbjFileValidVersion } = await import(`${DM}/QbjUtils`);
  const { default: FileParser } = await import(`${DM}/FileParsing`);
  const { Match } = await import(`${DM}/Match`);
  const { Player } = await import(`${DM}/Player`);

  const curYfVersion = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../../release/app/package.json'), 'utf8'))
    .version as string;

  const manifest: { file: string; out: string; format: string; error?: string }[] = [];
  const usedNames = new Set<string>();

  for (const file of files) {
    // Reset global ID counters so each file is processed as if the app had just launched.
    Match.overrideIdCounter(1000);
    (Player as unknown as { idCounter: number }).idCounter = 1000;

    const outName = uniqueName(path.basename(file).replace(/\.[^.]+$/, ''), usedNames);
    const entry: (typeof manifest)[number] = { file, out: `${outName}.ts.json`, format: '' };
    let dump: Record<string, unknown> = {};
    try {
      const contents = fs.readFileSync(file, 'utf8');
      let tourn;
      if (isOldYftFile(contents)) {
        entry.format = 'legacy';
        tourn = parseOldYfFile(contents);
      } else {
        entry.format = 'yft';
        const obj = JSON.parse(contents, (key, value) => {
          if (isNameOfDateField(key)) return dayjs(value).toDate();
          return value;
        });
        earlyYftFileConversions(obj);
        snakeCaseToCamelCase(obj);
        if (!qbjFileValidVersion(obj)) throw new Error('unsupported qbj version');
        const tournObj = findTournamentObject(obj.objects);
        if (!tournObj) throw new Error('no Tournament object');
        const parser = new FileParser(collectRefTargets(obj.objects));
        tourn = parser.parseYftTournament(tournObj, curYfVersion);
        if (!tourn) throw new Error('parse returned null');
        tourn.conversions();
        dump.parserWarnings = parser.warnings;
      }
      tourn.appVersion = curYfVersion;
      dump = { ...dump, ...dumpTournament(tourn) };
    } catch (err) {
      entry.error = (err as Error).message;
      dump.error = entry.error;
    }
    dump.format = entry.format;
    fs.writeFileSync(path.join(outDir, entry.out), `${JSON.stringify(dump, null, 1)}\n`);
    manifest.push(entry);
  }
  fs.writeFileSync(path.join(outDir, 'manifest.ts.json'), `${JSON.stringify(manifest, null, 1)}\n`);
  const errs = manifest.filter((m) => m.error).length;
  console.log(`wrote ${manifest.length} dumps to ${outDir} (${errs} failed to load)`);
}

function isNameOfDateField(key: string) {
  return key === 'startDate' || key === 'endDate' || key === 'start_date' || key === 'end_date' || key === 'savedAtTime';
}

function uniqueName(base: string, used: Set<string>) {
  const clean = base.replace(/[^A-Za-z0-9_.-]+/g, '_');
  let name = clean;
  for (let i = 2; used.has(name); i++) name = `${clean}__${i}`;
  used.add(name);
  return name;
}

/* ---------- dump builders. Keep field names in sync with rust/yf-parity/src/dump.rs ---------- */

/** Numbers that may be NaN/Infinity, which JSON can't hold */
function num(x: number | undefined | null) {
  if (x === undefined || x === null) return null;
  if (Number.isNaN(x)) return 'NaN';
  if (x === Infinity) return 'Inf';
  if (x === -Infinity) return '-Inf';
  return x;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Any = any;

function dumpTournament(t: Any) {
  const regTu = t.scoringRules.regulationTossupCount;
  const structure = {
    name: t.name,
    rules: {
      answerTypes: t.scoringRules.answerTypes.map((a: Any) => a.value),
      timed: t.scoringRules.timed,
      maxRegTu: t.scoringRules.maximumRegulationTossupCount,
      maxPlayers: t.scoringRules.maximumPlayersPerTeam,
      useBonuses: t.scoringRules.useBonuses,
      bounceBack: t.scoringRules.bonusesBounceBack,
      lightning: t.scoringRules.lightningCountPerTeam,
    },
    registrations: t.registrations.map((r: Any) => ({
      name: r.name,
      teams: r.teams.map((tm: Any) => ({ name: tm.name, letter: tm.letter, players: tm.players.map((p: Any) => p.name) })),
    })),
    seeds: t.seeds.map((tm: Any) => tm.name),
    phases: t.phases.map((ph: Any) => ({
      name: ph.name,
      type: ph.phaseType,
      code: ph.code,
      pools: ph.pools.map((p: Any) => ({
        name: p.name,
        position: p.position,
        size: p.size,
        teams: p.poolTeams.map((pt: Any) => pt.team.name),
      })),
      rounds: ph.rounds.map((rd: Any) => ({
        number: rd.number,
        name: rd.name,
        packet: rd.packet.name,
        matches: rd.matches.map((m: Any) => ({
          left: m.leftTeam.team?.name ?? null,
          right: m.rightTeam.team?.name ?? null,
          score: [num(m.leftTeam.points), num(m.rightTeam.points)],
          tuh: num(m.tossupsRead),
          ot: num(m.overtimeTossupsRead),
          carryover: m.carryoverPhases.map((cp: Any) => cp.name),
          omit: m.statsValidity === 1,
          errors: m.getErrorMessages(),
          warnings: m.getWarningMessages(),
        })),
      })),
    })),
  };

  // In-app standings view (TournamentManager.compileStats)
  t.compileStats(false, true);
  const inApp = {
    phases: t.stats.map((ps: Any) => dumpPhaseStandings(ps, regTu)),
    prelimsPlusPlayoffs: t.prelimsPlusPlayoffStats ? dumpPhaseStandings(t.prelimsPlusPlayoffStats, regTu) : null,
  };

  // Full stat report (TournamentManager.generateStatReports -> compileStats(true))
  t.prelimsPlusPlayoffStats = undefined;
  t.compileStats(true);
  const report = {
    phases: t.stats.map((ps: Any) => dumpPhaseStandings(ps, regTu, true)),
    prelimsPlusPlayoffs: t.prelimsPlusPlayoffStats ? dumpPhaseStandings(t.prelimsPlusPlayoffStats, regTu) : null,
    cumulative: t.cumulativeStats ? dumpAggregate(t.cumulativeStats, regTu) : null,
  };

  return { structure, inApp, report };
}

function dumpAnswerCounts(acs: Any[]) {
  return acs.map((ac: Any) => [ac.answerType.value, num(ac.number)]);
}

function dumpPoolTeamStats(s: Any, regTu: number) {
  return {
    team: s.team.name,
    rank: s.rank,
    rawRank: s.rawRank,
    finalRankCalculated: num(s.finalRankCalculated),
    currentSeed: num(s.currentSeed),
    advanceToTier: num(s.advanceToTier),
    recordTie: s.recordTieForAdvancement,
    ppgTie: s.ppgTieForAdvancement,
    record: s.getRecord(),
    winPct: s.getWinPctString(),
    wlt: [s.wins, s.losses, s.ties],
    nonForfeit: s.nonForfeitMatches,
    tuh: [s.tuhTotal, s.tuhRegulation],
    points: [s.totalPoints, s.totalPointsForPPG],
    ppg: s.getPtsPerRegTuhString(regTu),
    pptuhRaw: num(s.getPtsPerRegTuh()),
    bonus: [num(s.bonusesHeard), num(s.bonusPoints)],
    ppb: s.getPtsPerBonusString(),
    ppbRaw: num(s.getPtsPerBonus()),
    bb: [num(s.bounceBackPoints), num(s.bounceBackPartsHeard), num(s.bounceBackPartsConverted)],
    bbPct: s.getBouncebackConvPctString(),
    lightning: [num(s.lightningPoints), s.getLightningPtsPerMatchString()],
    counts: dumpAnswerCounts(s.tossupCounts),
    matches: s.matches.length,
  };
}

function dumpPlayerStats(p: Any) {
  return {
    player: p.player.name,
    team: p.team.name,
    rank: p.rank,
    rankTie: p.rankTie,
    gp: p.gamesPlayed.toFixed(1),
    gpRaw: num(p.gamesPlayed),
    tuh: p.tossupsHeard,
    points: p.getTotalPoints(),
    pptuh: num(p.getPptuh()),
    counts: dumpAnswerCounts(p.tossupCounts),
    matches: p.matches.length,
  };
}

function dumpPhaseStandings(ps: Any, regTu: number, withPlayers = false) {
  return {
    phase: ps.phase.name,
    anyTies: ps.anyTiesExist,
    pools: ps.pools.map((pool: Any) => ({
      pool: pool.pool.name,
      teams: pool.poolTeams.map((s: Any) => dumpPoolTeamStats(s, regTu)),
    })),
    players: withPlayers ? ps.players.map(dumpPlayerStats) : null,
  };
}

function dumpRoundStats(r: Any) {
  return {
    round: r.round.number,
    phase: r.phase?.name ?? null,
    games: r.games,
    tuh: [r.tossupsHeardTotal, r.tossupsHeardRegulation],
    points: r.points,
    bonusPoints: r.bonusPoints,
    lightning: r.lightningPoints,
    bbConv: num(r.bouncebackPartsConverted),
    counts: dumpAnswerCounts(r.tossupCounts),
    derived: [
      r.getPointsPerXTuh(),
      r.getPowerPct(),
      r.getTossupConversionPct(),
      r.getNegsPerXTuh(),
      r.getPointsPerBonus(),
      r.getBounceBackConvPct(),
      r.getTotalBonusConvPct(),
      r.getLightningPointsPerTeamPerMatch(),
    ].map(num),
  };
}

function dumpAggregate(agg: Any, regTu: number) {
  return {
    anyTies: agg.anyTiesExist,
    teams: agg.teamStats.map((s: Any) => dumpPoolTeamStats(s, regTu)),
    players: agg.players.map(dumpPlayerStats),
    rounds: agg.rounds.map(dumpRoundStats),
    total: dumpRoundStats(agg.roundReportTotalStats),
  };
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
