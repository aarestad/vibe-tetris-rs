# tetris-rs: Implementation Status vs. 2009 Tetris Design Guideline

Nov 6, 2026, updated Nov 7, 2026 · @Peter Aarestad

## Summary

tetris-rs now plays as Guideline Tetris: of 37 Guideline areas, 30 are met, 6 partial, 0 wrong and 1 missing. Before this pass the counts were 7 met, 13 partial, 6 wrong and 11 missing. All 12 listed defects are fixed, and the core engine and scoring rules are in place.

What remains:

- **Game flow:** the High Score Table, End Rewards, game statistics, a three-page Help and the high score during play are in. Starting Blocks remain.
- **Sound:** all six required sound effects and most recommended ones now play, and Peter has confirmed they sound good.
- **Basic terminals:** Auto-Repeat and 20x Soft Drop need a terminal that reports key releases. Elsewhere, holding a key falls back to OS key repeat.

One risk still needs your decision: the Guideline PDF, which is marked CONFIDENTIAL, and its extracted text are removed from `main`. They remain in the public GitHub repo's history.

This report describes `main` with the high score field added on top of commit `d881fe6`. Locally it passes lint and all 372 tests. CI's latest run, for `e0f4a62`, passed lint and all 367 tests then on Linux and macOS. Peter has listened to the music and confirmed that held keys work in Ghostty, and that the sound effects sound good.

## Current state

The code is a 10,593-line, single-player Rust terminal game on `main`, built on ratatui, crossterm and rodio. The program opens on a Main Menu, and each game runs in a `Session` with no terminal or sound device, so `Game` only does I/O. Game flow runs through a `Phase` enum, pause and help share a `Screen` enum, and scoring, sound effects, high scores and End Rewards have their own modules.

| Module | Lines (with tests) | Tests | Responsibility |
| --- | --- | --- | --- |
| `src/game_state.rs` | 1,865 | 83 | Phases, generation, gravity, Lock Down, Hold, T-Spin recognition, seedable bag, sound events |
| `src/ui.rs` | 2,082 | 51 | ratatui rendering: menu, Options and High Scores pages, Help pages, Matrix, queues, high score field, progress bar, overlays, Game Over statistics, name entry, fireworks, too-small message |
| `tests/end_to_end.rs` | 989 | 35 | Runs the real binary in a pseudo-terminal and reads its screen |
| `src/menu.rs` | 801 | 29 | Main Menu choices, the Options page and the High Scores page, with no terminal |
| `src/scoring.rs` | 651 | 28 | Score, Back-to-Back, level, Goal, the Sprint line count, Game Statistics and the scoring table for Help |
| `src/session.rs` | 597 | 27 | Input and time handling for one game, with no terminal or sound |
| `src/input.rs` | 519 | 19 | Play, menu and name entry key mapping, key press and release events, Auto-Repeat |
| `src/high_scores.rs` | 516 | 13 | High Score Table per variant, ranking, saving and loading, name entry |
| `src/board.rs` | 490 | 38 | Matrix and Buffer Zone storage, collision, Line Clear |
| `src/tetrimino.rs` | 352 | 11 | Facings, Tetrimino shapes, generation position, Super Rotation tables |
| `src/game.rs` | 314 | 0 | Connects the menu and each game to the real terminal and sound device, and saves options and high scores |
| `tests/skilled_play.rs` | 266 | 4 | A bot plays seeded Marathons, Sprints and Ultras through real key events |
| `src/config.rs` | 259 | 9 | JSON config with defaults, validation and saving, including the game mode |
| `src/audio.rs` | 238 | 8 | Embedded background music and sound effects via rodio |
| `src/sound.rs` | 224 | 7 | Sound effects as synthesized notes, rendered to samples |
| `tests/random_play.rs` | 111 | 1 | Seeded random key events in all game, Lock Down, goal and input modes |
| `src/end_reward.rs` | 95 | 2 | Which games earn an End Reward, its message and how many fireworks |
| `src/screen.rs` | 143 | 7 | Playing, Paused and Help screens, the Help pages and the keys that switch them |
| `build.rs` | 39 | 0 | Converts the Korobeiniki MIDI to WAV with `timidity` |

- **Tests:** `cargo test` passes all 372 tests: 332 in the library, 35 end-to-end, 1 random play and 4 skilled play. The end-to-end tests take about 5 seconds, run only on Unix, and always turn sound off.
- **Lint:** `cargo fmt --check` and `cargo clippy --all-targets` are both clean, with 0 warnings.
- **CI:** `.github/workflows/ci.yml` runs on every push and on pull requests from forks. It checks formatting and clippy on Linux, with warnings as errors, and runs all tests on Linux and macOS. The Linux test job also requires the music to be embedded. Its latest run, for `e0f4a62`, passed lint and all 367 tests on both. The high score field is not pushed yet.
- **Dependencies:** `rand` is on stable `0.10.3`. `portable-pty` and `vt100` are for tests only, so the shipped game gains nothing from them.
- **Docs:** the README covers the menu, its three modes, the Options page, the Game Over statistics, the High Score Table, End Rewards and Help, running, controls and configuration, including the sound and mode settings, and `AGENTS.md` matches the code.

## Guideline compliance

30 of the 37 areas below now fully meet the Guideline, and none are wrong. Rows follow Guideline order. Section numbers refer to the 2009 Tetris Design Guideline. For rotation, I compared the code against all 28 facing pages, for the I, T, L, J, S and Z.

| Guideline area | Section | Status | What the code does today | Code |
| --- | --- | --- | --- | --- |
| Matrix, 10 x 20 | 2.4 | Met | Default size is correct. Other sizes are validated on load. | `src/config.rs:35` |
| Buffer Zone and Skyline | 10 | Met | 20 rows above the Matrix, with the lowest 2 drawn above the Skyline. | `src/board.rs:4` |
| Tetrimino colors | 3.1 | Met | All seven colors follow the standard scheme. | `src/ui.rs` |
| Bag generation | 3.3 | Met | Seven-piece shuffled bag feeds the Next Queue. | `src/game_state.rs` |
| Generation location | 3.4, A1.2.1 | Met | North Facing on rows 21 and 22, centered as the Guideline lists for each type. | `src/tetrimino.rs:64` |
| Generation timing and first drop | 3.4, A1.2.1 | Met | 0.2 second delay after Lock Down, then an immediate one-row drop if clear. | `src/game_state.rs:390` |
| Tetrimino Facings | A1.3 | Met | All seven shapes rotate within their boxes, including the corrected I North. | `src/tetrimino.rs:79` |
| Movement | 5.1 | Met | One cell at a time with collision checks. Movement snaps, with no smoothing. | `src/game_state.rs:427` |
| Auto-Repeat | 5.2 | Partial | With key release reporting: 0.3 second delay, then 50 ms per move, with opposite-key handling. Otherwise it uses OS key repeat, which can also repeat rotation and Hard Drop. | `src/input.rs:157` |
| Super Rotation System | A1.4 | Met | Five rotation points per facing change, with the Guideline's two unused T points left out. The point used is recorded. | `src/tetrimino.rs:123` |
| Hard Drop | 5.4 | Met | Drops and locks instantly. | `src/game_state.rs:504` |
| Soft Drop | 5.5 | Partial | 20 times Fall Speed with key release reporting. Otherwise one row per key event. | `src/game_state.rs:438` |
| Hold | 5.6 | Met | Once per Lock Down, and the held Tetrimino returns at its generation position. | `src/game_state.rs:663` |
| Lock Down | 2.5.4, 5.7-5.9 | Met | 0.5 second timer with Extended (default), Infinite and Classic modes. | `src/game_state.rs:315` |
| Ghost Piece | 2.4, 2.5.3 | Met | Drawn as a shaded copy, on by default. | `src/ui.rs:418` |
| Next Queue | 2.4, 2.5.1 | Met | 1 to 6 pieces, North Facing, all fitting in 24 rows and next to the Matrix. | `src/ui.rs:471` |
| Fixed Goal System | 6 | Met | 10 lines per level. A higher start sets Goal to 10 x Level for that level. | `src/scoring.rs:135` |
| Variable Goal System | 6, 6.1 | Met | Goal = 5 x Level with the awarded Line Clear table and Back-to-Back bonus. Off by default. | `src/scoring.rs:148` |
| Fall Speed | 7 | Met | Uses the Guideline formula and follows the current level. | `src/game_state.rs:28` |
| Line Clear scoring | 8 | Met | 100, 300, 500 and 800 x Level. | `src/scoring.rs:174` |
| Soft and Hard Drop points | 8 | Met | 1 per Soft Dropped row and 2 per Hard Dropped row, independent of level. | `src/scoring.rs:67` |
| T-Spin recognition | 9 | Met | Checks last move, corner cells by Sides A to D, and Point 5. | `src/game_state.rs:609` |
| T-Spin scoring | 8 | Met | Table totals for all six T-Spin actions, including no-clear T-Spins. | `src/scoring.rs:174` |
| Back-to-Back | 8.1 | Met | The 0.5 x bonus and sequence rules reproduce the Guideline's 5,400-point example. | `src/scoring.rs:72` |
| Game Over conditions | 10 | Met | Block Out and Lock Out work. Top Out can only come from Line Attacks, which are not in single player. | `src/game_state.rs:535` |
| Game information display | 2.4, 7 | Met | Score, level, lines, Goal and time, with hundredths in Sprint and a countdown in Ultra. The mode name is shown in the panel's border. A box under the Next Queue shows the variant's high score, or the current score once it is higher, and in Sprint the best time. It gives way if a smaller Matrix leaves no room. Player name and rank, also listed as possible fields, are left out. | `src/ui.rs:396` |
| Pause | 13.1.6 | Met | Stops play and time, and hides the Matrix and both queues. | `src/ui.rs:548` |
| Controls | 4.1 | Partial | All arrow, letter, Space, Esc and F1 keys work, and numpad keys work as digits. Shift, Ctrl and remapping are absent. | `src/input.rs:103` |
| Visual effects | 13.1 | Partial | Line Clear effects scale with count, and action notifications show outside the Matrix. No Lock Down flash, Hard Drop trail or background. | `src/ui.rs:350` |
| Sound effects | 13.2 | Met | All six required effects plus Soft and Hard Drop, Hold, each Line Clear size, T-Spins, Back-to-Back, Level Up and blocked moves, synthesized in code. Peter has listened to them. | `src/sound.rs:13` |
| Background music | 2.5.5, 13.3 | Partial | Korobeiniki is embedded in the binary and loops during each game, restarting from the beginning. Volume is fixed at 0.5, with no track choice. | `src/audio.rs:10` |
| Main Menu and game flow | 2, 2.3 | Partial | Opens on a menu with New Game, mode (Marathon, Sprint or Ultra), starting level, goal system, Options, Help and Quit, and Game Over returns to it. High Scores opens the High Score Table. No player count. | `src/menu.rs:135` |
| Options | 2.5 | Met | An Options page sets the Next Queue size, Hold, Ghost Piece, Lock Down mode, music and sound effects, and saves each change to the config file. Mode, starting level and goal system are chosen on the Main Menu and not saved. | `src/menu.rs:101` |
| Help | 2.6 | Met | Three pages, turned with Left and Right: controls, how to play, and scoring. How to play covers the Matrix, Line Clears, Goal and Level Up, Hold, the Ghost Piece, Game Over and the variants, in Guideline terms. Scoring lists every action's points, taken from the scoring code, plus Back-to-Back and drop points. H opens it during play and stops play, and the menu's Help shows the same pages. No credits page, which the Guideline mentions only as common. | `src/ui.rs:631` |
| Variants | 12 | Met | Marathon ends after level 15. Sprint ends at 40 lines and Ultra after three minutes, with no Level Up in either. Both show the time to the hundredth. | `src/config.rs:22` |
| High Score Table, End Rewards, statistics | 11, A1.2.7 | Met | A top 10 per variant, saved to `tetris_scores.json` and opened from the Main Menu. A qualifying game asks for a name in the Game Over box, then shows the table with the new entry marked. Marathon ranks by score, Ultra by score and Sprint by time; quit games and unfinished Sprints do not count. The Game Over box lists Singles, Doubles, Triples, Tetrises, T-Spins, Tetriminos and lines per minute. A new record, a completed Marathon or a place in the table earns a message and fireworks, more for better results. Statistics are not saved in the table. | `src/end_reward.rs` |
| Multiplayer and Line Attacks | B | Missing | None. The Guideline treats this as product-dependent. |  |

The combo bonus of 50 x combo x Level has been removed, because the 2009 scoring table has no combo action.

## Areas of improvement

All 12 listed defects are fixed, along with one new bug found during testing. The main open item is the confidential files in the repo's history.

### Fixed defects

| Defect | Fix | Confirmed by |
| --- | --- | --- |
| Soft Drop jumped almost the whole Matrix when pressed (new, in this pass) | Gravity progress is rescaled when Soft Drop starts or stops | Simulated terminal: 5 rows in 0.27 seconds, then none after release; regression tests |
| Empty Matrix cells drew as solid blocks | Empty cells draw as spaces | tmux capture; render test |
| Z then X panicked with an overflow | Rotation uses modular arithmetic | tmux run; unit test |
| All Tetriminos generated at the top-left | Per-type generation position, centered | tmux capture; unit test |
| Fall Speed never changed with level | Gravity reads the current level each tick | Unit test against the Guideline table |
| A full top row was never cleared | Clearing covers every row, including the Buffer Zone | Unit test |
| The I-Tetrimino preview lost a Mino | The queue display fits four columns | tmux capture; render test for all 7 types |
| Ctrl+C triggered Hold | Modifiers are checked, and Ctrl+C now quits | tmux run; unit test |
| No terminal size check, and side panels were far from the Matrix | Centered fixed layout with a resize message | tmux at 49 x 20 and after resizing; render tests |
| The frame limiter slept on alternating frames | Sleeps for the rest of the current frame | Code reading; simulated terminal timing |
| Any terminal event could dismiss Game Over | Mouse capture removed, and only a key press after 0.5 seconds counts | tmux run for keys; mouse events not sent |
| Music depended on a build-time path | WAV embedded in the binary | Decode test; silent playback test |
| `eprintln!` wrote into the alternate screen | Warnings print after the terminal is restored | Code reading |

### Tests

The test suite now covers the game from rules through to the real terminal, and deliberately breaking the code in 19 ways was caught each time. Some gaps remain:

- **Music:** a silent test confirms the sound device reports paused and resumed and that the music restarts for each game, and CI confirms the theme is embedded and decodes on Linux. Peter has listened to it. The device test skips itself on machines with no sound device, which probably includes the CI runners. The sound effects are tested as rendered samples and game events, and Peter has listened to them.
- **Real terminals:** key-release tests send kitty-protocol key codes through a simulated terminal, and Peter confirmed held keys work in Ghostty. kitty, WezTerm and iTerm2 are untested, but they use the same keyboard protocol.
- **Bot seeds:** the skilled play tests use seeds the bot wins with, because it loses about 6% of unseeded runs. A `rand` upgrade could change the piece order and require new seeds, and a failure then would not necessarily mean a bug.
- **Fork pull requests:** the CI trigger is in place, but no fork pull request has exercised it yet.
- **Timing under load:** the end-to-end tests passed 90 of 90 runs with six copies running at once, after the check most likely behind an earlier failure was removed. CI has only seven runs so far, all passing, so its reliability there is not yet known.

All 28 facing pages for the Super Rotation System have now been read against the code, and the tables are covered by tests.

### Code and project health

- **Repo contents:** the CONFIDENTIAL PDF and `tetris_guidelines.txt` are removed from `main`, and `AGENTS.md` no longer links to the PDF. A local copy can go in `reference/`, which git ignores. Both files stay in the public repo's history. Removing them from it would need a history rewrite and a force push, so I left this for you to decide.

The structural cleanups from the earlier review are done: `Phase`, `Screen`, `Session`, `scoring.rs`, `Facing`, private `GameState` fields, the merged falling helpers, Tetrimino naming, and stable `rand`. Low-value tests were folded into table and property tests, which took the total from 271 to 227 even with new tests added.

## Features to implement

The engine rules and scoring are done, and so are sound effects, Sprint, Ultra, the Options page, the High Score Table, game statistics, End Rewards and Help, so the rest of game flow comes next. Rows are in recommended order within each group. Effort is my relative estimate: S, M or L.

### 1. Terminal input

| Feature | Section | Effort | Notes |
| --- | --- | --- | --- |
| Better fallback for basic terminals | 5.2, 5.5 | M | Terminals without key release reporting still rely on OS key repeat. A timing heuristic could approximate held keys. |

### 2. Decision to confirm

| Feature | Section | Effort | Notes |
| --- | --- | --- | --- |
| Combo bonus | 8 | S | Not in the 2009 scoring table, so recommended for removal to match the Guideline. I have removed it. It can come back if you prefer it. |

### 3. Game flow and modes

| Feature | Section | Effort | Notes |
| --- | --- | --- | --- |
| Starting Blocks | 2.4 | S | Optional rows of blocks at game start. |

### 4. Controls, effects and sound

| Feature | Section | Effort | Notes |
| --- | --- | --- | --- |
| Music and effects controls | 2.5.5, 2.5.6 | S | Music and effects can each be turned off on the Options page. Add volume settings and optional extra tracks. |
| Key remapping, Shift and Ctrl | 4, 4.1 | S | Shift and Ctrl alone need more keyboard enhancement flags. |
| Lock Down flash and Hard Drop trail | 13.1.2, 13.1.3 | S | Possible with terminal colors and modifiers. |

### 5. Optional

| Feature | Section | Effort | Notes |
| --- | --- | --- | --- |
| Multiplayer and Line Attacks | B | L | Local split screen needs a wide terminal, and networking adds more. Do this last, if at all. |
| In-app scoring demonstration | 8 | M | The Guideline requires a demo of all scoring actions. It is a useful tutorial here. |

I left out the legal screen, logo rules and naming rules in sections 2.1, 2.2 and 12, because they are licensing requirements rather than gameplay.
