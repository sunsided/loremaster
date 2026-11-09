# Contributing

loremaster uses GitHub Flow. There is one long-lived branch, `main`. Each change goes to `main` through a pull request from a feature branch.

## Development

### Crates and folders

| Path | Contents |
| --- | --- |
| `crates/loremaster-model` | World types: rooms, exits, things, fragments, rules, clock and verbs. |
| `crates/loremaster-lang` | The condition language and the inline markup. They compile to dramaturge conditions. |
| `crates/loremaster-project` | The project folder format (one YAML file for each entity) and the checker. |
| `crates/loremaster-build` | A helper for `build.rs`: it generates the host types (actions, events) of a project's `interface:` as Rust code. |
| `crates/loremaster-runtime` | The runtime: state, text assembly, rules and parser. It has no input or output of its own. Hosts play through its `Session`. |
| `crates/loremaster-workbench` | The authoring views over a running session: Atlas, map, rules, machines, symbols, markup and condition checks, previews, room matrix and explorer. `reload` carries the game over to a changed world. |
| `crates/loremaster-dsl` | The builder API in plain Rust: write a story as a pack, and join it with YAML folders (`Loremaster::new().pack(..).yaml(..).build()`). The crate `loremaster` gives it with the cargo feature `dsl`. |
| `crates/loremaster-macros` | The `world!` macro: a story pack in a grammar close to the YAML files. It expands to calls of the builder API, and runs the schema checks at compile time. The crate `loremaster` gives it with the cargo feature `dsl`. |
| `crates/loremaster-wasm` | The WebAssembly bindings for the web app. Trunk builds them. |
| `crates/loremaster-cli` | The crate `loremaster`, with the terminal player: `loremaster play` and `loremaster check`. Also `loremaster new`, `loremaster mcp` and `loremaster serve`. |
| `crates/loremaster-tools` | The operations that an AI assistant can do on a project: read, play and verify. It has no input or output of its own. |
| `crates/loremaster-mcp` | The MCP server and the embedded web editor (`ui/`, made by `task release:ui`) for one project: `loremaster mcp <project-folder>` on stdin and stdout, and `loremaster serve <project-folder>` on local HTTP with the proposal and job endpoints for the editor. A job gives a task prompt (`imagine_exit`, `review_room`) to Claude Code. |
| `crates/loremaster-zil` | The importer for ZIL games: `loremaster-zil <zil-folder> <project-folder>`. |
| `web/` | The SvelteKit app and the design system with its Storybook. |

### Crates

These crates are on crates.io: `loremaster`, `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-build`, `loremaster-runtime`, `loremaster-workbench`, `loremaster-tools`, `loremaster-dsl`, `loremaster-macros` and `loremaster-mcp`. All have the same version. `loremaster-wasm` and `loremaster-zil` are not on crates.io. The changes are in [CHANGELOG.md](CHANGELOG.md).

The crate `loremaster` builds with the web editor by default. In a git checkout, the editor must be in `crates/loremaster-mcp/ui` first: run `task release:ui`. To build without the editor, use `--no-default-features`.

### How loremaster uses dramaturge

loremaster uses [dramaturge](https://github.com/sunsided/dramaturge) 0.3 (`dramaturge-model`, `dramaturge-runtime`, `dramaturge-authoring` and `dramaturge-syntax`) for conditions, dialogues and the selection of text. loremaster keeps all state: its `State` holds the facts, relations and memories once, and dramaturge reads and writes them through its `NarrativeState` trait (`crates/loremaster-runtime/src/game/narrative.rs`).

| Part | dramaturge | loremaster |
| --- | --- | --- |
| Conditions | Each `when:` compiles to a dramaturge `Condition` tree | `loremaster-lang` parses the condition language and compiles it. dramaturge's `DefaultConditionEvaluator` evaluates the tree at run time, with a custom handler from `crates/loremaster-runtime/src/eval.rs`. Derived values (time of day, room, visits, carried things, the action) are one `Condition::Custom` with the key `loremaster.derived` |
| Stored state | Reads and writes through `NarrativeState` | `State.facts`, `State.relations` and `State.memories`, the only copy, also in the save |
| Dialogues | The dialogue AST of `dramaturge-syntax` and the runtime that selects answers and applies outcomes. `dramaturge-authoring` examines each graph in `loremaster check` | `dialogues/*.yaml` and `world!` read into the AST. The text stays loremaster markup |
| Text pools | The selection core: pools, `select`, `commit`, the trace | Markup, layers and room assembly on top (`crates/loremaster-runtime/src/text.rs`) |
| Rules, verbs, parser, map, clock, machines, combat | Not used | loremaster's own model and runtime |

The fit reports in `docs/` (`dramaturge-dialogue-fit.md`, `dramaturge-lock-keeper-fit.md`) give the details.

### Commands

Install [go-task](https://taskfile.dev), Trunk and Node.js before you start, and run `corepack enable` once. Corepack gives the Yarn version that `web/package.json` pins.

| Command | What it does |
| --- | --- |
| `task play` | Plays the example in the terminal. |
| `task import:zork` | Imports Zork I again from its ZIL source into `examples/zork1`. |
| `task web:install` | Installs the web dependencies. Do this one time. |
| `task dev` | Builds the WebAssembly package, then runs `trunk watch` and the Vite dev server. |
| `task build:static` | Builds the browser-only site for GitHub Pages into `web/build`, below `/loremaster`. |
| `task release:ui` | Builds the web app below `/` and copies it into `crates/loremaster-mcp/ui` for the embedded editor. |
| `task build:local` | Builds `target/release/loremaster` with the web editor in it. |
| `task serve:local` | Builds the local binary, then opens the example in the editor. |
| `task storybook` | Starts Storybook on port 6006. |

The commands that check a change are in "Checks" below.

If you do not use go-task:

1. Play in the terminal with `cargo run -p loremaster --no-default-features -- play examples/lighthouse`.
2. For the web app, run `trunk watch` in `crates/loremaster-wasm`.
3. At the same time, run `yarn dev` in `web/`.

### How Trunk and SvelteKit work together

Trunk builds the Rust part. The file `crates/loremaster-wasm/Trunk.toml` tells Trunk to build the crate with wasm-bindgen and TypeScript declarations. Trunk writes the output to `web/src/lib/engine/pkg/` without content hashes. `trunk watch` builds again when a crate changes.

SvelteKit and Vite make the user interface. They import the package as a normal module. Thus a change in Rust gets to the browser as a normal Vite reload.

### Design system

The web app uses design tokens and atomic design. Storybook shows and documents each element. Start with the "Introduction" page in Storybook.

These rules apply to all web work:

- Use only design tokens for colour, type, space, size and motion.
- Make each element accessible from the start. The target is WCAG 2.2 AA.
- Make each element work at all widths from 320px.
- Write a story with documentation for each element.

## Branches

- `main` must always build and pass all checks. Do not push to `main` directly. The only exception is the release commit (see "Releases").
- Make one branch for each change. Start the branch from the newest `main`.
- Name the branch `feature/<short-name>`. Use lowercase words and hyphens, for example `feature/room-matrix-export`.
- Keep a branch short-lived. Merge it soon, then delete it.

## Steps

1. Update `main`:

   ```sh
   git switch main
   git pull
   ```

2. Make the branch:

   ```sh
   git switch -c feature/<short-name>
   ```

3. Make the change. Commit in small steps.
4. Run the checks (see below).
5. Push the branch and open a pull request against `main`.
6. When `main` changes before the merge, merge `main` into the branch. Do not rewrite a branch that other persons use.
7. Merge the pull request when the checks pass and the review is complete. Then delete the branch.

## Checks

Run these commands before you open a pull request:

| Command | What it does |
| --- | --- |
| `task verify` | Examines the format, runs clippy and runs the Rust tests. |
| `task web:check` | Type-checks the Svelte code and the stories. Run it when you change `web/`. |
| `task package` | Builds the web editor and packages the crates for crates.io. Run it when you change a `Cargo.toml`. |
| `task test:pending` | Runs the pending tests. A pending test that passes is an error. Read "Pending tests" below. |
| `task test:loom` | Checks all thread orders of the job registry with loom. Run it when you change `crates/loremaster-mcp/src/jobs/`. Read "Loom tests" below. |
| `task lint:workflows` | Examines the GitHub workflows with actionlint and zizmor. Run it when you change `.github/`. It needs Docker and uv. |

### dramaturge

loremaster takes the dramaturge crates from crates.io (`"0.3"` in `Cargo.toml`). CI builds with `--locked`, so it tests exactly the version in `Cargo.lock`.

- When loremaster needs a change in dramaturge, make the change in dramaturge first and release a new dramaturge version. Then raise the version in `Cargo.toml` if needed and run `cargo update -p dramaturge-model -p dramaturge-runtime -p dramaturge-authoring -p dramaturge-syntax` in the same pull request.
- The CI job "dramaturge main (not required)" tests loremaster against the newest dramaturge on `main`. When it is red, the next bump needs changes in loremaster.
- To work on both repositories at once, put a patch into `.cargo/config.toml` (git does not track it):

  ```toml
  [patch.crates-io]
  dramaturge-model = { path = "../dramaturge/crates/dramaturge-model" }
  dramaturge-runtime = { path = "../dramaturge/crates/dramaturge-runtime" }
  dramaturge-authoring = { path = "../dramaturge/crates/dramaturge-authoring" }
  dramaturge-syntax = { path = "../dramaturge/crates/dramaturge-syntax" }
  ```

  Cargo uses a patch only when its version satisfies the requirement in `Cargo.toml`. Between releases, dramaturge keeps the released version on `main` (cargo-release sets the version at the release), so the patch applies. If `../dramaturge` has a version outside `0.3`, or a pre-release such as `0.3.2-alpha.1` (a requirement without a pre-release never takes one), cargo prints "Patch ... was not used" and builds the crates.io release: then relax the requirement in `Cargo.toml` locally, and do not commit that change.

  With the patch, each cargo command rewrites `Cargo.lock` with path sources. Do not commit that file: `task lock:check` (part of `task verify`, and the first step in CI) fails on a staged `Cargo.lock` without the crates.io source.

### Pending tests

Some tests describe a feature before the engine has it, for example the golden slice of the roadmap (phase 0). They are pending:

- A Rust test gets `#[ignore = "pending: phase N, <needs>"]`. `<needs>` is the requirement id or the issue, for example `#[ignore = "pending: phase 1, WO, #44"]`. Each `#[ignore]` in the workspace must have this form.
- A walkthrough gets `pending: { phase: N, needs: <needs> }`. Read "Walkthrough tests" in `docs/writing-stories.md`.

`cargo test` skips the pending Rust tests. `task test:pending` (the script `scripts/pending-tests.sh`) runs them. A pending test must fail. When one passes, the script fails with "unexpected pass". Then remove the mark in the same pull request.

CI runs the script in the job "Pending tests (not required)". The job is not a required check, so a pending test does not block a merge. A red job means that a pending test passes now.

### Loom tests

The runtime has no threads, and `World` is shared through `Arc` but never changed. The HTTP server answers one request after the other. The one state that threads share is the job registry of `loremaster-mcp` (`src/jobs/registry.rs`): the request thread starts, reads and cancels jobs, and each job has a thread that reads the agent's output and sets the final status. [loom](https://docs.rs/loom) checks all orders of these threads.

The registry takes `Arc` and `Mutex` from loom when the crate is built with `--cfg loom`, and from `std` else. The loom tests are in the module `loom_tests` there, with a fake agent process. `task test:loom` runs them (`RUSTFLAGS="--cfg loom" cargo test -p loremaster-mcp --release --lib loom`). The other tests of the crate do not run under loom. When you add a lock or a thread to the jobs, add a loom test for it. CI runs the tests in the job "Loom (not required)".

### Fuzzing

The fuzz targets in `fuzz/` feed random input to the code that reads input from outside: no input may make it panic or hang. They use [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and need a nightly toolchain, so `fuzz/` is a crate of its own, outside the workspace (its `Cargo.lock` is not committed).

| Target | Input |
| --- | --- |
| `save` | A save file, whole or as fields of a real save (Lighthouse, Salt Tower in a combat) |
| `protocol` | JSON-RPC lines to `loremaster protocol` |
| `mcp` | JSON-RPC lines to the MCP server (`loremaster mcp`, `/mcp` of `loremaster serve`); slow, as each input opens the project, so give it minutes |
| `commands` | Typed commands in the Lighthouse or the Salt Tower |
| `story_yaml` | Story files (`story.yaml`, a room, a thing, rules, a machine) |
| `condition`, `markup` | Condition and markup text |
| `dice_and_sums` | Dice and `calc` sums |

```sh
rustup toolchain install nightly
cargo install cargo-fuzz
task fuzz                              # each target for 30 s
task fuzz TARGETS=save DURATION=600    # one target for 10 minutes
```

`fuzz/seeds/<target>/` holds a few start inputs; the corpus that a run builds (`fuzz/corpus/`) and the inputs that fail (`fuzz/artifacts/`) are not committed. To run a failing input again: `cd fuzz && cargo +nightly fuzz run <target> artifacts/<target>/<file>`. Fix the bug with a regression test in the crate, or open an issue with the input when the fix is large. CI does not run the targets: a build with the fuzz instrumentation takes too long for every pull request. Run them when you change one of the parsers, loaders or servers above.

The crate `loremaster` embeds the web editor by default (the feature `embed-ui`). The editor comes from `crates/loremaster-mcp/ui/`, and git ignores this folder. If the folder is missing, the build stops with an error. Thus the Rust tasks and the Rust CI job use `--no-default-features`. To build with the editor, run `task release:ui` first. The CI web job builds the web app, copies it into the crate and runs `cargo package`.

## Releases

loremaster uses [cargo-release](https://github.com/crate-ci/cargo-release). The configuration is in `release.toml` and in `[package.metadata.release]` of some crates.

- These crates go to crates.io: `loremaster`, `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-build`, `loremaster-runtime`, `loremaster-workbench`, `loremaster-tools`, `loremaster-dsl`, `loremaster-macros` and `loremaster-mcp`.
- `loremaster-wasm` and `loremaster-zil` have `publish = false` and `release = false`. Their version changes together with the other crates.
- All crates have one version. A release makes one commit "Release <version>" and one tag `v<version>`. Git signs the commit and the tag.
- Releases come only from `main`.
- cargo-release pushes the release commit and the tag directly to `main`, without a pull request. If you add a branch protection rule to `main`, let the release push through it. If the push fails, the crates are already on crates.io, and you cannot remove them from crates.io.

### Why there are two commands

The crate `loremaster-mcp` contains the built web editor in `crates/loremaster-mcp/ui/`. git ignores this folder. Thus `cargo publish` sees its files as not committed, and stops. Only `--allow-dirty` prevents this, but cargo-release does not give this option to `cargo publish`.

Thus a release has two commands:

1. `cargo release` changes the version and `CHANGELOG.md`, builds the web editor, makes the commit, uploads the 9 crates without the editor, makes the tag and pushes.
2. `task release:publish` uploads `loremaster-mcp` and `loremaster` with `--allow-dirty`. It stops if the working tree has other changes, or if `ui/` is missing.

### Before the first release

1. Install cargo-release: `cargo install cargo-release`.
2. Get an API token from crates.io. Give it the scopes `publish-new` and `publish-update`.
3. Log in: `cargo login`. Then type the token.

### Make a release

1. Switch to `main` and pull. Make sure that the working tree is clean.
2. Add the changes to the section "Unreleased" in `CHANGELOG.md`. Commit them.
3. Do a dry run. The dry run does not upload, does not commit and does not push. It runs the hook, thus it builds the web editor:

   ```sh
   cargo release <level>
   ```

   `<level>` is `patch`, `minor`, `major` or a version, for example `0.2.0`. For the first release, use `0.1.0`: this is the version now.
4. Read the output. If it is correct, do the release:

   ```sh
   cargo release <level> --execute
   ```

   cargo-release does these steps:

   1. It runs `task release:ui` (the hook of `loremaster-mcp`). This builds the web app below `/` and copies it into `crates/loremaster-mcp/ui/`.
   2. It sets the new version in all crates and moves "Unreleased" in `CHANGELOG.md` to the new version.
   3. It makes the release commit.
   4. It uploads `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-build`, `loremaster-runtime`, `loremaster-workbench`, `loremaster-tools`, `loremaster-dsl` and `loremaster-macros` in the order of their dependencies. The proc-macro crate `loremaster-macros` comes after `loremaster-dsl`: the macro builds packs with the builder at compile time.
   5. It makes the tag and pushes the commit and the tag.

   If the dry run stops with "depends on unpublished workspace package", a crate that was never on crates.io is a dependency of another crate (in 0.2.0: `loremaster-dsl` and `loremaster-workbench`). cargo-release checks the old requirement (`^0.1.0`) before it sets the new version, and finds nothing on crates.io. Then do the steps one by one, with the new version set first:

   ```sh
   cargo release version <version> --execute
   cargo release replace --execute
   cargo release hook --execute
   cargo release commit --execute
   cargo release publish --execute
   cargo release tag --execute
   cargo release push --execute
   ```

   For a dry run of the upload, leave out `--execute` from `cargo release publish` after the commit, and remove the commit with `git reset --hard HEAD~1` afterwards.

5. Upload the two crates with the editor. Do this on the release commit, and do not change files before:

   ```sh
   task release:publish
   ```

   For a dry run, use `task release:publish -- --dry-run`. Before the first upload of the other crates, this dry run stops with "no matching package named `loremaster-project` found". This is correct. `task package` does the full check without crates.io.

   `task release:publish` stops if HEAD has no tag `v<version>`. Thus it uploads only from the release commit.

### If a step fails

A release is not one operation. Step 4 uploads 9 crates and pushes the tag before step 5 starts. crates.io does not let you remove or replace an uploaded version.

- If step 4 stops before the upload, nothing is on crates.io. Look at `git log -1`. If the last commit is "Release <version>", remove it with `git reset --hard HEAD~1`. If not, remove the changed files with `git checkout -- .`. Then correct the problem and start again.
- If step 4 stops after some uploads, do not change the version. Correct the problem, then run `cargo release publish --execute` and `cargo release tag --execute` and `cargo release push --execute`. cargo-release does not upload a version that is already on crates.io.
- If step 5 fails, do these steps:

  1. Check out the tag: `git checkout v<version>`.
  2. Build the web editor again: `task release:ui`.
  3. Run `task release:publish` again.
  4. Go back to `main`: `git switch main`.

### The first release

crates.io lets you make only 5 new crates at one time. After that, it lets you make 1 new crate each 10 minutes. Step 4 uploads 5 new crates. Thus, for the first release, wait 20 minutes between step 4 and step 5. From the second release on, only a crate that is new in that release counts, for example `loremaster-build`, `loremaster-dsl`, `loremaster-macros` and `loremaster-workbench` in 0.2.0. Four new crates are in the limit, and you do not have to wait.

### Size of the package

The `.crate` file of `loremaster-mcp` is approximately 3 MB. The limit of crates.io is 10 MB. Do not use the GitHub Pages build (`task build:static`) for the crate. It is below `/loremaster`, and the embedded editor must be below `/`.

Binaries on GitHub Releases are not available yet. They can come later.

## Commit messages

- Write the first line as a command, for example "Add an MCP server for projects". Start with a capital letter. Do not put a full stop at the end.
- Keep the first line short. Put more information in the body, after an empty line.

## Rules for the work

- Write all documentation in ASD-STE100 Simplified Technical English. The "Foundations/Writing documentation" page in Storybook gives the rules.
- For web work, obey the design system rules in [Design system](#design-system): design tokens only, WCAG 2.2 AA, all widths from 320px, and a story with documentation for each element.
