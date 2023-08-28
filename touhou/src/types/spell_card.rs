use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Deref;
use std::str;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{impl_wrapper_traits, Difficulty, Game, GameId, SpellCardId, Stage};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SpellCardInfo {
    pub name: &'static str,
    pub difficulty: Difficulty,
    pub stage: Stage,
    pub is_midboss: bool,
}

#[repr(transparent)]
pub struct SpellCard<G: Game>(G::SpellID);

impl<G: Game> AsRef<G::SpellID> for SpellCard<G> {
    fn as_ref(&self) -> &G::SpellID {
        &self.0
    }
}

impl<G: Game> Deref for SpellCard<G> {
    type Target = G::SpellID;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<G: Game> SpellCard<G> {
    pub const fn new(card_id: G::SpellID) -> Self {
        Self(card_id)
    }

    pub fn unwrap(self) -> G::SpellID {
        self.0
    }

    pub fn id(&self) -> u32 {
        self.0.raw_id()
    }

    pub fn info(&self) -> &'static SpellCardInfo {
        self.0.card_info()
    }

    pub fn name(&self) -> &'static str {
        self.info().name
    }

    pub fn difficulty(&self) -> Difficulty {
        self.info().difficulty
    }

    pub fn stage(&self) -> Stage {
        self.info().stage
    }

    pub fn is_midboss(&self) -> bool {
        self.info().is_midboss
    }
}

impl<G: Game> Debug for SpellCard<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SpellCard<{}>({:?} : {})",
            self.0.game_id().abbreviation(),
            self.0,
            self.name()
        )
    }
}

impl<G: Game> Display for SpellCard<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} #{}: {}",
            self.0.game_id().abbreviation(),
            self.0.raw_id(),
            self.name()
        )
    }
}

impl_wrapper_traits!(SpellCard, u32, G::SpellID);

#[derive(Debug, Clone, Copy, Error)]
pub enum InvalidCardId {
    #[error("Invalid card ID {1} for {0} (valid values are 1..={2})")]
    InvalidCard(GameId, u32, u32),
    #[error("Invalid game ID {0}")]
    InvalidGameId(u8),
    #[error("Incorrect game ID {0} (expected {1})")]
    UnexpectedGameId(GameId, GameId),
}