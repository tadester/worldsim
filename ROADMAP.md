# Roadmap

This project evolves in small “phases” to keep the simulation coherent and incremental.

## Completed

- Phase 0–2: foundation + lifecycle (map, entities, basic growth/repro/death, core sim loop)
- Phase 3: NPC needs, memory, and relationship behavior
- Phase 4: primitive mana practice
- Phase 5: mana experimentation and adaptation
- Phase 6: observation UI and inspector
- Phase 7: primitive shelter construction
- Phase 8: shelter ownership + upkeep
  - NPCs treat shelters as “home”, rest there, and repair them over time.
  - Shelters have integrity that decays and affects comfort/safety.
- Phase 9: NPC foraging economy
  - NPCs carry food/wood, stockpile them at home, and consume food over time.
  - Foraging and wood gathering draw down regional resources.
- Phase 10: factions + territory
  - NPCs align into factions and prefer allied shelters and allies when socializing.
  - Regions accrue faction control based on presence, becoming contested when rivals overlap.
- Phase 11: threats
  - Predators roam the world and hunt nearby animals and settlers.
  - NPCs can flee when predators get too close.
- Phase 12: long-term climate/seasons
  - Seasonal temperature cycle plus slow drift create per-region climate pressure.
  - Resource regrowth scales with climate pressure and temperature.
- Phase 13: population telemetry + ecological balancing
  - Births and deaths are tracked live across the whole simulation, including animal/NPC splits.
  - Animal reproduction now responds to local carrying capacity and forage pressure instead of growing unchecked.
  - NPC family growth is more permissive when homes are safe and stocked.

## Current

- Roadmap complete
  - Continue iterative improvements (simulation depth, UI, performance, and sandboxing).
