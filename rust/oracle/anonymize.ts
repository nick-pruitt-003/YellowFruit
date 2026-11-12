/**
 * Make a committable copy of a real tournament file: every team, school, player, pool and free-text value is
 * replaced with a placeholder, and everything that drives parsing and stats is kept as it was (numbers,
 * settings, key order, empty vs non-empty strings, and the leading/trailing spaces and team letters that
 * real files' quirks depend on). Used to build `rust/samples/` for the CI parity job.
 *
 * Usage (from the repo root):
 *   npx tsx rust/oracle/anonymize.ts <in.yft> <out.yft>
 *
 * It exits 1 if any original name of 4+ characters is still in the output; check those by hand.
 */

/* eslint-disable no-console */
import fs from 'fs';

/** Phase names that are generic enough to keep */
const GENERIC_PHASES = new Set(['Prelims', 'Playoffs', 'Finals', 'Tiebreaker', 'Tiebreakers', 'Superplayoffs', 'Championship', 'Consolation']);

/** Report configurations that ship with old YF */
const STANDARD_REPORT_CONFIGS = new Set(['YF Defaults', 'ACF Basic', 'ACF Standard', 'NAQT Standard', 'NAQT Basic']);

class Names {
  private maps = new Map<string, Map<string, string>>();

  /** Originals of every name replaced, for the leak check */
  originals = new Set<string>();

  /** Same original, same placeholder; numbered per kind in order of first appearance */
  get(kind: string, original: string, label: string) {
    let m = this.maps.get(kind);
    if (!m) {
      m = new Map();
      this.maps.set(kind, m);
    }
    let out = m.get(original);
    if (out === undefined) {
      out = `${label} ${m.size + 1}`;
      m.set(original, out);
      this.originals.add(original);
    }
    return out;
  }

  /** The placeholder already given to this original, of any kind */
  lookup(original: string) {
    for (const m of this.maps.values()) {
      const v = m.get(original);
      if (v !== undefined) return v;
    }
    return undefined;
  }
}

const names = new Names();

/** Keep surrounding whitespace, since trimming is one of the quirks under test */
function keepSpaces(raw: string, map: (core: string) => string) {
  const core = raw.trim();
  if (core === '') return raw;
  const lead = raw.slice(0, raw.length - raw.trimStart().length);
  const trail = raw.slice(raw.trimEnd().length);
  return `${lead}${map(core)}${trail}`;
}

const school = (raw: string) => keepSpaces(raw, (c) => names.get('school', c, 'School'));
const player = (raw: string) => keepSpaces(raw, (c) => names.get('player', c, 'Player'));
const pool = (raw: string) => keepSpaces(raw, (c) => names.get('pool', c, 'Pool'));
const phase = (raw: string) => keepSpaces(raw, (c) => (GENERIC_PHASES.has(c) ? c : names.get('phase', c, 'Phase')));
const round = (raw: string) => keepSpaces(raw, (c) => (/^\d+$/.test(c) ? c : names.get('round', c, 'Round')));
const text = (raw: string) => (raw === '' ? raw : 'redacted');

/** A team name keeps its letter ("X A" and "X B" stay one school), as teamGetNameAndLetter splits it */
function team(raw: string) {
  return keepSpaces(raw, (core) => {
    if (core.length >= 3 && core.charAt(core.length - 2) === ' ' && /^[A-Za-z]$/.test(core.charAt(core.length - 1))) {
      names.originals.add(core);
      return `${school(core.slice(0, -2).trim())} ${core.charAt(core.length - 1)}`;
    }
    return school(core);
  });
}

/** Names inside free text such as validation messages */
function replaceNamesIn(s: string) {
  let out = s;
  const all = [...names.originals].filter((n) => n.length >= 2).sort((a, b) => b.length - a.length);
  for (const n of all) {
    if (out.includes(n)) out = out.split(n).join(names.lookup(n) ?? team(n));
  }
  return out;
}

/* ---------------------------------- old YF 2.x/3.x ---------------------------------- */

function anonymizeLegacy(contents: string) {
  const lines = contents.split('\n');
  const sectionIdx = lines.map((l, i) => (l === 'divider_between_sections' ? -1 : i)).filter((i) => i >= 0 && lines[i] !== '');
  const kinds =
    sectionIdx.length >= 6
      ? ['meta', 'packets', 'settings', 'phases', 'teams', 'games']
      : ['settings', 'phases', 'teams', 'games'];
  const sections: Record<string, unknown> = {};
  kinds.forEach((k, j) => {
    sections[k] = JSON.parse(lines[sectionIdx[j]]);
  });

  // teams and players first, so free text can refer to them
  const teams = sections.teams as Record<string, unknown>[];
  for (const t of teams) {
    t.teamName = team(t.teamName as string);
    t.roster = renameKeys(t.roster as Record<string, unknown>, player);
    t.divisions = mapEntries(t.divisions as Record<string, string>, phase, (d) => (typeof d === 'string' ? pool(d) : d));
  }
  const settings = sections.settings as Record<string, unknown>;
  if (Array.isArray(settings.defaultPhases)) settings.defaultPhases = settings.defaultPhases.map((p: string) => phase(p));
  // custom report configurations are named by users (e.g. after a school)
  if (typeof settings.rptConfig === 'string' && !STANDARD_REPORT_CONFIGS.has(settings.rptConfig)) settings.rptConfig = 'Custom';
  if (typeof settings.defaultPhase === 'string' && settings.defaultPhase !== 'noPhase') settings.defaultPhase = phase(settings.defaultPhase);
  sections.phases = mapEntries(sections.phases as Record<string, string[]>, phase, (divs) =>
    Array.isArray(divs) ? divs.map((d) => (typeof d === 'string' ? pool(d) : d)) : divs,
  );
  if (sections.packets) {
    sections.packets = mapEntries(sections.packets as Record<string, string>, (k) => k, (v) => (typeof v === 'string' ? text(v) : v));
  }
  for (const g of sections.games as Record<string, unknown>[]) {
    if (typeof g.team1 === 'string') g.team1 = team(g.team1);
    if (typeof g.team2 === 'string') g.team2 = team(g.team2);
    if (Array.isArray(g.phases)) g.phases = g.phases.map((p: unknown) => (typeof p === 'string' ? phase(p) : p));
    if (g.players1 && typeof g.players1 === 'object') g.players1 = renameKeys(g.players1 as Record<string, unknown>, player);
    if (g.players2 && typeof g.players2 === 'object') g.players2 = renameKeys(g.players2 as Record<string, unknown>, player);
    if (typeof g.notes === 'string') g.notes = text(g.notes);
    if (typeof g.validationMsg === 'string') g.validationMsg = replaceNamesIn(g.validationMsg);
  }

  kinds.forEach((k, j) => {
    lines[sectionIdx[j]] = JSON.stringify(sections[k]);
  });
  return lines.join('\n');
}

function renameKeys(o: Record<string, unknown>, f: (k: string) => string) {
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(o ?? {})) out[f(k)] = v;
  return out;
}

function mapEntries<V>(o: Record<string, V>, fk: (k: string) => string, fv: (v: V) => V) {
  const out: Record<string, V> = {};
  for (const [k, v] of Object.entries(o ?? {})) out[fk(k)] = fv(v);
  return out;
}

/* ---------------------------------- YF4 .yft ---------------------------------- */

/** Which kind of object an array or child key holds */
const CHILD_CONTEXT: Record<string, string> = {
  registrations: 'registration',
  teams: 'team',
  players: 'player',
  phases: 'phase',
  pools: 'pool',
  rounds: 'round',
  matches: 'match',
  packets: 'packet',
  tournament_site: 'site',
  tournamentSite: 'site',
  rankings: 'ranking',
  scoring_rules: 'rules',
  scoringRules: 'rules',
  answer_types: 'answerType',
  answerTypes: 'answerType',
};

const TYPE_CONTEXT: Record<string, string> = {
  Tournament: 'tournament',
  ScoringRules: 'rules',
  Registration: 'registration',
  Team: 'team',
  Player: 'player',
  Phase: 'phase',
  Pool: 'pool',
  Round: 'round',
  Match: 'match',
  TournamentSite: 'site',
  Ranking: 'ranking',
  AnswerType: 'answerType',
};

const FREE_TEXT = new Set(['notes', 'location', 'moderator', 'scorekeeper', 'serial', 'importedFile', 'description', 'question_set', 'questionSet']);

function anonymizeId(id: string): string {
  let m;
  if ((m = /^Team_(.*)$/s.exec(id))) return `Team_${team(m[1])}`;
  if ((m = /^PoolTeam_(.*)$/s.exec(id))) return `PoolTeam_${team(m[1])}`;
  if ((m = /^Registration_(.*)$/s.exec(id))) return `Registration_${school(m[1])}`;
  if ((m = /^Player_(.*)_(\d+)$/s.exec(id))) return `Player_${player(m[1])}_${m[2]}`;
  if ((m = /^Pool_(.*)$/s.exec(id))) return `Pool_${pool(m[1])}`;
  if ((m = /^Phase_(.*)$/s.exec(id))) return `Phase_${phase(m[1])}`;
  if ((m = /^Round_(.*)$/s.exec(id))) return `Round_${round(m[1])}`;
  // the loader only reads the number; the rest is team-name abbreviations
  if ((m = /^Match_(\d+)~.*$/s.exec(id))) return `Match_${m[1]}`;
  if (/^TournamentSite_/.test(id)) return 'TournamentSite_Sample Site';
  return id;
}

function nameFor(ctx: string, raw: string) {
  switch (ctx) {
    case 'tournament':
      return 'Sample Tournament';
    case 'site':
      return 'Sample Site';
    case 'registration':
      return school(raw);
    case 'team':
      return team(raw);
    case 'player':
      return player(raw);
    case 'pool':
      return pool(raw);
    case 'phase':
      return phase(raw);
    case 'round':
      return round(raw);
    case 'packet':
      return text(raw);
    case 'rules':
      return raw === '' ? raw : 'Sample Rules';
    default:
      return raw;
  }
}

function anonymizeYftValue(v: unknown, ctx: string, key: string): unknown {
  if (Array.isArray(v)) return v.map((x) => anonymizeYftValue(x, CHILD_CONTEXT[key] ?? ctx, ''));
  if (v && typeof v === 'object') {
    // top-level objects in a qbj file say what they are
    const type = (v as Record<string, unknown>).type;
    if (typeof type === 'string' && TYPE_CONTEXT[type]) ctx = TYPE_CONTEXT[type];
    const out: Record<string, unknown> = {};
    for (const [k, x] of Object.entries(v)) {
      const childCtx = Array.isArray(x) ? ctx : (CHILD_CONTEXT[k] ?? ctx);
      out[k] = anonymizeYftValue(x, k === 'YfData' ? ctx : childCtx, k);
    }
    return out;
  }
  if (typeof v !== 'string') return v;
  if (key === 'name') return nameFor(ctx, v);
  if (key === 'id' || key === '$ref') return anonymizeId(v);
  if (key === 'nonNumericName') return round(v);
  if (key === 'packets' && ctx === 'match') return text(v);
  if (FREE_TEXT.has(key)) return text(v);
  if (key === 'message') return v;
  return v;
}

/** Names first (so validation messages can be rewritten with them), then the rest */
function anonymizeYft(contents: string) {
  const obj = JSON.parse(contents);
  const out = anonymizeYftValue(obj, 'tournament', '') as Record<string, unknown>;
  (function fixMessages(x: unknown) {
    if (Array.isArray(x)) x.forEach(fixMessages);
    else if (x && typeof x === 'object') {
      for (const [k, y] of Object.entries(x)) {
        if (k === 'message' && typeof y === 'string') (x as Record<string, unknown>)[k] = replaceNamesIn(y);
        else fixMessages(y);
      }
    }
  })(out);
  for (const o of (out.objects as Record<string, unknown>[]) ?? []) {
    if (o.type === 'Tournament' || o.registrations) o.name = 'Sample Tournament';
  }
  return JSON.stringify(out);
}

/* ---------------------------------- main ---------------------------------- */

function isOldYfFile(contents: string) {
  return contents.search(/^\{"version":"\d+\.\d+\.\d+"\}/) === 0 || contents.search(/^\{.*"powers":.*\}/) === 0;
}

function main() {
  const [inFile, outFile] = process.argv.slice(2);
  if (!inFile || !outFile) {
    console.error('usage: anonymize.ts <in.yft> <out.yft>');
    process.exit(2);
  }
  const contents = fs.readFileSync(inFile, 'utf8');
  const out = isOldYfFile(contents) ? anonymizeLegacy(contents) : anonymizeYft(contents);

  // generic names (a pool called "Finals" or "Pool 1") can't identify anyone
  const isGeneric = (n: string) => GENERIC_PHASES.has(n) || /^(Pool|Division|Bracket|Group) [A-Z0-9]{1,2}$/i.test(n);
  const leaks = [...names.originals].filter(
    (n) => n.length >= 4 && !isGeneric(n) && new RegExp(`\\b${escapeRe(n)}\\b`).test(out),
  );
  fs.writeFileSync(outFile, out);
  if (leaks.length > 0) {
    // run locally on real files only, so printing the names is fine
    console.error(`${outFile}: original names still present: ${JSON.stringify(leaks)}`);
    process.exit(1);
  }
}

function escapeRe(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

main();
