# yf-core: Rust port of the YellowFruit core (milestone 1, in progress)

A Rust port of the data model, file loading and stats engine from `src/renderer/DataModel/`. Each piece gets parity-tested against the TypeScript app on real tournament files before anything else is built on it. See `../RUST_PORT_ASSESSMENT.md` for why this comes first.

## Run the parity check

```bash
rust/parity.sh                 # every .yft under ~/Downloads/_QuizBowl
rust/parity.sh /some/folder    # or any folder
```

It needs `node_modules` (`npm ci --ignore-scripts` is enough) and cargo, and takes about 15 seconds. Steps:

1. `rust/oracle/dump.ts` loads each file with the real TS DataModel (headless, no Electron) and writes `<name>.ts.json`.
2. `yf-parity dump` loads the same files with `yf-core` and writes `<name>.rs.json` in the same shape.
3. `yf-parity diff` compares them field by field. Numbers compare as f64, strings exactly. It exits 1 on any difference (`diff-all` prints every difference, not just the first few).

The dumps hold real team and player names, so they live in `rust/target/parity/`, which git ignores.

## What a dump covers

- **Structure:** scoring rules, registrations, teams, rosters, seeds, phases, pools, rounds, packets, and every match: score, TUH, OT, carryover phases, whether it's omitted from stats, and the exact error and warning text from match validation.
- **In-app standings:** `compileStats(false, true)`, i.e. per-phase pool standings with final-rank sorting.
- **Stat report:** `compileStats(true)`, i.e. per-phase standings and individuals, prelims+playoffs, cumulative team, player and round-report stats. It includes the display strings (`ppg`, `ppb`, `winPct`, `gp`, bounceback %), not just raw numbers.

## Status (2026-11-11)

`139 files: 122 identical, 17 fail to load in both, 0 differ`

| | Files | Result |
|---|---|---|
| Legacy YF 2.x/3.x | 116 load | all identical |
| YF4 `.yft` | 6 load | all identical |
| Fail in both apps | 17 | same failure: 9 old files with unescaped quotes in names (invalid JSON written by old YF), 6 that aren't YF4 files (MODAQ/qbj), 2 LIQBA seed files with inconsistent bonus settings plus 1 that crashes the TS parser |

A mutation test confirmed the check catches real problems: swapping in Rust's default `{:.2}` for PPB makes 14 real files differ (8.125 shows as `8.13` in YF, `8.12` in Rust). The data has 463 such tie values in PPB and PPG.

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

- **Save parity in Rust:** port `toFileObject` and the camel-to-snake pass (it appends snake_case keys in a fixed order, so port it as a transform on an ordered `serde_json::Value`), then diff the bytes against the oracle's `saved` field. The TS round trip is already in place to compare against.
- Error message text: load failures match on *whether* a file fails, not on the wording.
- HTML report, SQBS export/import, QBJ export, qbj/MODAQ match import, schedule templates. These are milestone 2.

## Layout

- `yf-core/src/js.rs`, `jsval.rs`: JS semantics helpers, with unit tests.
- `model.rs`: teams, players, phases, pools, rounds, matches and match validation (arena + indices instead of shared references).
- `validation.rs`: `MatchValidationMessage` / `MatchValidationCollection`.
- `legacy.rs`: `OldYfParsing.ts`.
- `yft.rs`: the `.yft` load path (`FileParsing.ts`, `CaseConversion.ts`, `QbjUtils2.ts`).
- `stats.rs`: `StatSummaries.ts`.
- `tournament.rs`: the parts of `Tournament.ts` that parsing and stats use.
- `yf-parity/`: dump and diff CLI.
- `oracle/dump.ts`: the TS side.
