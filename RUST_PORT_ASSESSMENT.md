# YellowFruit → Rust/egui: port assessment

Study only, written 2026-11-11 against master `03ddded2` (Electron 45, MUI 9, React 19). No Rust code was written except a one-file rounding spike (see Risk 1).

## Recommendation

**If you port, do Option B, in stages: a Rust core crate first, proved against the TypeScript app on all 139 real `.yft` files, and only then an egui UI.** Treat Milestone 1 (below, 3–4 weeks) as a go/no-go gate. It is the only part that is cheap, testable and useful whatever you decide next: the parity CLI it produces also catches regressions in the current TS app.

Don't do Option C (Rust/WASM inside Electron). It costs most of a rewrite and gives you almost nothing: stats run fast enough already, and the UI is tightly bound to mutable TS objects.

If the real goal is "less Electron" (smaller app, fewer dependency upgrades) rather than "Rust everywhere", Option D (Tauri, keeping the React/MUI UI) costs about a quarter of a full port and keeps MUI's accessibility.

## 1. Map of the app

Line counts are TS/TSX lines. Total is about 29k.

| Area | Where | Lines | What it does |
|---|---|---|---|
| Data model | `DataModel/{Tournament,Phase,Pool,PoolTeam,Round,Match,MatchTeam,MatchPlayer,MatchQuestion,Team,Player,Registration,ScoringRules,AnswerType,Rank,…}.ts` | ~6.5k | A class graph with direct object references (e.g. `MatchTeam.team: Team`, `MatchPlayer.player: Player`, `PoolTeam.team: Team`, `Match.carryoverPhases: Phase[]`). Validation lives on the model (`MatchValidationMessage.ts` plus ~70 validate sites in Match/MatchTeam/Team/Player). |
| Stats engine | `StatSummaries.ts` | 859 | `AggregateStandings`, `PhaseStandings`, `PoolStats`, `PoolTeamStats`, player stats. Tie handling, advancement tiers, final ranks, PPB, P/TUH. Fractional games played (`tossupsHeard / tossupsRead`). |
| .yft / .qbj I/O | `FileParsing.ts` (1303), `QbjUtils*.ts`, `CaseConversion.ts` (324), `Interfaces.ts`, each class's `toFileObject()`, `TournamentManager.parseJSON/makeJSON` | ~2.2k | `.yft` is a QBJ 2.1.1 "whole file" (`{version, objects:[tournament]}`) plus `YfData` extensions, snake_cased on write. Objects are linked by `id` and `$ref`. Dates go through dayjs ISO strings. `.qbj` import/export uses the same code with `qbjOnly`. Fuzzy name matching (threshold 0.8) for NAQT and game imports. |
| Legacy YF 1–3 files | `OldYfParsing.ts` | 625 | Six newline-separated JSON sections. **128 of your 139 real files are this format** (67 × v3.0, 57 × v2.5, 4 × v2.1). Only ~11 are YF4 `.yft`. |
| SQBS | `SqbsFileGeneration.ts` (447), `SqbsParsing.ts` (110) | 557 | Line-oriented text export (one file per phase or combined) and a team/roster import. |
| HTML reports | `HTMLReports.ts` | 1417 | Pure string building: standings, individuals, scoreboard, team detail, player detail, round report, plus inline CSS. 26 `toFixed`/format sites. |
| Schedule templates | `DataModel/Schedules/*.ts` (51 files), `ScheduleUtils.ts`, `Pool.ts` helpers | ~4.9k | Declarative `StandardSchedule` objects for 4–36 teams; `constructPhases()` calls `makePoolSet`, `snakeSeed`, `makePlacementPools`. Mechanical to port. |
| App state | `TournamentManager.ts` (1675), `Modal Managers/` (9 files, 1548) | ~3.2k | Owns the open tournament, unsaved flag, file path, toasts, and IPC. Each dialog edits a temp copy (`TempTeamManager`, `TempMatchManager`, …) and commits on Accept. |
| Main process | `src/main/` | 1074 | `main.ts`: window, close guard, crash relaunch, opening a file from argv, autosave every 2 min. `FileUtils.ts`: ~10 open/save dialogs, backups (`userData/yfBackup/prod_backup.yftbak`, recovered on launch), writing report HTML, the `yf-stat-report://` protocol that serves report files into the iframe. `menu.ts`: native menus and accelerators (Ctrl/Cmd+N/O/S/U, Shift+M). `UpdateChecker.ts`: GitHub latest-release API via Octokit. |
| UI | `Components/` (31 files, 6.6k) | 6.6k | Pages: General, Rules (6 settings cards), Schedule (picker plus detail), Teams (standings and seeding views), Games (pool view), Stat Report (iframe). Dialogs: Match edit (1135 lines, drag-and-drop, autocomplete), Team edit (553), Phase, Pool, Rank, Pool assignment, Match import results, SQBS export, Tossup values, About, Generic confirm, plus the NavBar drawer. 11 components use `useHotkeys` Alt+letter accelerators. 4 Autocomplete and 4 Draggable uses. |
| Tests | `src/__tests__/` | 738 | 12 files, 70 tests. Unit level only; no stats-output snapshots. |

## 2. What ports cleanly and what doesn't

**Moves cleanly to Rust**

- **Schedules.** They're data plus small constructors. About 1 week including tests that compare the generated pools with the TS output.
- **Stats math.** Plain arithmetic over the model, with stable sorts. It needs the rounding and sort fixes in Risks 1–2, but nothing about it is hard.
- **SQBS export/import, HTML report generation.** Both are string building. Port them as `write!` calls or `maud` templates, and diff the output byte for byte.
- **QBJ/.yft with serde.** Doable, but not "just derive". The `$ref`/`id` graph needs a two-pass resolver into an arena (`slotmap`, or `Vec` plus typed ids), because Rust can't hold a `Team` referenced from many `MatchTeam`s the way the TS classes do. The snake/camel conversion and the qbjOnly/YfData split need custom serializers. Budget 2 weeks, not 2 days.
- **Main-process plumbing.** `rfd` for file dialogs, `std::fs` plus a timer for autosave and backup, `ureq` for the update check, `open` for launching the browser. About 1 week.

**Hard in egui**

- **Form dialogs and validation** (the bulk of the UI work). egui is immediate-mode: no form library, no built-in error-text-under-field, no autocomplete, and focus handling is basic. Match edit, with its per-player tossup grid, drag-to-reorder, team autocomplete and live validation, is 1.5–2 weeks alone. The temp-copy-then-commit pattern maps well (clone the struct, edit it, swap it in on Accept).
- **The stat report.** egui can't render HTML. Choices: (a) open the generated HTML in the system browser (already a menu action); (b) embed a `wry` webview, which works but is awkward to layer with egui and brings back a browser engine; (c) redraw the standings natively with `egui_extras::TableBuilder`, which means maintaining the report in two places. Recommend (a), plus a native standings table on the Teams page, which already exists as `TeamsPageStandingsView`.
- **Menus and shortcuts.** egui has no native menu bar, and macOS users expect one. Use `muda` for native menus and accelerators. Re-implement the Alt+letter dialog shortcuts with `ctx.input(|i| i.modifiers.alt && i.key_pressed(..))` per dialog. Text-edit copy/paste and undo already work.
- **macOS "open with" / double-clicking a `.yft`.** This arrives as an Apple Event (`application:openFiles:`), which winit/eframe does not expose cleanly. Windows passes argv and is fine. Expect a small objc2 shim, about 2–3 days.
- **Theming.** The yellow Materialize look can be approximated with `egui::Visuals` and an embedded Roboto font. It won't be pixel-identical (no ripples, elevation or floating labels). This is cosmetic, but it's work: about 3–5 days to look deliberate.

## 3. Options

Estimates are full-time weeks for one developer comfortable in Rust, including parity testing. Double them for evenings-and-weekends pace.

| # | Option | Effort | Gains | Costs / risks |
|---|---|---|---|---|
| **B (recommended)** | **Staged: Rust core crate, then egui UI.** Core first (model, all file formats, stats, reports, schedules), gated on parity, then the egui shell. | **Core 6–8 wk + UI 8–11 wk + packaging 1–2 wk = 15–21 wk.** First usable output (parity CLI) at wk 3–4. | Native ~10–20 MB app, no Node/Electron upgrade churn, one language. Parity is proven before any UI is written. You can stop after the core with something useful. | Longest total. Accessibility regresses (Risk 4). Two codebases until cutover. |
| A | **Big-bang full egui rewrite.** Port everything together and switch when it's done. | 14–19 wk | Slightly less total effort than B (no intermediate CLI or gate). | Nothing works until near the end, and parity bugs surface late, mixed with UI bugs. Not worth the ~2 weeks it saves over B. |
| C | **Keep Electron, move the core to Rust/WASM** (`wasm-bindgen`). | 10–14 wk | Keeps MUI and its accessibility. | The UI mutates TS model objects directly everywhere (Temp*Managers, `TournamentManager` methods). Crossing a WASM boundary means a handle-based API or serializing on every edit, and rewiring about 3k lines of manager code. It doesn't remove Electron. The gain (speed) solves a problem you don't have: your largest file is under 500 KB. |
| D | **Tauri, keep React/MUI.** Swap the Electron main process for Rust and leave the renderer alone. | 3–5 wk | Smaller app, fewer Electron upgrades, keeps the UI and accessibility, and the stats code is untouched (no parity risk). | Still a webview app, using WebKit on macOS and WebView2 on Windows, so CSS and iframe behaviour needs a QA pass. The `yf-stat-report://` protocol maps to Tauri custom protocols. |
| 0 | **Stay on Electron.** | 0 | No risk. | Electron upgrade work twice a year (you just did Electron 45). |

## 4. Risks

1. **Number formatting doesn't match JS. Confirmed by a spike.** Rust `format!("{:.1}")` rounds exact ties half-to-even; JS `toFixed` rounds them up. Output from rustc 1.95 vs node:

   | value | JS `toFixed(1)` | Rust `{:.1}` |
   |---|---|---|
   | 0.25 | 0.3 | 0.2 |
   | 1.25 | 1.3 | 1.2 |
   | 0.125 (`toFixed(2)`) | 0.13 | 0.12 |

   This affects real output: games played is `Σ tossupsHeard / tossupsRead`, so a player who hears 5 of 20 tossups shows `0.3` in YF today and would show `0.2` in Rust. Fix: a `js_to_fixed(x, n)` helper, tested against node on a value sweep. Separately, `serde_json` writes `1.0` where JS writes `1`, so whole-number fields must be integer types, or the saved `.yft` changes.
2. **Sort semantics.** JS sorts are stable and tolerate inconsistent comparators. Since Rust 1.81, `sort_by` may **panic** when a comparator violates total order, which a comparator returning NaN (e.g. PPB of a team with no bonuses heard) does. Use `total_cmp` or explicit NaN-last rules, and keep the stable `sort_by`. Two `localeCompare` sorts (registrations, team letters) need a defined collation, because byte order differs for accents and case.
3. **Round-trip and stats parity on real files.** 128 of the 139 files are legacy YF 2.x/3.x. For them, "parity" means *legacy import → stats* and *legacy import → saved `.yft`* match between TS and Rust; they can't round-trip in their own format. The earlier overtime double-count bug shows the TS side has bugs of its own. Decide per bug whether Rust matches it (for parity) or fixes it, and record that in an allow-list.
4. **Losing MUI's accessibility.** egui has AccessKit (screen readers on Windows and macOS), but labelled fields, error-text association, focus trapping in dialogs and tab order all have to be built by hand per dialog, and the result will be thinner than MUI. If anyone running tournaments with you uses a screen reader, this weighs toward D or 0.
5. **Windows/macOS packaging.** electron-builder does a lot today: icons, `.yft` file association, DMG/NSIS, signing. Rust replacement: `cargo-packager` (or `cargo-bundle` plus `cargo-wix`). File associations go in `Info.plist` and the Windows registry by hand. macOS codesign and notarization are unchanged in kind. Budget 1–2 weeks, plus the Apple-Event shim from section 2. Build for both on GitHub Actions, not on this Mac (disk).

## 5. Suggested first milestone: parity on real files (3–4 weeks)

Goal: prove that a Rust core reads all 139 files and produces the same stats and the same saved file as the TS app, before any UI work.

1. **TS oracle script (2–3 days).** A `tsx` script over the existing DataModel, headless like the earlier overtime investigation. For each file, write `out/<name>.ts.json` holding (a) the saved `.yft` JSON after `parse → toFileObject` and (b) a stats dump: every phase's `PhaseStandings` and `PoolStats` rows, aggregate standings and individual stats, with numbers formatted exactly as `HTMLReports.ts` formats them.
2. **Rust `yf-core` (2–3 weeks).** Arena-based model, `.yft` and legacy-YF parse, `.yft` serialize, `StatSummaries` port, `js_to_fixed`. Leave out SQBS, QBJ export, HTML reports and schedules.
3. **`yf-parity` CLI (2 days).** Same dump format; a structural JSON diff per file; a summary line like `139/139 stats identical, 139/139 save identical`.

Exit criteria: zero unexplained diffs across all 139 files, any known TS bugs listed explicitly, and a CI job that runs both sides on a committed sample of 5–10 anonymised files. If this takes more than 5 weeks, stop and take Option D or 0.

### Progress (2026-11-11)

Steps 1 and 3 are done, and step 2 is done except `.yft` serialize. Code is in `rust/`; run `rust/parity.sh`. Result: **122 of 122 loadable files are identical** on structure, match validation, in-app standings and the full stat report. The other 17 fail to load in both apps, for the same reasons. Things learned along the way:

- 128 of the 139 files are legacy YF 2.x/3.x; only 11 are YF4.
- The rounding risk is real. 463 displayed PPB/PPG values in your data sit on exact ties, and plain Rust formatting gets 14 files wrong.
- Old files contain `null` scores (51 games), which need JS null semantics to keep match validation identical.
- 17 of your files can't be opened by the current app. Nine are old files with unescaped quotes in team or player names, which a lenient loader could fix.

- Save parity is done (2026-11-12): the Rust writer's `.yft` output is byte-identical to the app's on all 122 loadable files, and so are the stats after reopening. Building the round trip also turned up save/reopen bugs in the TS app itself on 16 old files, now fixed in both (see `rust/README.md`).

- CI (2026-11-12): `.github/workflows/parity.yml` runs the parity check on 11 anonymised samples in `rust/samples/`.

Milestone 1 is done. Milestone 2 is done: the QBJ export, the SQBS export and all six HTML report pages match the app byte for byte on all 122 loadable files and the 15 CI samples (see `rust/README.md`). Next is the egui UI, starting with the read-only pages. The per-bug allow-list it planned for wasn't needed: every difference found was fixed on one side or the other.

Milestone 2 (2–3 weeks) would add SQBS, QBJ and HTML report output, with byte-diffs against the TS output for the same files. Only after that, start the egui UI, beginning with the read-only pages (Teams standings, Games list) before the dialogs.
