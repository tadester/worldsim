# WorldSim

A small Bevy prototype for experimenting with emergent, tile-based world simulation: terrain capacity, animals, NPC needs/decisions, simple magic, and lightweight observation UI.

## Run

```bash
cargo run
```

## Controls

- `Space` pause / unpause
- `1` 1x
- `2` 5x
- `3` 20x
- `4` hard skip
- `Tab` cycle inspected entity

## Notes

- The simulation advances in fixed “steps per frame” controlled by the time-skip keys.
- UI overlays are intentionally minimal: dashboard (top-left), inspector (bottom-left), log panel (right).

