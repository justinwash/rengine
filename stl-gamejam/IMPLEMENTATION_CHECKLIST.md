# St. Louis Armory Defense Checklist

This document locks the current scope for the jam build and tracks what is already done versus what still needs implementation.

## Locked Premise

- [x] The game is a St. Louis-themed tower-defense-style sabotage game.
- [x] The setting is The Armory in St. Louis, treated as a venue under threat of conversion into an AI datacenter.
- [x] The enemies are contractors and tech-install crews trying to lay infrastructure, not monsters.
- [x] The player side uses improvised barricades, traps, and sabotage gear instead of military weapons.
- [x] The shell flow is main menu -> game -> thanks screen.

## MVP Requirements

- [x] One playable Armory defense map.
- [x] Three contractor entry routes feeding one install objective.
- [x] A Conversion meter that represents datacenter buildout progress.
- [x] Build phase and wave phase loop.
- [x] Scrap currency earned from stopping crews.
- [x] Fixed build pads instead of freeform placement.
- [x] One barricade build: Event Fence.
- [x] One trap build: Breaker Pop Box.
- [x] One core enemy type: Contractor Crew.
- [x] Multiple short waves with increasing pressure.
- [x] Victory and defeat states that route back into the existing thanks screen flow.

## Next Gameplay Expansion

- [ ] Add more contractor archetypes: Fiber Spool Crew, Breaker Electrician, Rack Hauler, Drone Mapper.
- [ ] Add a player-controlled rebel avatar that can run the floor during waves.
- [ ] Add repair interaction for damaged barricades.
- [ ] Add more sabotage builds: Cable Snare, Signal Jammer, Server Cage Blocker.
- [ ] Add objective-side install behavior beyond the single conversion node.
- [ ] Add wave intro banners and better event messaging.
- [x] Add better result stats on the thanks screen.

## Polish And Tuning

- [ ] Replace placeholder shell copy with final game copy.
- [ ] Improve map art so the floor reads more clearly as The Armory.
- [ ] Tune scrap costs, enemy HP, wave counts, and conversion pacing.
- [ ] Add audio and feedback for trap triggers, barricade damage, and conversion spikes.
- [ ] Add defeat-specific thanks or restart messaging if needed.