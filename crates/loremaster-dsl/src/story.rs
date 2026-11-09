//! The story header: everything of `story.yaml`.

use crate::convert::{IntoTime, IntoVarValue, id, strings};
use crate::pack::register;
use crate::{HasFrames, HasNotes, IntoFragment};
use loremaster_model::{
    ClockConfig, Frames, Id, Limit, Note, Phase, Rank, Score, Value, VarDef, VarType,
};
use loremaster_project::{Pack, StoryHeader};

/// The story header of a pack, from [`BuildPack::story`](crate::BuildPack::story).
pub struct StoryBuilder<'a> {
    pack: &'a mut Pack,
}

impl<'a> StoryBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, title: String, start: Id) -> Self {
        assert!(
            pack.header.is_none(),
            "pack `{}` has a story header already",
            pack.name
        );
        pack.header = Some(StoryHeader {
            title,
            author: None,
            start,
            intro: Vec::new(),
            clock: ClockConfig::default(),
            vars: Vec::new(),
            verbs: Vec::new(),
            relations: Vec::new(),
            frames: Frames::default(),
            seed: 0,
            reload_rolls: Default::default(),
            death_prompt: true,
            carry_limit: None,
            playable: Vec::new(),
            step_limit: None,
            stats: Vec::new(),
            export: Default::default(),
            import: Default::default(),
            notes: Vec::new(),
            timers: Vec::new(),
            machines: Vec::new(),
            behaviors: Vec::new(),
            score: None,
            interface: None,
        });
        register(pack, "story".into());
        StoryBuilder { pack }
    }

    fn header(&mut self) -> &mut StoryHeader {
        self.pack
            .header
            .as_mut()
            .expect("the builder made the header")
    }

    /// `interface:` the contract with the host (IF-1).
    pub fn interface(mut self, interface: loremaster_model::Interface) -> Self {
        self.header().interface = Some(interface);
        self
    }

    /// `author:`.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.header().author = Some(author.into());
        self
    }

    /// `death_prompt: false`: an ending of kind `death` does not ask to undo,
    /// restart or restore.
    pub fn no_death_prompt(mut self) -> Self {
        self.header().death_prompt = false;
        self
    }

    /// `seed:`, for shuffled and sticky fragments.
    pub fn seed(mut self, seed: u64) -> Self {
        self.header().seed = seed;
        self
    }

    /// One fragment of `intro:`.
    pub fn intro(mut self, f: impl IntoFragment) -> Self {
        self.header().intro.push(f.into_fragment());
        self
    }

    /// `clock: { start }`: the time of day at the start.
    #[track_caller]
    pub fn clock_start(mut self, time: impl IntoTime) -> Self {
        self.header().clock.start = time.into_time();
        self
    }

    /// `clock: { minutes_per_turn }`.
    pub fn minutes_per_turn(mut self, minutes: u16) -> Self {
        self.header().clock.minutes_per_turn = minutes;
        self
    }

    /// `clock: { phases }`: all phases of the day as `(name, from)`. They
    /// replace the default phases.
    #[track_caller]
    pub fn phases<T: IntoTime>(
        mut self,
        phases: impl IntoIterator<Item = (impl Into<String>, T)>,
    ) -> Self {
        // A loop, not a closure: a closure would hide the caller's line
        // from the panic of a malformed time.
        let mut list = Vec::new();
        for (name, from) in phases {
            list.push(Phase {
                name: name.into(),
                from: from.into_time(),
            });
        }
        self.header().clock.phases = list;
        self
    }

    /// A variable of type `bool` or `int`, with its initial value. The type
    /// comes from the value; use [`var_enum`](Self::var_enum) for an enum.
    pub fn var(mut self, var: impl AsRef<str>, initial: impl IntoVarValue) -> Self {
        let (ty, initial) = initial.into_var_value();
        self.header().vars.push(VarDef {
            id: id(var),
            ty,
            initial,
            min: None,
            max: None,
        });
        self
    }

    /// The lowest value of an `int` variable: a number, or the id of
    /// another `int` variable. The engine clamps each write.
    #[track_caller]
    pub fn var_min(mut self, var: impl AsRef<str>, min: impl Into<Limit>) -> Self {
        self.declared(var.as_ref()).min = Some(min.into());
        self
    }

    /// The highest value of an `int` variable, for example
    /// `.var_max("stamina", "stamina_initial")`. The engine clamps each write.
    #[track_caller]
    pub fn var_max(mut self, var: impl AsRef<str>, max: impl Into<Limit>) -> Self {
        self.declared(var.as_ref()).max = Some(max.into());
        self
    }

    #[track_caller]
    fn declared(&mut self, var: &str) -> &mut VarDef {
        let Some(v) = self.header().vars.iter_mut().find(|v| v.id.as_str() == var) else {
            panic!("the variable `{var}` is not declared: call `var` first");
        };
        v
    }

    /// A variable of an enum type: its values and the initial one.
    pub fn var_enum(
        mut self,
        var: impl AsRef<str>,
        values: impl IntoIterator<Item = impl AsRef<str>>,
        initial: impl Into<String>,
    ) -> Self {
        self.header().vars.push(VarDef {
            id: id(var),
            ty: VarType::Enum(strings(values)),
            initial: Value::Text(initial.into()),
            min: None,
            max: None,
        });
        self
    }

    /// A relation axis between the player and the characters, such as `trust`.
    pub fn relation(mut self, relation: impl AsRef<str>) -> Self {
        self.header().relations.push(id(relation));
        self
    }

    /// `score:`: the maximum score, in the variable `score`.
    pub fn score(mut self, max: i64) -> Self {
        self.header().score = Some(Score {
            var: Id::new("score"),
            max,
            ranks: Vec::new(),
        });
        self
    }

    /// The variable of the score, when it is not `score`. Call after [`score`](Self::score).
    #[track_caller]
    pub fn score_var(mut self, var: impl AsRef<str>) -> Self {
        self.score_mut().var = id(var);
        self
    }

    /// A rank of the score, from a score up. Call after [`score`](Self::score).
    #[track_caller]
    pub fn rank(mut self, from: i64, name: impl Into<String>) -> Self {
        self.score_mut().ranks.push(Rank {
            from,
            name: name.into(),
        });
        self
    }

    #[track_caller]
    fn score_mut(&mut self) -> &mut Score {
        match &mut self.header().score {
            Some(score) => score,
            None => panic!("call `score` first"),
        }
    }
}

impl HasNotes for StoryBuilder<'_> {
    fn notes_mut(&mut self) -> &mut Vec<Note> {
        &mut self.header().notes
    }
}

impl HasFrames for StoryBuilder<'_> {
    fn frames_mut(&mut self) -> &mut Frames {
        &mut self.header().frames
    }
}
