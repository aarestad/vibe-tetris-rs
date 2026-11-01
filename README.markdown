# vibe-tetris-rs
Vibe-coded Tetris CLI implementation. Caveat lector!

## Running

`cargo run` builds and starts the game. The terminal must be at least 50 columns by 24 rows.
Background music is converted at build time and needs `timidity`; without it, the game builds
with no music.

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

Numpad keys are read as digits, so the number row does the same thing.

Auto-Repeat and 20x Soft Drop need a terminal that reports key releases, such as kitty, WezTerm,
foot, Ghostty or iTerm2. In other terminals, holding a key uses your system's key repeat.

## Configuration

Settings are read from `tetris_config.json` in the current directory. The file is optional, and any
field left out takes its default. An invalid file stops the game with an error.

```json
{
  "board_width": 10,
  "board_height": 20,
  "starting_level": 1,
  "lines_per_level": 10,
  "enable_ghost_piece": true,
  "enable_hold": true,
  "enable_variable_goal": false,
  "enable_sound": true,
  "preview_count": 3,
  "das_delay": 300,
  "das_repeat": 50,
  "lock_down_mode": "extended"
}
```

`lock_down_mode` can be `extended`, `infinite` or `classic`.
