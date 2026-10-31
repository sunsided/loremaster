# Contributing

loremaster uses GitHub Flow. There is one long-lived branch, `main`. Each change goes to `main` through a pull request from a feature branch.

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
| `task lint:workflows` | Examines the GitHub workflows with actionlint and zizmor. Run it when you change `.github/`. It needs Docker and uv. |

The crate `loremaster` embeds the web editor by default (the feature `embed-ui`). The editor comes from `crates/loremaster-mcp/ui/`, and git ignores this folder. If the folder is missing, the build stops with an error. Thus the Rust tasks and the Rust CI job use `--no-default-features`. To build with the editor, run `task release:ui` first. The CI web job builds the web app, copies it into the crate and runs `cargo package`.

## Releases

loremaster uses [cargo-release](https://github.com/crate-ci/cargo-release). The configuration is in `release.toml` and in `[package.metadata.release]` of some crates.

- These crates go to crates.io: `loremaster`, `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-runtime`, `loremaster-tools` and `loremaster-mcp`.
- `loremaster-wasm` and `loremaster-zil` have `publish = false` and `release = false`. Their version changes together with the other crates.
- All crates have one version. A release makes one commit "Release <version>" and one tag `v<version>`. Git signs the commit and the tag.
- Releases come only from `main`.
- cargo-release pushes the release commit and the tag directly to `main`, without a pull request. If you add a branch protection rule to `main`, let the release push through it. If the push fails, the crates are already on crates.io, and you cannot remove them from crates.io.

### Why there are two commands

The crate `loremaster-mcp` contains the built web editor in `crates/loremaster-mcp/ui/`. git ignores this folder. Thus `cargo publish` sees its files as not committed, and stops. Only `--allow-dirty` prevents this, but cargo-release does not give this option to `cargo publish`.

Thus a release has two commands:

1. `cargo release` changes the version and `CHANGELOG.md`, builds the web editor, makes the commit, uploads the 5 crates without the editor, makes the tag and pushes.
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
   4. It uploads `loremaster-model`, `loremaster-lang`, `loremaster-project`, `loremaster-runtime` and `loremaster-tools` in the order of their dependencies.
   5. It makes the tag and pushes the commit and the tag.

5. Upload the two crates with the editor. Do this on the release commit, and do not change files before:

   ```sh
   task release:publish
   ```

   For a dry run, use `task release:publish -- --dry-run`. Before the first upload of the other crates, this dry run stops with "no matching package named `loremaster-project` found". This is correct. `task package` does the full check without crates.io.

   `task release:publish` stops if HEAD has no tag `v<version>`. Thus it uploads only from the release commit.

### If a step fails

A release is not one operation. Step 4 uploads 5 crates and pushes the tag before step 5 starts. crates.io does not let you remove or replace an uploaded version.

- If step 4 stops before the upload, nothing is on crates.io. Look at `git log -1`. If the last commit is "Release <version>", remove it with `git reset --hard HEAD~1`. If not, remove the changed files with `git checkout -- .`. Then correct the problem and start again.
- If step 4 stops after some uploads, do not change the version. Correct the problem, then run `cargo release publish --execute` and `cargo release tag --execute` and `cargo release push --execute`. cargo-release does not upload a version that is already on crates.io.
- If step 5 fails, do these steps:

  1. Check out the tag: `git checkout v<version>`.
  2. Build the web editor again: `task release:ui`.
  3. Run `task release:publish` again.
  4. Go back to `main`: `git switch main`.

### The first release

crates.io lets you make only 5 new crates at one time. After that, it lets you make 1 new crate each 10 minutes. Step 4 uploads 5 new crates. Thus, for the first release, wait 20 minutes between step 4 and step 5. From the second release on, no crate is new, and you do not have to wait.

### Size of the package

The `.crate` file of `loremaster-mcp` is approximately 3 MB. The limit of crates.io is 10 MB. Do not use the GitHub Pages build (`task build:static`) for the crate. It is below `/loremaster`, and the embedded editor must be below `/`.

Binaries on GitHub Releases are not available yet. They can come later.

## Commit messages

- Write the first line as a command, for example "Add an MCP server for projects". Start with a capital letter. Do not put a full stop at the end.
- Keep the first line short. Put more information in the body, after an empty line.

## Rules for the work

- Write all documentation in ASD-STE100 Simplified Technical English. The "Foundations/Writing documentation" page in Storybook gives the rules.
- For web work, obey the design system rules in the [README](README.md#design-system): design tokens only, WCAG 2.2 AA, all widths from 320px, and a story with documentation for each element.
