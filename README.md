# Tournament Manager — Tauri / Rust proof of concept

For Windows builds and the supplied Windows drivers, see [WINDOWS.md](WINDOWS.md).

The desktop GUI uses **Tauri 2** with a local HTML/CSS/JavaScript interface. Scoring, scheduling, rankings, brackets, passwords, SQLite storage, and V5 hardware control remain in Rust. New tournaments use **2026/2027 VEX IQ Level Up** and **VEX V5 Override**, decoded from the supplied `.tmg2` definitions (Level Up v1 and Override v2). The 2025/2026 rules remain embedded for existing tournament files.

## Run

Open the packaged `Tournament Manager Rust.app`. The Apple Silicon app uses macOS system WebKit; Node, Python, REA, and a Rust toolchain are not required to run it. Try either demo on the welcome screen for a fictional 24-team tournament, or choose **Create a new tournament**.

The 12-step walkthrough starts with competition type, tournament preset, event name/network password, game, teams, finals setup, fields, skills, displays, schedule, match creation, and awards. Creating a tournament first prompts for a new `.tmrs` file, then opens setup. Completing setup saves into that file. Existing `.tmrs` files remain compatible. Local reopening does not request the network password; optional Protect Options gates setup access.

## Scheduling

**Schedule setup** now presents an event-day plan rather than a dense configuration form:

- **Build a day plan** creates practice and qualification blocks from target matches per team.
- Add, edit, duplicate, or remove dated blocks; gaps represent lunch and other breaks.
- Set cycle time in minutes and seconds. It includes the match, reset, and changeover.
- **Fit duration** reserves enough cycles for complete rounds.
- Capacity summaries show separate practice and qualification counts and approximate appearances per team.
- **Preview team appearances** runs the Rust scheduler and reports actual counted matches, excluding surrogates.
- Validation rejects invalid dates, overlaps, unsupported match types, and blocks shorter than one cycle. Warnings flag short reset times and uneven counted appearances.
- Generate practice and qualification matches separately, with confirmation before replacement and a schedule preview.

Capacity currently assumes configured fields run simultaneously. For a field set that alternates matches, configure one scheduling field. Multi-field rounding can introduce additional capacity; inspect the actual appearance preview. Changing game, roster, fields, or blocks clears generated draft matches so they can be recreated. Changes apply only when the walkthrough finishes.

## Tournament operations

The GUI includes team CSV/original-TM imports, check-in and inspection, match/skills score controls from recovered rules, validation, qualification rankings, strictly **2v2 V5 eliminations**, IQ Teamwork finals, awards, audience announcements, sponsor images, data exports, and printable schedules by time or by team. Browser reports use their **Print schedule** button or ⌘P.

Pit, field, and audience displays open as separate Tauri windows. Configure fullscreen and assigned fields in setup. Pit rankings use the reference split layout: white/gray rankings and interleaved sponsor logos scroll infinitely on the left; six upcoming match cards appear on the right. Read-only remote displays can be started explicitly from **Setup**, on port 8790. The server root opens an HTML password sign-in and a chooser for all configured pit, field and audience displays. Selecting one requests browser fullscreen; the display toolbar changes displays or re-enters fullscreen. Root displays and /TM use the same expiring session login. This is a local-network HTTP prototype, not the original TM RPC protocol or a public Internet service.

## V5 hardware

Use **Setup → Field hardware** to discover and connect a V5 controller. The app bundles the supplied original `v5field-darwin-amd64.so` in Resources and loads it through the packaged Intel helper. Rosetta is required on Apple Silicon. The packaged app no longer requires the original TM installation for this driver.

A Rust worker runs field timing and controller heartbeats independently of the webview. Start, pause, resume/driver, abort, reset, and shutdown drive the original control interface; the helper disables after two seconds without a heartbeat. Controllers are never opened or enabled automatically. Actual driver discovery reported zero devices. Fake-driver command and watchdog tests pass; physical controller/robot behavior remains unverified. Classic Cortex, hardware-button callbacks, original RPC interoperability, RobotEvents uploads, leagues/multiple divisions, complete original database conversion, and advanced media/reports are not implemented.

## Build and test

Install Rust and macOS developer tools, then:

```sh
cargo build --locked --bin tournament-manager-rs
cargo run --locked --bin tournament-manager-rs
cargo test --locked --no-default-features --tests
npm ci
npm test
```

The GUI is plain local web assets: no npm build step is required. npm dependencies are used only for frontend tests. See `tools/package-macos.sh` for packaging and the Intel V5 helper. Source runs can set `TMRS_V5_HELPER` to a compiled helper's absolute path. The CLI remains available with `cargo run --no-default-features --bin tmrs -- games`.

Tests cover 400 scoring/constraint comparisons with the recovered Python sources, scheduling, appearances, date/block validation, password behavior, persistence, brackets, reports, and an automated frontend walkthrough with a mocked native bridge. That walkthrough tests DOM behavior, not physical hardware. The previous egui GUI is retained only in `legacy/`, behind `--no-default-features --features legacy-egui --bin tmrs-egui-legacy`; it is not the default GUI.

Architecture follows Tauri’s documented [Rust command interface](https://v2.tauri.app/develop/calling-rust/) and [window capabilities](https://v2.tauri.app/security/capabilities/). Display windows can only request read-only snapshots; they cannot run operator commands. Password hashes are removed from UI snapshots and JSON exports.

Recovered game definitions and reference scoring source retain their original ownership. This independently labeled proof of concept has no original-vendor affiliation. Original-analysis evidence and remaining reconstruction limits are in `analysis/`.


## Readex Pro, field control, and display editing (0.3.0)

The existing software font is bundled locally under `frontend/fonts/`, including its license; it is preserved unchanged. It applies to operator screens, the field-control popout, local/remote displays, and printable reports. No external font request is needed.

Match selection now opens scoring only. Use **Queue to field control** on a match, then **Load next match** in the separate **Match Field Set #1** window. Queueing/loading never starts a match. Start, pause, resume/driver, end early/disable, reset, field choice, autonomous bonus, saved result information, and audience screen selection live in this window. A running or paused match cannot be replaced. Audience modes include None, Logo, Up Next, In-Match, Saved Match Results, Schedule, Rankings, Skills Rankings, Alliance Selection, Elim Bracket, Award Slides, and Inspection. Field control currently manages one active match/field set at a time.

In **Displays → Edit displays & logos**, add/remove/name displays, change their type, field, screen and fullscreen setting; add, replace, rename, reorder, or remove sponsor logos. Apply changes updates open display windows. This does not recreate matches or reopen tournament creation. Applied changes save automatically. Protected options still require the network password.

**Setup → Field hardware → USB diagnostics** shows macOS USB visibility separately from the original driver's controller count. **Rescan** refreshes the count without recreating the driver process. Detection still requires macOS to enumerate the V5 Brain; a hub or drive appearing in USB inventory does not establish a Brain connection. No real controller was opened or enabled during this update's validation.

The operator UI now uses compact 15 px body text, smaller controls, tighter tables/cards, a narrower wizard sidebar, and reduced headers. The main window defaults to 1240×860 and field control to 1000×720. Radio sizing and grid minimum widths are explicit to prevent clipped text and horizontal overflow. Public pit/field/audience typography remains designed for distant viewing.

Audience list screens now show every row in a continuously repeating scroll with fixed table headings. This applies to rankings, skills rankings, schedule, inspection, alliance selection, and elimination bracket screens. Match timers and scoreboards remain stationary; award slides continue rotating.

## Browser match control

Start **Setup → Start event server**, then visit **http://localhost:8790/TM**. Enter the tournament network password on the HTML sign-in page. No username is required. Both displays and `/TM` require authentication, including on localhost. On another computer, replace localhost with the server computer's IP address.

The browser operator can select matches for score entry, save drafts, submit validated scores, queue matches, and open a field-control popout for loading/starting/pausing/resuming/ending/resetting matches and changing audience screens. It shares the desktop tournament and original Rust validation. Setup, file dialogs, hardware configuration, and arbitrary native commands are excluded from the browser API. Protected operator requests use same-origin JSON POSTs; read-only displays remain available separately.

Browser match control now has phone layouts with stacked panels, 44–46 px touch targets, 16 px form fields, responsive score inputs, and same-tab field control with a back link. Password sign-in uses a random eight-hour HttpOnly, SameSite=Strict session cookie. Sign out revokes it; changing the tournament password invalidates existing sessions. The /TM interface no longer uses the browser Basic-auth dialog.

Field displays omit their display-name heading, and native field windows use the assigned field name. Pressing Start now begins a three-second pre-start countdown; match time and hardware enable begin only afterward. Abort cancels the countdown. Original TM Start/Pause/Stop/Abort sounds and season-specific driver warnings play on the server Mac; toggle them with Play match sounds on server in field control. Sound files are bundled in the macOS app.

## Supplied 2026/2027 definitions

Scoring, scoring constraints, control ranges and choices, rankings, match timing and warning sounds come from the supplied game files. The app runs the compiled rule IR in Rust; Python and protobuf are used only in developer extraction and comparison tools. Game-source versions are preserved; support does not establish that these files match later published rule updates.

`analysis/GAMES-2026.md` records input hashes, decoded behavior, checks and limits. `tools/decode-game-definitions.py` decodes `.tmg2` data with the recovered protobuf descriptor without executing embedded code. `tools/export-game-rules.py` compiles decoded JSON into rule IR. `tools/generate-fixtures.py` executes the retained reference rules only for developer parity tests.

## Override field scoring

The scoring screen includes the supplied quadrant diagram with selectable Red 1 (left), Red 2 (bottom), Blue 1 (right), Blue 2 (top) and Center regions. Select a region to edit its pins and toggle ownership. The same layout supports skills and the mobile /TM scoring page. Field reference expands the second supplied diagram. Switching regions preserves current inputs.

Audience and field displays show a server-clock-based **Scheduled start in: mm:ss** countdown for the loaded match before it starts. Overdue starts show 00:00; unscheduled matches show no countdown.

## Ranking displays

Qualification and skills ranking screens use the same pit split layout on both pit and audience displays, with infinite team/sponsor scrolling and upcoming matches. On the Displays page, choose Qualification rankings or Skills rankings for each pit display; the selection saves and updates connected viewers. Ranking rows, alliance-selection cards and in-match displays show team names with numbers.

TM field control includes **Show score preview** on desktop and mobile. Live audience/field scores update from score-entry drafts during a match. The visibility choice saves with the tournament; explicitly displayed submitted results remain visible. Automatic saved-result display changes occur only on final submission.

## Compact match controls and PDF reports

The tab bar replaces the main header, with Field Control on the right. New/Open/Save and event-server controls live in Setup. Match selection lists team numbers only, preserves scroll position and centers newly loaded field matches. Prestart displays show a large single 3, 2 or 1.

Reports exports native PDFs for schedules by time/team, team lists, qualification rankings (marked provisional until complete), skills rankings, elimination brackets/finals, match results and awards. PDFs embed the same bundled font as the software, use purple accents and outlined tables; elimination bracket entries contain team numbers only. Large brackets use a suitably sized landscape PDF page to preserve legibility.

## Resizing public displays

Audience, field and pit screens size typography against both window dimensions. Portrait rankings stack upcoming matches underneath; clocks fit their available area without covering teams or scores. Alliance-selection cards rotate through pages every eight seconds when all pairs cannot fit legibly. Large brackets can be panned inside small windows. Narrow/short layouts prioritize the main timer, teams and scores.

Schedule PDFs (by time or team), team-list PDFs and printable HTML schedules highlight teams consistently by organization, with an organization color key. Team lists tint rows; mixed-organization matches highlight individual team numbers. Matching ignores capitalization and repeated whitespace; unassigned organizations stay neutral.
