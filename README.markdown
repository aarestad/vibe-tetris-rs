# vibe-tetris-rs
Vibe-coded Tetris CLI implementation. Caveat lector!

## Running

`cargo run` builds and starts the game. The terminal must be at least 50 columns by 24 rows.
Background music is converted at build time and needs `timidity`; without it, the game builds
with no music.

The game opens on the Main Menu, where you can start a game, choose the mode, starting level, goal
system and rows of Starting Blocks, change options, see the high scores, read the controls or quit.

At Game Over, the screen shows your result and statistics: Singles, Doubles, Triples, Tetrises,
T-Spins, Tetriminos placed and lines per minute. A game that makes the top 10 of its mode's High
Score Table asks for your name, then shows the table; otherwise any key returns to the menu. Scores
are saved to `tetris_scores.json`. A new record, a completed Marathon or any place in the table
earns a congratulatory message and fireworks, with more fireworks for better results.

The Options page sets the Next Queue size, Hold, Ghost Piece, Lock Down mode, and the music and
sound effects with their volumes from 0 to 10. Changing the effects volume plays a sample effect.
Each change is saved to `tetris_config.json` right away. The mode, starting level, goal
system and Starting Blocks last only until the program exits.

There are three modes:

- **Marathon**: play for points through 15 levels.
- **Sprint**: clear 40 lines as fast as you can. The level stays at the one you start on, and the
  clock shows hundredths of a second. The goal system does not apply.
- **Ultra**: score as many points as you can in three minutes. The level stays at the one you
  start on, the clock counts down, and the goal system does not apply.

During play, the mode's name is shown at the top of the score panel. Below the Next Queue is the
high score for the mode, or your score once it is higher; in Sprint it is the best time.

The Tetrimino in play is drawn solid and Locked Down Minos are textured. A Tetrimino flashes
white as it Locks Down, and a Hard Drop leaves a fading trail behind it.

## Controls

| Action | Keys |
| --- | --- |
| Move left or right | Left, Right (numpad 4, 6) |
| Soft Drop | Down (numpad 2) |
| Hard Drop | Space (numpad 8) |
| Rotate clockwise | Up, X (numpad 1, 5, 9) |
| Rotate counterclockwise | Z (numpad 3, 7) |
| Hold | C (numpad 0) |
| Pause | Esc, F1, P |
| Help | H |
| Quit | Q, Ctrl+C |

In the menu, use the arrow keys, Enter or Space to choose, Esc to go back, and Q or Ctrl+C to quit.

Help has three pages: the controls, how to play, and the scoring table. Left and Right turn the
pages, both from the menu and during a game, where play stops until H closes Help.

Numpad keys are read as digits, so the number row does the same thing.

Auto-Repeat and 20x Soft Drop need a terminal that reports key releases, such as kitty, WezTerm,
foot, Ghostty or iTerm2. In other terminals, holding a key uses your system's key repeat.

## Configuration

Settings are read from `tetris_config.json` in the current directory. The file is optional, and any
field left out takes its default. An invalid file stops the game with an error. Changing an option
in the game rewrites the file with every setting filled in.

```json
{
  "board_width": 10,
  "board_height": 20,
  "game_mode": "marathon",
  "starting_level": 1,
  "starting_blocks": 0,
  "lines_per_level": 10,
  "enable_ghost_piece": true,
  "enable_hold": true,
  "enable_variable_goal": false,
  "enable_sound": true,
  "enable_music": true,
  "enable_sound_effects": true,
  "music_volume": 5,
  "sound_effects_volume": 4,
  "preview_count": 3,
  "das_delay": 300,
  "das_repeat": 50,
  "lock_down_mode": "extended"
}
```

`enable_sound` turns all sound on or off. When it is on, `enable_music` and
`enable_sound_effects` control the background music and the sound effects separately. `music_volume`
and `sound_effects_volume` go from 0, silent, to 10, full volume.

`game_mode` can be `marathon`, `sprint` or `ultra`.

`starting_blocks` fills that many rows at the bottom of the Matrix with random Minos when a game
begins, from 0 to 10 and at most half of `board_height`. Every row has at least one gap.

`lock_down_mode` can be `extended`, `infinite` or `classic`.
