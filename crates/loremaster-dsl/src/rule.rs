//! Rules, the events they listen for ([`on`](crate::on)) and their effects
//! ([`Effects`]).

use crate::convert::{IntoValue, id, ids};
use crate::pack::{register, with_notes};
use loremaster_model::{AwaitSpec, CombatSpec, Effect, EventPattern, Id, Rule, StartTimers};
use loremaster_project::Pack;

/// A rule of a pack, from [`BuildPack::rule`](crate::BuildPack::rule).
///
/// ```
/// use loremaster_dsl::prelude::*;
///
/// let mut p = Pack::new("drawer");
/// p.rule("find_iron_key", on::verb(["open", "drawer"]))
///     .when("not drawer_searched")
///     .set("drawer_searched", true)
///     .move_to("iron_key", "cottage")
///     .say("Under a fold of oilcloth lies an iron key.");
/// assert_eq!(p.rules[0].then.len(), 3);
/// ```
pub struct RuleBuilder<'a> {
    pub(crate) pack: &'a mut Pack,
    pub(crate) i: usize,
}

impl<'a> RuleBuilder<'a> {
    #[track_caller]
    pub(crate) fn new(pack: &'a mut Pack, id: Id, on: EventPattern) -> Self {
        register(pack, format!("rule:{id}"));
        pack.rules.push(Rule {
            id,
            on,
            when: None,
            then: Vec::new(),
            once: Default::default(),
            free: false,
            instead: false,
            within: None,
            priority: 0,
            machine: None,
            closes: Vec::new(),
            reason: None,
            notes: Vec::new(),
        });
        let i = pack.rules.len() - 1;
        RuleBuilder { pack, i }
    }

    fn rule(&mut self) -> &mut Rule {
        &mut self.pack.rules[self.i]
    }

    /// `when:`, the condition.
    pub fn when(mut self, condition: impl Into<String>) -> Self {
        self.rule().when = Some(condition.into());
        self
    }

    /// `once: true`: fire at most once per game.
    pub fn once(mut self) -> Self {
        self.rule().once = loremaster_model::Once::Rule(true);
        self
    }

    /// `once: <key>`: all rules with this key fire at most once in total.
    pub fn once_group(mut self, key: impl AsRef<str>) -> Self {
        self.rule().once = loremaster_model::Once::Group(id(key));
        self
    }

    /// `free: true`: the player's command that fires the rule takes no time.
    pub fn free(mut self) -> Self {
        self.rule().free = true;
        self
    }

    /// `instead: true`: for a verb event, replace the default response.
    pub fn instead(mut self) -> Self {
        self.rule().instead = true;
        self
    }

    /// `within:`, the thing, room or region that the rule belongs to.
    pub fn within(mut self, scope: impl AsRef<str>) -> Self {
        self.rule().within = Some(id(scope));
        self
    }

    /// `priority:`, the order in one scope.
    pub fn priority(mut self, priority: i32) -> Self {
        self.rule().priority = priority;
        self
    }

    /// `closes:` and `reason:`: the endings that the rule closes on purpose
    /// (a point of no return that the ending checks accept), and why.
    pub fn closes(
        mut self,
        endings: impl IntoIterator<Item = impl AsRef<str>>,
        reason: impl Into<String>,
    ) -> Self {
        let rule = self.rule();
        rule.closes = ids(endings);
        rule.reason = Some(reason.into());
        self
    }
}

impl Effects for RuleBuilder<'_> {
    fn effects_mut(&mut self) -> &mut Vec<Effect> {
        &mut self.rule().then
    }
}

with_notes!(RuleBuilder, rules);

/// The events of a rule or a machine step, as in `on:` in YAML.
pub mod on {
    use crate::convert::{IntoTime, id, ids};
    use loremaster_model::{CommandPattern, EventPattern};

    /// `start`: the game begins.
    pub fn start() -> EventPattern {
        EventPattern::Start
    }

    /// `turn`: every turn, after the command.
    pub fn turn() -> EventPattern {
        EventPattern::Turn
    }

    /// `next`: in a machine step, the step runs by itself right after the
    /// machine enters its `from` state.
    pub fn next() -> EventPattern {
        EventPattern::Next
    }

    /// `enter: room`.
    pub fn enter(room: impl AsRef<str>) -> EventPattern {
        EventPattern::Enter(id(room))
    }

    /// `leave: room`.
    pub fn leave(room: impl AsRef<str>) -> EventPattern {
        EventPattern::Leave(id(room))
    }

    /// `time: "HH:MM"`: the clock reaches a time of day.
    #[track_caller]
    pub fn time(time: impl IntoTime) -> EventPattern {
        EventPattern::Time(time.into_time())
    }

    /// `verb: [verb, thing, other]`: after a verb, or instead of it.
    pub fn verb(pattern: impl IntoIterator<Item = impl AsRef<str>>) -> EventPattern {
        EventPattern::Verb(CommandPattern::from_parts(ids(pattern)))
    }

    /// `check: [verb, thing, other]`: before a verb.
    pub fn check(pattern: impl IntoIterator<Item = impl AsRef<str>>) -> EventPattern {
        EventPattern::Check(CommandPattern::from_parts(ids(pattern)))
    }

    /// `{ verb: …, direct: …, actor: … }`: after a verb, with any slot of
    /// the Command record (D6). Build the pattern with
    /// `CommandPattern::new("examine").with_direct("stove")`.
    pub fn command(pattern: CommandPattern) -> EventPattern {
        EventPattern::Verb(pattern)
    }

    /// `{ check: …, direct: …, actor: … }`: before a verb, with any slot of
    /// the Command record.
    pub fn check_command(pattern: CommandPattern) -> EventPattern {
        EventPattern::Check(pattern)
    }

    /// `timer: id`: a timer runs out.
    pub fn timer(timer: impl AsRef<str>) -> EventPattern {
        EventPattern::Timer(id(timer))
    }

    /// `event: id`: an `emit` effect sends it.
    pub fn event(event: impl AsRef<str>) -> EventPattern {
        EventPattern::Event(id(event))
    }

    /// `talk: [dialogue, node]`: a dialogue reaches a node.
    pub fn talk(pattern: impl IntoIterator<Item = impl AsRef<str>>) -> EventPattern {
        EventPattern::Talk(ids(pattern))
    }
}

/// The effects of a rule, a machine step or a timer, as in `then:` in YAML.
/// They are added in order.
///
/// [`set`](Self::set), [`add`](Self::add), [`move_to`](Self::move_to) and
/// [`start_timer_for`](Self::start_timer_for) join a call that directly
/// follows the same effect into it, as one YAML map does:
/// `.set("beacon_lit", true).set("lens.lit", true)` is
/// `set: { beacon_lit: true, lens.lit: true }`. A key that the map has
/// already starts a new effect, so `.add("gold", -5).add("gold", 5)` is two
/// `add` effects and adds 0, as two YAML entries do.
pub trait Effects: Sized {
    #[doc(hidden)]
    fn effects_mut(&mut self) -> &mut Vec<Effect>;

    /// Any effect.
    fn effect(mut self, effect: Effect) -> Self {
        self.effects_mut().push(effect);
        self
    }

    /// `set:` a variable or property.
    fn set(mut self, name: impl Into<String>, value: impl IntoValue) -> Self {
        let (name, value) = (name.into(), value.into_value());
        match self.effects_mut().last_mut() {
            Some(Effect::Set(map)) if !map.contains_key(&name) => {
                map.insert(name, value);
            }
            _ => self.effects_mut().push(Effect::Set([(name, value)].into())),
        }
        self
    }

    /// `add:` to an integer variable.
    fn add(mut self, name: impl Into<String>, amount: i64) -> Self {
        let name = name.into();
        match self.effects_mut().last_mut() {
            Some(Effect::Add(map)) if !map.contains_key(&name) => {
                map.insert(name, amount);
            }
            _ => self
                .effects_mut()
                .push(Effect::Add([(name, amount)].into())),
        }
        self
    }

    /// `calc:` an integer variable or property from a sum, for example
    /// `.calc("total", "r + skill")`. Each call is its own effect, so a
    /// later sum reads the result of an earlier one.
    fn calc(self, name: impl Into<String>, sum: impl Into<String>) -> Self {
        self.effect(Effect::Calc([(name.into(), sum.into())].into()))
    }

    /// `say:` a line: a text, or a [`Line`](loremaster_model::Line) of segments.
    fn say(self, text: impl Into<loremaster_model::Line>) -> Self {
        self.effect(Effect::Say(text.into()))
    }

    /// `move: { thing: target }`. The target is a room, `carried`, `nowhere`,
    /// `in:box` or `on:table`.
    fn move_to(mut self, thing: impl AsRef<str>, target: impl AsRef<str>) -> Self {
        let (thing, target) = (id(thing), id(target));
        match self.effects_mut().last_mut() {
            Some(Effect::Move(map)) if !map.contains_key(&thing) => {
                map.insert(thing, target);
            }
            _ => self
                .effects_mut()
                .push(Effect::Move([(thing, target)].into())),
        }
        self
    }

    /// `goto:` a room: move the player.
    fn goto(self, room: impl AsRef<str>) -> Self {
        self.effect(Effect::Goto(id(room)))
    }

    /// `end:` the game with an ending.
    fn end(self, ending: impl AsRef<str>) -> Self {
        self.effect(Effect::End(id(ending)))
    }

    /// `stop`: no more rules, and in a `check` rule, refuse the action.
    fn stop(self) -> Self {
        self.effect(Effect::Stop)
    }

    /// `emit:` a custom event.
    fn emit(self, event: impl AsRef<str>) -> Self {
        self.effect(Effect::Emit(id(event)))
    }

    /// `start: timer`, with the timer's own turns.
    fn start_timer(self, timer: impl AsRef<str>) -> Self {
        self.effect(Effect::Start(StartTimers::One(id(timer))))
    }

    /// `start: { timer: turns }`.
    fn start_timer_for(mut self, timer: impl AsRef<str>, turns: u32) -> Self {
        let timer = id(timer);
        match self.effects_mut().last_mut() {
            Some(Effect::Start(StartTimers::Many(map))) if !map.contains_key(&timer) => {
                map.insert(timer, turns);
            }
            _ => self
                .effects_mut()
                .push(Effect::Start(StartTimers::Many([(timer, turns)].into()))),
        }
        self
    }

    /// `cancel:` a timer.
    fn cancel(self, timer: impl AsRef<str>) -> Self {
        self.effect(Effect::Cancel(id(timer)))
    }

    /// `await:` stop the rule and ask the host, for example
    /// `.awaits(AwaitSpec { roll: Some("2d6".into()), into: Some("r".into()), ..Default::default() })`.
    /// Only a rule can wait.
    fn awaits(self, spec: AwaitSpec) -> Self {
        self.effect(Effect::Await(Box::new(spec)))
    }

    /// `combat:` a fight by the built-in rules. Only a rule can fight.
    fn combat(self, spec: CombatSpec) -> Self {
        self.effect(Effect::Combat(Box::new(spec)))
    }

    /// `end_combat:` a room. In a rule on a combat event: the combat ends,
    /// and the player goes to the room.
    fn end_combat(self, room: impl AsRef<str>) -> Self {
        self.effect(Effect::EndCombat(id(room)))
    }

    /// Play a sound asset once: `play sound gate_creak`.
    fn play_sound(self, sound: impl AsRef<str>) -> Self {
        self.effect(Effect::PlaySound(id(sound)))
    }
}

/// A plain list of effects, for a dialogue's `then:`
/// ([`DialogueBuilder::then_effects`](crate::DialogueBuilder::then_effects)).
impl Effects for Vec<Effect> {
    fn effects_mut(&mut self) -> &mut Vec<Effect> {
        self
    }
}
