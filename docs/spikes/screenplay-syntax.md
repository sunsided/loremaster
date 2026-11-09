# Spike: screenplay syntax for dialogue in `world!` (#112)

## The syntax

In a `world!` dialogue line (a node's line, a speaker line, an option), a
stage direction in parentheses is narration, and it makes the line a
*screenplay line*: its bare strings are speech.

```rust
keeper: "Nobody." (turns the poker in his hands.);
// is
keeper: says "Nobody." narrate "turns the poker in his hands.";

who_fed_it: (He turns the poker in his hands.) "Nobody. The stove was cold all night." { .. }
// is
who_fed_it: "He turns the poker in his hands." says "Nobody. The stove was cold all night." { .. }
```

- The direction is words and `. , ! ? ; :`, or one string for anything
  else: `("He doesn't look up.")`. An apostrophe, a quote or a `-` does not
  tokenize as Rust, so prose with them needs the string.
- A direction before any speech is a bare string, one after speech is
  `narrate` (the "bare string after `says`" lint of #111). That is the
  shortest form a writer would spell out, so both forms give the same model.
- Speech of a screenplay line gets its speaker as `says` does: the line's
  speaker, the dialogue's thing for a node, the player for an option.
- `when <cond> then "…"` stays narration, also in a screenplay line.
- Only in dialogues. A rule's `line` effect has no screenplay form.

## What changed where

- dramaturge: nothing. `dramaturge_syntax::tokens` hands the text after
  `key:` to the host (`ParseHost::text`), and loremaster's host reads the
  segments. The direction is a host payload, so no AST field and no
  dramaturge release is needed.
- loremaster `crates/loremaster-macros/src/parse.rs`: `line_until` reads
  `(…)` in a dialogue (`stage_direction`) and flips the voices at the end of
  the line. About 70 lines; no new AST type, the direction is a
  `Segment` with `SegmentVoice::Plain` or `Narrate`.
- Tests: `crates/loremaster-macros/tests/screenplay.rs` (screenplay form =
  spelled-out form, five cases); `crates/loremaster-cli/tests/slice_dsl.rs`
  (the keeper dialogue with `who_fed_it` in screenplay form equals
  `dialogues/keeper.yaml`). The worked example is
  `examples/lock-keeper-slice/dsl/dialogue.rs` (`who_fed_it`).

## What it costs

- **The model and YAML: nothing.** It is sugar in the `world!` source; the
  model is `Line` with segments as before. The YAML equivalent is the
  segment list, `say: ["He turns …", { says: "Nobody. …" }]`. A YAML
  screenplay form (a string like `'"Nobody." (turns …)'`) would need its
  own small parser and would clash with "a bare string is narration";
  this spike adds none.
- **The meaning of a bare string now depends on the line.** Today a bare
  string is narration everywhere (`world!`, YAML, #164 speaker lines). In a
  screenplay line it is speech. A `(…)` added to or removed from a line
  flips the voice of its other strings. That is the real cost: one more
  rule for readers, writers and AI tools, and two ways to write each line
  with a stage direction.
- **Lines without a direction gain nothing.** Options and plain lines keep
  `says`; the keeper slice changes one node of seven lines. The saving per
  line is `says` plus the quotes of the direction.
- **Tools that show or edit dialogue:** the web UI, the runtime and
  `loremaster check` read the model, so they see nothing new. `loremaster
  codegen` prints dialogues as "port by hand", so it does not have to
  choose a form yet; a later printer of dialogues would have to (print the
  screenplay form when a line has speech and narration, or never).
  Rust editors and rustfmt see a token tree: no highlighting of the
  direction as text, no spell check inside it (unlike a string).
- **dramaturge's own `dialogue!`** (the `Plain` host) does not get the form;
  a world! dialogue and a `dialogue!` dialogue would read differently. Moving
  it there means a dramaturge change to `Plain::text` and its `Text` type,
  which has no speech/narration split today.
- **Error messages** in a screenplay line still talk about `says` and
  `narrate`, and a direction with an apostrophe gets the "write it as a
  string" error.

## Recommendation

**Drop it for now (keep #112 parked).** It works, it is small, and it
needs no dramaturge change, but it pays off only on lines that mix speech
with a stage direction, and it buys that with a context-dependent meaning
of bare strings, the one rule #111 and #164 kept simple. The only
hand-written `world!` dialogue has one such line. If a real `world!` story
writes many mixed lines, adopt it *with changes*: only on speaker lines
(`<speaker>: …`), where "the quoted text is what the speaker says" reads
naturally, and not on a node's own line or an option. The host can not
tell these apart today (`ParseHost::text` gets no context), so that
version needs a small dramaturge change (a context argument or a separate
`line_text` hook). Take the branch of this spike as the start.
