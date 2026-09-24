# rengine — agent instructions

## Bounded decisions: call Jev before expensive work

Before substantial browser research, repeating a failed approach, loading several
tools or skills, spawning agents, choosing between materially different execution
routes, or proposing a consequential action, consider whether a small bounded Jev
decision would change the next step. If yes, build a compact state without secrets,
call the installed router, interpret its action, and continue the original task.
Skip Jev for simple answers, deterministic calculations, routine file edits, and
situations where the call adds no useful decision. Respect `bypass Jev`. Keep
irreversible actions behind human confirmation.

### The invocation

Windows / bash (this repo's shell):

```bash
~/dev/jev/.venv/Scripts/python.exe ~/dev/jev/jev_router.py \
    -p next_step -s '<compact state as JSON or literal text>'
```

To check the install, or to see what is not arriving, run these instead:

```bash
~/dev/jev/.venv/Scripts/python.exe ~/dev/jev/jev_router.py --dry-run   # offline: config + payload
~/dev/jev/.venv/Scripts/python.exe ~/dev/jev/jev_router.py --check     # one live call
~/dev/jev/.venv/Scripts/python.exe ~/dev/jev/jev_router.py --stats     # summarise the log
```

Reads `~/dev/jev/config.yaml`, writes `~/dev/jev/decisions.jsonl`, prints a JSON
object whose `action` field is the routing result. Exit `0` ok, `1` service or
verification failure, `2` bad config/usage.

Rules of use:

* **`shadow` is the default and currently the only sanctioned mode.** In shadow the
  action is advisory (`"authoritative": false`); you keep authority and decide.
  Do not pass `--active` without the user asking for it.
* **Read the log before trusting the router.** `--stats` summarises
  `decisions.jsonl` (real vs simulated, unique states, consistency). Fewer than
  five unique states is not enough to judge anything about disagreement.
* **A `score` question's answer is the expected level index, not a 0-1 score.**
  With three levels it returns 0-2, and a real call gives about `1.9` for the top
  level -- never `2`. A threshold sitting at the top of a range is a rule that
  never fires. `--dry-run` now fails on any threshold its question cannot reach,
  so run it after editing `config.yaml`.
* **State must be compact and secret-free.** Send the goal, the approach you are
  about to repeat, and the evidence you actually gathered — not file dumps, not
  credentials. The log stores only a sha256 fingerprint of the state, but the state
  itself does go to the TypeSafe API.
* **Do not block on it.** If no key is set the router exits `2` with setup
  instructions; carry on with the task instead of asking for the key.
* **The router is advisory, not authoritative.** A `change_approach` is a prompt to
  reconsider, not a mandate. If you disagree, say so and keep going.
* Requires `OPENROUTER_API_KEY` (or `TYPESAFE_API_KEY`) in the shell environment.
  Jev runs through OpenRouter as `typesafe/jev-1.13`, at roughly $0.000012 a call
  (output tokens are free), so cost is never a reason to skip it. The key is never
  committed and never pasted into chat.

### Why this repo

`rengine` is the engine crate; `../game-motorsport-prototype` (`formula-r`) consumes
it as a path dependency. Both are Rust workspaces where the expensive mistakes look
alike: a long `cargo test` sweep, a re-blessed golden, a multi-hour refactor slice
aimed at the wrong layer. Those are the calls worth gating.

### When `active` becomes justified

Start in `shadow`, then read `decisions.jsonl`. Move to `active` only when the log
shows the router disagreeing with the choice you would have made *rarely*, and being
right when it does. Until then, treat every action as a suggestion.

## The TypeSafe skill

The official TypeSafe skill (which teaches how to design the questions, thresholds,
and patterns) is installed at `~/.agents/skills/typesafe-ai/`. Use it when changing
the policies in `~/dev/jev/config.yaml` — the question wording and thresholds are the
only parts that carry real judgement, which is why they live in one reviewable file.

Verified sources: <https://docs.typesafe.ai/introduction/quickstart>,
<https://github.com/typesafe-ai/skills>.

## Repo conventions

* This is a Rust workspace (`engine`, `editor`, `samples/*`); `rust-toolchain.toml`
  pins the toolchain. Build with `cargo`, not by hand.
* See `CONTRIBUTING.md`, `ARCHITECTURE.md`, and the three `ROADMAP*.md` files before
  making structural changes.
* Prefer small, verified slices with a test that fails before and passes after.