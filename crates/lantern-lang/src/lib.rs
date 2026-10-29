//! The small languages inside a lantern story.
//!
//! * **Conditions** such as `time.phase == night and not lamp.lit`. They are
//!   parsed, type-checked against the story's variables, things and rooms, and
//!   lowered to dramaturge [`Condition`](dramaturge_model::Condition) trees.
//!   Story variables and thing properties become world facts; values the
//!   runtime derives (time, location, visits) become `Custom` conditions that
//!   the runtime answers.
//! * **Inline markup** in fragment text, such as
//!   `The {night: moonlit | sunlit} courtyard{first: , new to you,} lies quiet.`

mod derived;
mod error;
mod expr;
mod lexer;
mod lower;
mod markup;
mod scope;

pub use derived::{CmpOp, Derived, NameForm, Operand};
pub use error::{LangError, Span};
pub use expr::{Expr, parse_expr};
pub use lower::compile_condition;
pub use markup::{ChoiceSyntax, Segment, compile_markup};
pub use scope::{Scope, Ty};
