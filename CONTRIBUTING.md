# Contributing

loremaster uses GitHub Flow. There is one long-lived branch, `main`. Each change goes to `main` through a pull request from a feature branch.

## Branches

- `main` must always build and pass all checks. Do not push to `main` directly.
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

The crate `loremaster` embeds the web editor by default (the feature `embed-ui`). The editor comes from `crates/loremaster-mcp/ui/`, and git ignores this folder. If the folder is missing, the build stops with an error. Thus the Rust tasks and the Rust CI job use `--no-default-features`. To build with the editor, run `task release:ui` first. The CI web job builds the web app, copies it into the crate and runs `cargo package`.

## Releases

loremaster uses [cargo-release](https://github.com/crate-ci/cargo-release). The configuration is in `release.toml` and in `[package.metadata.release]` of some crates.

- These crates go to crates.io: `loremaster`, `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-runtime`, `loremaster-tools` and `loremaster-mcp`.
- `loremaster-wasm` and `loremaster-zil` have `publish = false` and `release = false`. Their version changes together with the other crates.
- All crates have one version. A release makes one commit "Release <version>" and one tag `v<version>`. Git signs the commit and the tag.
- Releases come only from `main`.

Before the first release:

1. Install cargo-release: `cargo install cargo-release`.
2. Get an API token from crates.io. Give it the scope `publish-new` and `publish-update`.
3. Log in: `cargo login`. Then type the token.

To make a release:

1. Switch to `main` and pull. Make sure that the working tree is clean.
2. Add the changes to the section "Unreleased" in `CHANGELOG.md`. Commit them.
3. Do a dry run. The dry run does not change files, does not upload and does not push:

   ```sh
   cargo release <level>
   ```

   `<level>` is `patch`, `minor`, `major` or a version, for example `0.2.0`. For the first release, read "The first release" below.
4. Read the output. If it is correct, do the release:

   ```sh
   cargo release <level> --execute
   ```

cargo-release then does these steps:

1. It runs `task release:ui` (the hook of `loremaster-mcp`). This builds the web app below `/` and copies it into `crates/loremaster-mcp/ui/`.
2. It sets the new version in all crates and moves "Unreleased" in `CHANGELOG.md` to the new version.
3. It makes the release commit.
4. It uploads the crates to crates.io in the order of their dependencies.
5. It makes the tag and pushes the commit and the tag.

### The first release

crates.io lets you make only 5 new crates at one time. After that, it lets you make 1 new crate each 10 minutes. The first release has 7 new crates. Thus `cargo release 0.1.0` stops with the error "attempting to publish 7 new crates which is above the rate limit: 5".

Thus do the first release in steps. The version is 0.1.0 already, so no step changes the version. On `main`, with a clean working tree:

1. Build the web editor into the crate:

   ```sh
   task release:ui
   ```

2. Move "Unreleased" in `CHANGELOG.md` to 0.1.0, and make the release commit:

   ```sh
   cargo release replace --execute
   cargo release commit --execute
   ```

3. Upload the first 5 crates:

   ```sh
   cargo release publish --execute --workspace --exclude loremaster-mcp --exclude loremaster
   ```

4. Wait 20 minutes.
5. Upload the other 2 crates. cargo-release skips the crates that are on crates.io already:

   ```sh
   cargo release publish --execute
   ```

6. Make the tag `v0.1.0` and push the commit and the tag:

   ```sh
   cargo release tag --execute
   cargo release push --execute
   ```

Do a dry run of each step first: remove `--execute`. From the second release on, `cargo release <level> --execute` does all steps, because no crate is new.

### Size of the package

The crate `loremaster-mcp` contains the built web editor. The `.crate` file is approximately 3 MB. The limit of crates.io is 10 MB. Do not use the GitHub Pages build (`task build:static`) for the crate. It is below `/loremaster`, and the embedded editor must be below `/`.

Binaries on GitHub Releases are not available yet. They can come later.

## Commit messages

- Write the first line as a command, for example "Add an MCP server for projects". Start with a capital letter. Do not put a full stop at the end.
- Keep the first line short. Put more information in the body, after an empty line.

## Rules for the work

- Write all documentation in ASD-STE100 Simplified Technical English. The "Foundations/Writing documentation" page in Storybook gives the rules.
- For web work, obey the design system rules in the [README](README.md#design-system): design tokens only, WCAG 2.2 AA, all widths from 320px, and a story with documentation for each element.
