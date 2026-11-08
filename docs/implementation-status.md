# tetris-rs: Implementation Status vs. 2009 Tetris Design Guideline

Nov 6, 2026, updated Nov 7, 2026 · @Peter Aarestad

## Summary

tetris-rs now plays as Guideline Tetris: of 38 Guideline areas, 34 are met, 3 partial and 0 wrong. The last, multiplayer, is left out by your decision. Before this pass the counts were 7 met, 13 partial, 6 wrong and 11 missing. All 12 listed defects are fixed, and the core engine and scoring rules are in place.

What remains:

- **Game flow:** the High Score Table, End Rewards, game statistics, a three-page Help, the high score during play and Starting Blocks are in. The Main Menu has no player count, because the game stays single player.
- **Sound:** all six required sound effects and most recommended ones now play, and Peter has confirmed they sound good. Music and effects each have an on/off switch and a volume from 0 to 10 on the Options page.
- **Basic terminals:** Auto-Repeat and 20x Soft Drop need a terminal that reports key releases. Elsewhere, held moves and Soft Drop follow the system's key repeat, whose delay and rate the game cannot see or change. Rotation, Hard Drop, Hold, Pause and Help ignore the steady system repeats, so holding one acts at most twice.

One risk still needs your decision: the Guideline PDF, which is marked CONFIDENTIAL, and its extracted text are removed from `main`. They remain in the public GitHub repo's history.

This report describes `main` at commit `8441b99`, which records your decisions to keep the combo bonus out and not add multiplayer. It passes lint and all 448 tests locally and in CI on Linux and macOS. Peter has listened to Korobeiniki, though not yet to the two new tracks, and has confirmed that held keys work in Ghostty, and that the sound effects sound good.

## Current state

The code is a 14,252-line, single-player Rust terminal game on `main`, built on ratatui, crossterm and rodio. The program opens on a Main Menu, and each game runs in a `Session` with no terminal or sound device, so `Game` only does I/O. Game flow runs through a `Phase` enum, pause and help share a `Screen` enum, and scoring, sound effects, high scores and End Rewards have their own modules.

| Module | Lines (with tests) | Tests | Responsibility |
| --- | --- | --- | --- |
| `src/game_state.rs` | 2,052 | 90 | Phases, generation, gravity, Lock Down, Hold, T-Spin recognition, seedable bag, Starting Blocks, sound and visual effect events |
| `src/ui.rs` | 2,636 | 61 | ratatui rendering: menu, Options, Controls and High Scores pages, Help pages with the bound keys, Matrix with textured locked Minos, Lock Down flash, Hard Drop trail and level-colored walls, the scoring demonstration, queues, high score field, progress bar, overlays, Game Over statistics, name entry, fireworks, too-small message |
| `tests/end_to_end.rs` | 1,194 | 43 | Runs the real binary in a pseudo-terminal and reads its screen |
| `src/menu.rs` | 1,266 | 41 | Main Menu choices, the Options page, the Controls page with its key capture, and the High Scores page, with no terminal |
| `src/scoring.rs` | 660 | 28 | Score, Back-to-Back, level, Goal, the Sprint line count, Game Statistics and the scoring table for Help |
| `src/session.rs` | 677 | 31 | Input and time handling for one game, with no terminal or sound, including the repeat guard |
| `src/input.rs` | 816 | 26 | Play keys through the bindings, Shift and Ctrl alone, menu, key capture and name entry keys, key press and release events, Auto-Repeat, and guessing system key repeat |
| `src/high_scores.rs` | 516 | 13 | High Score Table per variant, ranking, saving and loading, name entry |
| `src/board.rs` | 494 | 38 | Matrix and Buffer Zone storage, collision, Line Clear |
| `src/tetrimino.rs` | 365 | 11 | Facings, Tetrimino shapes, generation position, Super Rotation tables |
| `src/game.rs` | 444 | 0 | Connects the menu and each game to the real terminal and sound device, reports Shift and Ctrl during play and while the Controls page waits for a key, runs the scoring demonstration, sets the volumes, and saves options, controls and high scores |
| `tests/skilled_play.rs` | 266 | 4 | A bot plays seeded Marathons, Sprints and Ultras through real key events |
| `src/config.rs` | 416 | 13 | JSON config with defaults, validation and saving, including the game mode, Starting Blocks, volumes and controls |
| `src/controls.rs` | 485 | 10 | Key bindings for each play action, key names for the config file, their checks, and adding, moving and removing keys |
| `src/audio.rs` | 348 | 11 | Embedded music tracks, Random Play's choice of song, and sound effects via rodio |
| `src/sound.rs` | 224 | 7 | Sound effects as synthesized notes, rendered to samples |
| `tests/random_play.rs` | 112 | 1 | Seeded random key events in all game, Lock Down, goal and input modes, with 0 to 10 rows of Starting Blocks |
| `src/visual_effects.rs` | 261 | 5 | Lock Down flash, Hard Drop trail and wall highlight timing and cells |
| `src/demo.rs` | 522 | 6 | The scoring demonstration: 13 scripted scenes on the real engine, one per point-scoring action |
| `src/end_reward.rs` | 95 | 2 | Which games earn an End Reward, its message and how many fireworks |
| `src/screen.rs` | 143 | 7 | Playing, Paused and Help screens, the Help pages and the keys that switch them |
| `build.rs` | 215 | 0 | Writes the Minuet and Ode to Joy as MIDI from note lists, and converts them and the Korobeiniki MIDI to WAV with `timidity` |

- **Tests:** `cargo test` passes all 448 tests: 400 in the library, 43 end-to-end, 1 random play and 4 skilled play. The end-to-end tests take about 8 seconds, run only on Unix, and always turn sound off. They cover 98.84% of lines; see Test coverage below.
- **Lint:** `cargo fmt --check` and `cargo clippy --all-targets` are both clean, with 0 warnings.
- **CI:** `.github/workflows/ci.yml` runs on every push and on pull requests from forks. It checks formatting and clippy on Linux, with warnings as errors, and runs all tests on Linux and macOS. The Linux test job also requires the music to be embedded. Its latest run, for `8441b99`, passed lint and all 448 tests on both.
- **Dependencies:** `rand` is on stable `0.10.3`. `portable-pty` and `vt100` are for tests only, so the shipped game gains nothing from them.
- **Docs:** the README covers the menu, its three modes, the Options page, the Game Over statistics, the High Score Table, End Rewards and Help, running, controls and configuration, including the sound and mode settings, and `AGENTS.md` matches the code.

## Guideline compliance

34 of the 38 areas below fully meet the Guideline, 3 are partial, none are wrong, and multiplayer is left out by your decision. Rows follow Guideline order. Section numbers refer to the 2009 Tetris Design Guideline. For rotation, I compared the code against all 28 facing pages, for the I, T, L, J, S and Z.

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
| Auto-Repeat | 5.2 | Partial | With key release reporting: 0.3 second delay, then 50 ms per move, with opposite-key handling. Otherwise moves follow the system's key repeat, and presses of a one-shot key less than 120 ms apart are taken as system repeats and ignored, so a held rotation or Hard Drop key acts at most twice. | `src/input.rs:44` |
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
| Scoring demonstration | 8 | Met | Scoring Demo on the Main Menu plays every action in the scoring table, plus Back-to-Back, Soft Drop and Hard Drop, as scripted scenes on the real engine. Each scene shows the action's name, description and points, then what it scored, with drop points apart. Left and Right change scenes. Tests check that every scene scores what the table says. | `src/demo.rs:47` |
| T-Spin recognition | 9 | Met | Checks last move, corner cells by Sides A to D, and Point 5. | `src/game_state.rs:609` |
| T-Spin scoring | 8 | Met | Table totals for all six T-Spin actions, including no-clear T-Spins. | `src/scoring.rs:174` |
| Back-to-Back | 8.1 | Met | The 0.5 x bonus and sequence rules reproduce the Guideline's 5,400-point example. | `src/scoring.rs:72` |
| Game Over conditions | 10 | Met | Block Out and Lock Out work. Top Out can only come from Line Attacks, which are not in single player. | `src/game_state.rs:535` |
| Game information display | 2.4, 7 | Met | Score, level, lines, Goal and time, with hundredths in Sprint and a countdown in Ultra. The mode name is shown in the panel's border. A box under the Next Queue shows the variant's high score, or the current score once it is higher, and in Sprint the best time. It gives way if a smaller Matrix leaves no room. Player name and rank, also listed as possible fields, are left out. | `src/ui.rs:396` |
| Pause | 13.1.6 | Met | Stops play and time, and hides the Matrix and both queues. | `src/ui.rs:548` |
| Controls | 4, 4.1 | Met | Every key in the Guideline's keyboard table works, with numpad keys read as digits. Shift holds and Ctrl rotates counterclockwise on terminals with keyboard enhancement, where play turns on reporting every key; elsewhere C and Z do the same. Every key can be remapped on the Options page's Controls page, which adds the next key pressed to an action, moves a key from another action unless it is that action's only key besides Shift and Ctrl, removes keys and resets to the defaults, saving each change. The config file's `controls` section does the same, and rejects a key bound twice and an action with only Shift or Ctrl. Esc cancels on the Controls page, so only the config file can bind it. While Ctrl is a play key, Ctrl+C during play acts as C, and Q still quits. Help lists the keys as bound. | `src/controls.rs:134` |
| Visual effects | 13.1 | Met | Line Clear effects scale with count, and action notifications show outside the Matrix. A Tetrimino flashes white when it Locks Down, a Hard Drop leaves a dim trail that shrinks toward where it landed, and locked Minos are drawn textured (▓) so they stand apart from the solid Tetrimino in play. The Matrix walls are the background graphics: their color shows the level, they flash in the action's color for a Tetris, T-Spin or Back-to-Back, and they run through the level colors for a Level Up. | `src/visual_effects.rs:1` |
| Sound effects | 13.2 | Met | All six required effects plus Soft and Hard Drop, Hold, each Line Clear size, T-Spins, Back-to-Back, Level Up and blocked moves, synthesized in code. Peter has listened to them. | `src/sound.rs:13` |
| Background music | 2.5.5, 13.3 | Met | Korobeiniki is the default track. Petzold's Minuet in G and Beethoven's "Ode to Joy", public domain melodies arranged in `build.rs`, are the extra tracks, and Random Play picks a different one for each game. The music loops during each game, restarting from the beginning, and can be turned off and its volume set from 0 to 10 on the Options page. | `src/audio.rs:10` |
| Main Menu and game flow | 2, 2.3 | Partial | Opens on a menu with New Game, mode (Marathon, Sprint or Ultra), starting level, goal system, Starting Blocks (0 to 10 rows of random Minos, each row with a gap), Options, Help, Scoring Demo and Quit, and Game Over returns to it. High Scores opens the High Score Table. No player count, because the game stays single player. | `src/menu.rs:235` |
| Options | 2.5 | Met | An Options page sets the Next Queue size, Hold, Ghost Piece, Lock Down mode, music and sound effects, the music track and their volumes, and the play keys on its Controls page, and saves each change to the config file. Changing the effects volume plays a sample effect. Mode, starting level, goal system and Starting Blocks are chosen on the Main Menu and not saved. | `src/menu.rs:190` |
| Help | 2.6 | Met | Three pages, turned with Left and Right: controls, how to play, and scoring. How to play covers the Matrix, Line Clears, Goal and Level Up, Hold, the Ghost Piece, Game Over and the variants, in Guideline terms. Scoring lists every action's points, taken from the scoring code, plus Back-to-Back and drop points. H opens it during play and stops play, and the menu's Help shows the same pages. No credits page, which the Guideline mentions only as common. | `src/ui.rs:631` |
| Variants | 12 | Met | Marathon ends after level 15. Sprint ends at 40 lines and Ultra after three minutes, with no Level Up in either. Both show the time to the hundredth. | `src/config.rs:22` |
| High Score Table, End Rewards, statistics | 11, A1.2.7 | Met | A top 10 per variant, saved to `tetris_scores.json` and opened from the Main Menu. A qualifying game asks for a name in the Game Over box, then shows the table with the new entry marked. Marathon ranks by score, Ultra by score and Sprint by time; quit games and unfinished Sprints do not count. The Game Over box lists Singles, Doubles, Triples, Tetrises, T-Spins, Tetriminos and lines per minute. A new record, a completed Marathon or a place in the table earns a message and fireworks, more for better results. Statistics are not saved in the table. | `src/end_reward.rs` |
| Multiplayer and Line Attacks | B | Not planned | Left out by your decision. The Guideline treats multiplayer as product-dependent. |  |

The combo bonus of 50 x combo x Level has been removed, because the 2009 scoring table has no combo action. You decided not to bring it back.

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

- **Music:** a silent test confirms the sound device reports paused and resumed and that the music restarts for each game, and CI confirms every music track is embedded and decodes on Linux, and a test checks each extra track lasts at least 20 seconds. Peter has listened to Korobeiniki, but not yet to the Minuet or Ode to Joy. The device test skips itself on machines with no sound device, which probably includes the CI runners. The sound effects are tested as rendered samples and game events, and Peter has listened to them.
- **Real terminals:** key-release tests send kitty-protocol key codes through a simulated terminal, and Peter confirmed held keys work in Ghostty. kitty, WezTerm and iTerm2 are untested, but they use the same keyboard protocol.
- **Bot seeds:** the skilled play tests use seeds the bot wins with, because it loses about 6% of unseeded runs. A `rand` upgrade could change the piece order and require new seeds, and a failure then would not necessarily mean a bug.
- **Fork pull requests:** the CI trigger is in place, but no fork pull request has exercised it yet.
- **Timing under load:** the end-to-end tests passed 90 of 90 runs with six copies running at once, after the check most likely behind an earlier failure was removed. CI has only seven runs so far, all passing, so its reliability there is not yet known.

All 28 facing pages for the Super Rotation System have now been read against the code, and the tables are covered by tests.

### Test coverage

I measured coverage with `cargo-llvm-cov` 0.6.23 on macOS, running all 448 tests with `REQUIRE_EMBEDDED_THEME=1 cargo llvm-cov`. It includes the game binary that the end-to-end tests run.

| Measure | Covered | Total | Coverage |
| --- | --- | --- | --- |
| Lines | 8,403 | 8,502 | 98.84% |
| Regions | 14,698 | 14,884 | 98.75% |
| Functions | 980 | 990 | 98.99% |

The figures include the test modules inside each source file, which run in full, so the game code alone is slightly lower. Branch coverage needs a nightly compiler and was not measured. `llvm-cov` warns that 1 function has mismatched data between test binaries, which does not affect these totals.

Every line of `board.rs`, `scoring.rs`, `screen.rs`, `sound.rs`, `tetrimino.rs` and `visual_effects.rs` runs. The rest:

| File | Lines | Missed | Coverage | What is missed |
| --- | --- | --- | --- | --- |
| `src/main.rs` | 17 | 3 | 82.35% | Printing warnings at exit, and a terminal that fails to start |
| `src/audio.rs` | 228 | 23 | 89.91% | Opening a real sound device, and its failures. The end-to-end tests turn sound off, and the device test skips itself without a device |
| `src/game.rs` | 271 | 15 | 94.46% | Failed saves of options, controls and high scores, music when no track is embedded, and terminal errors while switching keyboard modes |
| `src/controls.rs` | 323 | 13 | 95.98% | Some arms of the per-action matches for config error names and for changing keys, and the Enter and Tab labels |
| `src/end_reward.rs` | 53 | 2 | 96.23% | A test's failure message |
| `src/input.rs` | 548 | 12 | 97.81% | Terminal read errors, and events other than keys, such as resizes, while polling |
| `src/config.rs` | 236 | 3 | 98.73% | Error messages for a config file that cannot be read or written |
| `src/ui.rs` | 2,102 | 19 | 99.10% | A few name and color arms (Infinite, Minuet, one level color, the Back-to-Back highlight), part of the scoring demonstration's scored line, and a test helper |
| `src/demo.rs` | 233 | 2 | 99.14% | Up and Down, which do nothing, and a test's failure message |
| `src/high_scores.rs` | 335 | 2 | 99.40% | Error messages for a scores file that cannot be read or written |
| `src/menu.rs` | 790 | 2 | 99.75% | Backspace on the Options page, which does nothing, and a key arriving when no action is selected, which the page never allows |
| `src/session.rs` | 479 | 1 | 99.79% | Pause, Help and Quit releases, which do nothing |
| `src/game_state.rs` | 1,489 | 2 | 99.87% | Ultra completing on a Line Clear, which cannot happen since Ultra ends on time, and an empty Game Over arm |

The gaps are mostly error handling that needs a failing disk, terminal or sound device, and match arms that only change a label. None is game logic. Two gaps this report found are closed: an unused `held_piece()` accessor is removed, and end-to-end tests now check that Ctrl+C during name entry and during the scoring demonstration exits the program, without saving the score from name entry.

Before this report, each end-to-end test killed the game when it finished. A killed process never writes its coverage data, so only tests that quit cleanly counted, and `game.rs` showed 73% and the total 97.77%. The tests now quit with Ctrl+C first and kill the game only if it has not exited within 2 seconds, which adds about a second to the end-to-end run.

### Code and project health

- **Repo contents:** the CONFIDENTIAL PDF and `tetris_guidelines.txt` are removed from `main`, and `AGENTS.md` no longer links to the PDF. A local copy can go in `reference/`, which git ignores. Both files stay in the public repo's history. Removing them from it would need a history rewrite and a force push, so I left this for you to decide.

The structural cleanups from the earlier review are done: `Phase`, `Screen`, `Session`, `scoring.rs`, `Facing`, private `GameState` fields, the merged falling helpers, Tetrimino naming, and stable `rand`. Low-value tests were folded into table and property tests, which took the total from 271 to 227 even with new tests added.

## Features to implement

The engine rules and scoring are done, and so are sound effects, Sprint, Ultra, the Options page, the High Score Table, game statistics, End Rewards, Help, Starting Blocks, the music and effects volumes, the Lock Down flash, the Hard Drop trail, the wall highlights, Shift and Ctrl, key remapping in the config file and on the Controls page, the repeat guard for basic terminals, the scoring demonstration, and the extra music tracks. Nothing is left to implement. You decided to keep the combo bonus out, since the 2009 scoring table has none, and not to add multiplayer, so the game stays single player.

I left out the legal screen, logo rules and naming rules in sections 2.1, 2.2 and 12, because they are licensing requirements rather than gameplay.
