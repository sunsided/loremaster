# Contributing

lantern uses GitHub Flow. There is one long-lived branch, `main`. Each change goes to `main` through a pull request from a feature branch.

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

## Commit messages

- Write the first line as a command, for example "Add an MCP server for projects". Start with a capital letter. Do not put a full stop at the end.
- Keep the first line short. Put more information in the body, after an empty line.

## Rules for the work

- Write all documentation in ASD-STE100 Simplified Technical English. The "Foundations/Writing documentation" page in Storybook gives the rules.
- For web work, obey the design system rules in the [README](README.md#design-system): design tokens only, WCAG 2.2 AA, all widths from 320px, and a story with documentation for each element.
