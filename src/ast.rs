use std::borrow::Cow;
use std::collections::HashMap;
use itertools::Itertools;

pub type ReactionTerms<'s> = HashMap<Symbol<'s>, usize>;

#[derive(Debug)]
pub enum Goal<'s> {
    Resources(ReactionTerms<'s>),
    Reactions
}

#[derive(Debug)]
pub enum Item<'s> {
    Target(Target<'s>),
    Reaction(Reaction<'s>),
}

#[derive(Debug)]
pub enum TargetItem<'s> {
    Input(Vec<Symbol<'s>>),
    Constraint(Vec<ReactionTerms<'s>>),
    InTime(usize),
    Goal(Goal<'s>),
}

#[derive(Copy, Clone, Debug)]
pub struct Cost(pub isize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Symbol<'s>(pub &'s str);

#[derive(Debug)]
pub struct Target<'s> {
    pub inputs: Vec<Symbol<'s>>,
    pub constraints: ReactionTerms<'s>,
    pub in_time: usize,
    pub name: &'s str,
    pub goal: Option<Goal<'s>>,
    pub span: (usize, usize),
}

#[derive(Debug)]
pub struct Program<'s> {
    pub targets: HashMap<&'s str, Target<'s>>,
    pub reactions: Vec<Reaction<'s>>,
}

#[derive(Debug)]
pub struct Reaction<'s> {
    pub inputs: ReactionTerms<'s>,
    pub outputs: ReactionTerms<'s>,
    pub cost: Cost,
    pub label: Option<Cow<'s, str>>
}

impl<'s> Reaction<'s> {
    pub fn var_name(&self) -> String {
        format!(
            "machine_{}_into_{}",
            self.inputs
                .iter()
                .map(|(symbol, scalar)| format!("{scalar}{}", symbol.0.replace("-", "_")))
                .format("_"),
            self.outputs
                .iter()
                .map(|(symbol, scalar)| format!("{scalar}{}", symbol.0.replace("-", "_")))
                .format("_")
        )
    }
}
