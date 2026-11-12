# yf-core: Rust port of the YellowFruit core (milestone 1, in progress)

A Rust port of the data model, file loading and stats engine from `src/renderer/DataModel/`. Each piece gets parity-tested against the TypeScript app on real tournament files before anything else is built on it. See `../RUST_PORT_ASSESSMENT.md` for why this comes first.

## Run the parity check

```bash
rust/parity.sh                 # every .yft under ~/Downloads/_QuizBowl
rust/parity.sh rust/samples    # the anonymized samples CI uses
rust/parity.sh /some/folder    # or any folder
```

It needs `node_modules` (`npm ci --ignore-scripts` is enough) and cargo, and takes about 15 seconds. Steps:

1. `rust/oracle/dump.ts` loads each file with the real TS DataModel (headless, no Electron) and writes `<name>.ts.json`.
2. `yf-parity dump` loads the same files with `yf-core` and writes `<name>.rs.json` in the same shape.
3. `yf-parity diff` compares them field by field. Numbers compare as f64, strings exactly. It exits 1 on any difference (`diff-all` prints every difference, not just the first few).

The dumps hold real team and player names, so they live in `rust/target/parity/`, which git ignores.

## CI and the sample files

`.github/workflows/parity.yml` runs the Rust unit tests and `rust/parity.sh rust/samples` on pushes and PRs that touch `rust/`, `src/renderer/DataModel/` or `src/renderer/Utils/`. It runs with `TZ=America/New_York` so the date-only sample goes through the local-time re-save path.

The real files can't be committed, so `rust/samples/` holds 11 anonymized copies made with `rust/oracle/anonymize.ts`:

```bash
npx tsx rust/oracle/anonymize.ts <real.yft> rust/samples/<name>.yft
```

It replaces every school, team, player, pool, non-generic phase and round name, plus notes, packet names, locations, the tournament and site names, custom report-config names and scoring-rule names. It keeps everything that affects parsing and stats: numbers, settings, key order, team letters, and leading and trailing spaces. It exits 1 if a known original name of 4+ characters survives. On all 122 loadable real files, the anonymized copy gives the same stat numbers and the same validation results as the original, and both apps still agree on it.

Each sample covers a real-file quirk: forfeits with tossups read, overtime and a blank player name (`legacy-forfeits-overtime-blank-player`), a seeded team with no players, stray spaces in team and player names, `null` scores, 20-point powers, a PPB that rounds on an exact tie, and YF4 files with overtime or a date-only start date. A mutation check (8 deliberate bugs in yf-core, one at a time: tie rounding, forfeit TUH, trimming, blank players, skipped seeds, `null` scores, key order, dates) fails the samples run every time.

Not covered, because no real file has them: old files from before YF 2.5, and old files with bouncebacks or lightning rounds. Before adding a sample, look through it for anything identifying that the leak check can't know about (e.g. `grep -oE '"[A-Z][^"]+"' rust/samples/<name>.yft | sort -u`).

## What a dump covers

- **Structure:** scoring rules, registrations, teams, rosters, seeds, phases, pools, rounds, packets, and every match: score, TUH, OT, carryover phases, whether it's omitted from stats, and the exact error and warning text from match validation.
- **In-app standings:** `compileStats(false, true)`, i.e. per-phase pool standings with final-rank sorting.
- **Stat report:** `compileStats(true)`, i.e. per-phase standings and individuals, prelims+playoffs, cumulative team, player and round-report stats. It includes the display strings (`ppg`, `ppb`, `winPct`, `gp`, bounceback %), not just raw numbers.
- **Saved file:** what File > Save writes (`saved`), compared byte for byte. Then the saved file is reopened and everything above is dumped again (`reloaded`), and `resaveIdentical` records whether saving the reopened file gives the same bytes. Each side reopens its own saved file.

## Status (2026-11-12)

`139 files: 122 identical, 17 fail to load in both, 0 differ`

| | Files | Result |
|---|---|---|
| Legacy YF 2.x/3.x | 116 load | all identical |
| YF4 `.yft` | 6 load | all identical |
| Fail in both apps | 17 | same failure: 9 old files with unescaped quotes in names (invalid JSON written by old YF), 6 that aren't YF4 files (MODAQ/qbj), 2 LIQBA seed files with inconsistent bonus settings plus 1 that crashes the TS parser |

A mutation test confirmed the check catches real problems: swapping in Rust's default `{:.2}` for PPB makes 14 real files differ (8.125 shows as `8.13` in YF, `8.12` in Rust). The data has 463 such tie values in PPB and PPG.

## Save parity (2026-11-12)

`yf-core/src/writer.rs` writes `.yft` files. On all 122 files that load, its output is byte-identical to the TS app's, and so are the stats after reopening and the result of a second save. Getting there needed:

- **Key order.** `camelCaseToSnakeCase` moves each renamed key to the end of its object, in the order of its rename list (duplicates included). The writer builds camelCase objects in `toFileObject` order and replays that pass on an ordered `serde_json::Value`. A mutation test (moving one entry in the list) makes every file differ.
- **Numbers** print the JS way (`5`, not `5.0`; NaN as `null`), through `js::num_to_string`.
- **Dates.** dayjs reads a date with no `Z` (e.g. `"2026-04-17"`) as local time, so the app re-saves it as local midnight in UTC, which depends on the Mac's time zone. The writer does the same with chrono's local zone. Checked against the app under `TZ=America/Los_Angeles` and `TZ=Asia/Kolkata`.
- **`points: null`** in a saved file: the app keeps it as null, and the Rust loader now does too.

Not covered, because none of the real files have them: question-level (MODAQ) match data, which yf-core doesn't load, so a save would drop it; date strings ending in `Z` other than `toISOString()` output (the writer returns an error); and YfData values the TS loader passes through untouched (wild-card rules, pool seeds and advancement rules, bonus tracking mode). Those three are rebuilt from the model, which gives the same bytes for anything YF itself wrote.

## Save and reopen in the TS app (2026-11-12)

The oracle now also saves each file the way File > Save does, reopens the saved file, and dumps it again (`saved`, `reloaded`, `resaveIdentical`, `resaveDiff` in the TS dumps). Before the fixes below, importing an old file and saving it went wrong on 16 of the 116 legacy files:

| Bug | Files | Fix |
|---|---|---|
| Forfeits imported from old files kept their tossups read, so PPTUH changed after a save and reopen | 6 | `OldYfParsing`: forfeits get no tossups read, as the `.yft` loader already did |
| Team names with trailing spaces produced IDs like `Team_Alpha ` that the loader (which trims names) couldn't resolve, so the saved file wouldn't open | 2 | Trim team and player names on import; the loader falls back to the trimmed ID for files already saved this way |
| A seeded team with no players: the loader skips the team, then failed on the seed pointing at it | 2 | Seeds and pool entries pointing at a skipped team are dropped |
| A roster entry with a blank name: the saved file wouldn't open | 3 | Skip blank names on import; the loader skips them with a warning in files already saved |
| Player names with trailing spaces were trimmed only on reopen | 3 | Trimmed on import |

After the fixes, no file changes stats on save and reopen and every saved file reopens. `src/__tests__/OldYfRoundTrip.test.ts` covers each case with a synthetic file. Saving the reopened file again gives the same bytes for 107 of 122 files; the other 15 differ only in player ID counter numbers (13) or in the dropped empty team (2). yf-core has the same import and loader fixes and still matches on all 122 files.

## JS behaviour that had to be reproduced (see `yf-core/src/js.rs`, `jsval.rs`)

- `toFixed` rounds ties away from zero; Rust rounds them to even.
- `Number#toString`, `Math.round` (halves go up, not away from zero), and `String#trim` (U+FEFF yes, U+0085 no) all differ from their Rust counterparts.
- `Array#sort` tolerates NaN comparators. `js::sort_by` is a stable merge sort that can't panic.
- `localeCompare` sorts registrations. The port uses ICU4X root collation, the same as Node's ICU.
- `for...in` key order (integer-like keys first) sets roster and player order in legacy files.
- Legacy files store `null` scores in 51 games. JS treats null as 0 in arithmetic but not in `===`, which changes the tie warning and score-mismatch validation; see `MatchTeam::points_null`.
- Pre-2.5.2 files coerce fields with `+x` (`undefined` becomes NaN, `""` becomes 0).

## Not done yet (rest of milestone 1, then milestone 2)

- Error message text: load failures match on *whether* a file fails, not on the wording.
- HTML report, SQBS export/import, QBJ export, qbj/MODAQ match import, schedule templates. These are milestone 2.

## Layout

- `yf-core/src/js.rs`, `jsval.rs`: JS semantics helpers, with unit tests.
- `model.rs`: teams, players, phases, pools, rounds, matches and match validation (arena + indices instead of shared references).
- `validation.rs`: `MatchValidationMessage` / `MatchValidationCollection`.
- `legacy.rs`: `OldYfParsing.ts`.
- `yft.rs`: the `.yft` load path (`FileParsing.ts`, `CaseConversion.ts`, `QbjUtils2.ts`).
- `stats.rs`: `StatSummaries.ts`.
- `writer.rs`: the `.yft` save path (`toFileObject` in each model class, `camelCaseToSnakeCase`, `TournamentManager.makeJSON`).
- `tournament.rs`: the parts of `Tournament.ts` that parsing and stats use.
- `yf-parity/`: dump and diff CLI.
- `oracle/dump.ts`: the TS side.
- `oracle/anonymize.ts`: makes the sample files.
- `samples/`: anonymized real files for CI.
