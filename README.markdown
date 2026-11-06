# vibe-tetris-rs
Vibe-coded Tetris CLI implementation. Caveat lector!

## Running

`cargo run` builds and starts the game. The terminal must be at least 50 columns by 24 rows.
Background music is converted at build time and needs `timidity`; without it, the game builds
with no music.

The game opens on the Main Menu, where you can start a game, choose the mode, starting level and
goal system, change options, read the controls or quit. After Game Over, any key returns to the
menu.

The Options page sets the Next Queue size, Hold, Ghost Piece, Lock Down mode, music and sound
effects. Each change is saved to `tetris_config.json` right away. The mode, starting level and goal
system last only until the program exits.

There are three modes:

- **Marathon**: play for points through 15 levels.
- **Sprint**: clear 40 lines as fast as you can. The level stays at the one you start on, and the
  clock shows hundredths of a second. The goal system does not apply.
- **Ultra**: score as many points as you can in three minutes. The level stays at the one you
  start on, the clock counts down, and the goal system does not apply.

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
  "lines_per_level": 10,
  "enable_ghost_piece": true,
  "enable_hold": true,
  "enable_variable_goal": false,
  "enable_sound": true,
  "enable_music": true,
  "enable_sound_effects": true,
  "preview_count": 3,
  "das_delay": 300,
  "das_repeat": 50,
  "lock_down_mode": "extended"
}
```

`enable_sound` turns all sound on or off. When it is on, `enable_music` and
`enable_sound_effects` control the background music and the sound effects separately.

`game_mode` can be `marathon`, `sprint` or `ultra`.

`lock_down_mode` can be `extended`, `infinite` or `classic`.
