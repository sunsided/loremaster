use crate::lower::resolve;
use crate::{
    CompiledLine, LangError, LineError, Operand, Scope, compile_condition, compile_line, parse_expr,
};
use dramaturge_model::{
    Condition, DialogText, DialogueGraph, EntityId, Fact, FactId, FactScope, Memory, Outcome,
    RelationshipAxisId, SymbolTable,
};
use dramaturge_syntax::lower::fact_value;
use dramaturge_syntax::{Cx, Lower, LowerError, Place, Severity, Value, lower};
use loremaster_model::{Dialogue, Effect, Line, PLAYER_SPEAKER};

/// A dialogue lowered to a dramaturge graph.
///
/// dramaturge holds the structure, the conditions and the outcomes. The start
/// nodes are the entry nodes, in order. A `then` list is a host outcome
/// ([`Outcome::Host`]) that the runtime applies. A line is host text
/// ([`DialogText::Host`]): the compiled line that the runtime renders. The
/// speech of a node is spoken by the dialogue's thing, the speech of an
/// option by the player, unless a segment names its speaker with `by`.
pub type CompiledDialogue = DialogueGraph<Vec<Effect>, CompiledLine>;

/// A problem in a dialogue, with the place it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueError {
    /// For example `nodes.meet.options[1].when`.
    pub at: String,
    /// The source text the error is in, to locate it in the file.
    pub source: String,
    pub error: LangError,
}

/// Lower a dialogue to a dramaturge graph with [`dramaturge_syntax::lower`].
/// Node keys are interned in `symbols` as `<id>/<node>`, option keys as
/// `<id>/<option id>`, or `<id>/<node>#<index>` for an option without `id`;
/// the player, things and memories must be in it already (see
/// [`Scope::symbols`]).
///
/// The structure (`start`, the targets, the keys) is checked by
/// [`dramaturge_syntax::check`]; its first error is the error here.
pub fn compile_dialogue(
    d: &Dialogue,
    scope: &Scope,
    symbols: &mut SymbolTable,
) -> Result<CompiledDialogue, DialogueError> {
    match lower(d, symbols, &mut Host { d, scope }) {
        Ok(lowered) => Ok(lowered.graph),
        Err(LowerError::Host { place, error, .. }) => Err(DialogueError {
            at: at(d, &place, &error.at),
            ..error
        }),
        Err(LowerError::Check(issues)) => {
            // `lower` fails on a check error; without one (not possible with
            // dramaturge 0.3.0-alpha.4) name the first issue in the message,
            // so `check` does not fold it into the warning of the same issue.
            let Some(issue) = issues.iter().find(|i| i.severity == Severity::Error) else {
                let why = (issues.first())
                    .map_or(String::new(), |i| format!(": {} ({})", i.message, i.path));
                let error = fail("start:", format!("the dialogue can not be lowered{why}"));
                return Err(DialogueError {
                    at: "start".into(),
                    ..error
                });
            };
            // Find the text of the place: a start key, else the node key.
            let source = match issue.place {
                Place::Start(i) => d.start.get(i).cloned().unwrap_or_default(),
                _ => (issue.path.strip_prefix("nodes."))
                    .and_then(|rest| rest.split(['.', '[']).next())
                    .map_or(String::new(), |k| format!("{k}:")),
            };
            Err(DialogueError {
                at: issue.path.clone(),
                ..fail(&source, issue.message.clone())
            })
        }
        // A kind of error that a later dramaturge adds.
        Err(_) => Err(not_lowered()),
    }
}

/// The place of an error as loremaster's diagnostics name it: a line error
/// is at its node or option (`.say[i]` inside a line with segments), an
/// outcome error at the node or option that has the outcome.
fn at(d: &Dialogue, place: &Place, inner: &str) -> String {
    let base = match place {
        Place::NodeSay(n) | Place::NodeOutcome(n, _) => Place::Node(*n),
        Place::OptionOutcome(n, o, _) => Place::Option(*n, *o),
        other => other.clone(),
    };
    let path = base.path(d);
    match place {
        Place::NodeSay(_) | Place::Line(..) | Place::Option(..) if !inner.is_empty() => {
            format!("{path}.say{inner}")
        }
        _ => path,
    }
}

/// loremaster's payloads for dramaturge's lowering. A hook error has the
/// place inside a line in `at` (see [`LineError::at`]).
struct Host<'a> {
    d: &'a Dialogue,
    scope: &'a Scope,
}

/// An error in `source`, with its place to be filled in.
fn lang(source: &str, error: LangError) -> DialogueError {
    let source = source.to_string();
    DialogueError {
        at: String::new(),
        source,
        error,
    }
}

/// An error without an issue to show, at the dialogue's `start`.
fn not_lowered() -> DialogueError {
    let error = fail("start:", "the dialogue can not be lowered".into());
    DialogueError {
        at: "start".into(),
        ..error
    }
}

fn fail(source: &str, message: String) -> DialogueError {
    lang(source, LangError::new(message, 0..source.len()))
}

impl Host<'_> {
    /// The default speaker of the line at `place`: the node's speaker (the
    /// dialogue's thing), the line's speaker, or the player for an option.
    fn speaker(&self, place: &Place) -> &str {
        let node = |n: &usize| &self.d.nodes[*n];
        match place {
            Place::NodeSay(n) => node(n).speaker.as_deref().unwrap_or(&self.d.with),
            Place::Line(n, l) => &node(n).lines[*l].speaker,
            _ => PLAYER_SPEAKER,
        }
    }

    /// A name that `set` or `add` can change: a story variable or a thing
    /// property. It is a world fact with the full name.
    fn fact_name(&self, name: &str) -> Result<FactId, DialogueError> {
        let lang = |e| lang(name, e);
        match resolve(&parse_expr(name).map_err(lang)?, self.scope).map_err(lang)? {
            (Operand::Fact(_), _) => Ok(FactId::new(name)),
            _ => Err(fail(
                name,
                format!("`{name}` is computed by the game; it can not be set"),
            )),
        }
    }
}

type Outcomes = Result<Vec<Outcome<Vec<Effect>>>, DialogueError>;

impl Lower<String, Line, Effect> for Host<'_> {
    type Host = Vec<Effect>;
    type Text = CompiledLine;
    type Error = DialogueError;

    fn condition(&mut self, _: &mut Cx<'_>, when: &String) -> Result<Condition, DialogueError> {
        let c = compile_condition(when, self.scope).map_err(|e| lang(when, e))?;
        // A dialogue's conditions have no perceiver: `seen_by` would never hold.
        if crate::has_seen_by(&c) {
            return Err(lang(
                when,
                LangError::new(crate::SEEN_BY_ELSEWHERE, 0..when.len()),
            ));
        }
        Ok(c)
    }

    fn text(
        &mut self,
        cx: &mut Cx<'_>,
        say: &Line,
    ) -> Result<DialogText<CompiledLine>, DialogueError> {
        // `say: { text: <id> }`: the runtime shows the named text pool.
        if let Some(pool) = &say.pool {
            return Ok(DialogText::Pool(pool.as_str().into()));
        }
        let speaker = self.speaker(&cx.place);
        compile_line(say, Some(speaker), self.scope)
            .map(DialogText::Host)
            .map_err(|LineError { at, source, error }| DialogueError { at, source, error })
    }

    fn effects(&mut self, _: &mut Cx<'_>, then: &[Effect]) -> Outcomes {
        Ok(vec![Outcome::Host(then.to_vec())])
    }

    fn set(&mut self, _: &mut Cx<'_>, name: &str, value: &Value) -> Outcomes {
        let fact = Fact::new(self.fact_name(name)?, fact_value(value));
        Ok(vec![Outcome::SetFact {
            scope: FactScope::World,
            fact,
        }])
    }

    fn add(&mut self, _: &mut Cx<'_>, name: &str, delta: i64) -> Outcomes {
        Ok(vec![Outcome::AddFact {
            scope: FactScope::World,
            fact_id: self.fact_name(name)?,
            delta,
        }])
    }

    /// The relation of the player to the dialogue's thing, of a declared axis.
    fn relation(&mut self, cx: &mut Cx<'_>, axis: &str, delta: i32) -> Outcomes {
        if !self.scope.relations.contains(axis) {
            return Err(fail(
                axis,
                format!("unknown relation `{axis}`; declare it in `relations` in story.yaml"),
            ));
        }
        Ok(vec![Outcome::AddRelationship {
            source: cx.player,
            target: cx.with,
            axis: RelationshipAxisId::new(axis),
            delta,
        }])
    }

    /// A memory that some condition can ask for, kept by the dialogue's thing.
    fn remember(&mut self, cx: &mut Cx<'_>, key: &str) -> Outcomes {
        let Some(id) = self.scope.symbols.memory(key) else {
            return Err(fail(key, format!("unknown memory `{key}`")));
        };
        Ok(vec![Outcome::AddMemory {
            owner: cx.with,
            memory: Memory::new(id).with_subject(cx.player),
        }])
    }

    fn entity(&mut self, _: &mut Cx<'_>, key: &str) -> Result<EntityId, DialogueError> {
        if key == PLAYER_SPEAKER {
            return Ok(self.scope.player());
        }
        self.scope
            .entity(key)
            .ok_or_else(|| fail(key, format!("unknown thing `{key}`")))
    }
}

#[cfg(test)]
mod tests;
