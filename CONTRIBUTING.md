# Contributing to Rengine

## Pull Request Expectations

Every PR that adds or changes engine functionality must include:

1. **Feature sample** — Create or update a sample in `samples/features/feature-<name>/` that demonstrates the new feature in isolation. This should be a minimal, focused demo.

2. **Kitchen-sink integration** — Update the kitchen-sink demo (`samples/games/game-everything/`) to exercise the new feature alongside existing ones. Wire it into the demo flow so automated `--demo --headless` runs hit it.

3. **Roadmap update** — Mark the feature as completed in `ROADMAP.md` with a one-line summary of what was shipped.

4. **Architecture update** — Add or revise the relevant section in `ARCHITECTURE.md` describing the new subsystem, data structures, and public API.

## Code Style

- No comments. No doc comments (`///`), no inline comments, no section dividers. The code should be self-explanatory. `ARCHITECTURE.md`, `ROADMAP.md`, and `CONTRIBUTING.md` are exceptions.
- No conventional commit prefixes (`feat:`, `fix:`, `chore:`, etc). Commit messages should be casual and human-readable.
- Run `cargo fmt` before submitting.
- Run `cargo check` (and `cargo test` if tests exist) before pushing.

## Branch Workflow

1. Create a feature branch off `master`.
2. Develop and commit on the feature branch.
3. Open a PR against `master`.
4. Request a review (Copilot or human).
5. Address review comments, then the maintainer merges manually.

## Project Layout

```
engine/             Main engine crate
  src/
    app.rs          Entry points, game loop, Engine/Engine3D structs
    lib.rs          Public re-exports
    ui.rs           Immediate-mode UI widget system
    canvas/         Canvas overlay rendering, GPU clipping
    input/          Keyboard, mouse, gamepad, action mapping
    math/           Rect, Rng, Time, Timer, Tween, Easing
    renderer/       2D sprite renderer, camera, nine-slice, post-fx
    renderer3d/     3D mesh renderer, camera, viewmodel
    scene/          Scene traits, scene stack, globals, 2D scene data
    world/          Tilemap, AABB collision, triggers, isometric helpers
    assets/         Asset pipeline, audio, spritesheets, color, pixelart
    text.rs         Font atlas, text measurement
    particle.rs     Particle emitter system
    save.rs         Slot-based JSON save/load
samples/
  features/         One directory per engine feature demo
  games/            Larger sample games (platformer, top-down, etc.)
```

## Adding a New Feature

1. Implement the feature in `engine/src/`.
2. Re-export public types from `engine/src/lib.rs`.
3. Create `samples/features/feature-<name>/` with a `Cargo.toml` and `src/main.rs`.
4. Add the sample to the workspace `Cargo.toml` members list.
5. Wire the feature into `samples/games/game-everything/` so it's exercised in demo mode.
6. Update `ROADMAP.md` (mark done, add to progress section).
7. Update `ARCHITECTURE.md` (add subsystem docs).
8. `cargo fmt && cargo check` across the workspace.
