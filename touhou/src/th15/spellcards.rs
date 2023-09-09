use std::fmt::Display;
use std::num::NonZeroU16;

use touhou_macros::spellcards;

use super::{Difficulty, Stage, Touhou15};
use crate::types::{GameId, GameValue, InvalidCardId, SpellCardInfo, SpellType};

const SPELL_CARDS: &[SpellCardInfo<Touhou15>; 119] = spellcards! {
    Game: Touhou15,
    S1: {
        Midboss: [
            Hard | Lunatic : #1 "Assassin's Bullet \"Speed Strike\"",
        ],
        Boss: [
            {
                Easy | Normal | Hard: #3 "Bullet Sign \"Eagle Shooting\"",
                Lunatic: #6 "Bullet Sign \"The Eagle Has Shot Its Target\""
            },
            Easy | Normal | Hard | Lunatic: #7 "Gun Sign \"Lunatic Gun\"",
        ]
    },
    S2: {
        Boss: [
            {
                Easy | Normal: #11 "Rabbit Sign \"Strawberry Dango\"",
                Hard | Lunatic: #13 "Rabbit Sign \"Berry Berry Dango\"",
            },
            Easy | Normal | Hard | Lunatic: #15 "Rabbit Sign \"Dango Influence\"",
            {
                Easy | Normal | Hard: #19 "Moon-Viewing \"September Full Moon\"",
                Lunatic: #22 "Moon-Viewing Saké \"Lunatic September\"",
            }
        ]
    },
    S3: {
        Midboss: [
            {
                Easy | Normal: #23 "Dream Sign \"Scarlet Nightmare\"",
                Hard | Lunatic: #25 "Dream Sign \"Scarlet Oppressive Nightmare\"",
            }
        ],
        Boss: [
            {
                Easy | Normal: #27 "Dream Sign \"Indigo Dream of Anxiety\"",
                Hard: #29 "Dream Sign \"Indigo Three-Layered Dream of Anxiety\"",
                Lunatic: #30 "Dream Sign \"Eternally Anxious Dream\"",
            },
            {
                Easy | Normal: #31 "Dream Sign \"Ochre Confusion\"",
                Hard | Lunatic: #33 "Dream Sign \"Ochre Labyrinthine Confusion\"",
            },
            {
                Easy | Normal: #35 "Dream Sign \"Dream Catcher\"",
                Hard: #37 "Dream Sign \"Azure Dream Catcher\"",
                Lunatic: #38 "Dream Sign \"Losing Oneself in a Dream\"",
            },
            Easy | Normal | Hard | Lunatic: #39 "Moon Sign \"Ultramarine Lunatic Dream\"",
        ]
    },
    S4: {
        Midboss: [
            {
                Easy | Normal: #43 "Orb Sign \"Disorderly Flock's Curse\"",
                Hard: #45 "Orb Sign \"Disorderly Flock's Reverse Curse\"",
                Lunatic: #46 "Orb Sign \"Disorderly Flock's Duplex Curse\"",
            }
        ],
        Boss: [
            {
                Easy | Normal: #47 "Orb Sign \"Impure Body Detection Mines\"",
                Hard | Lunatic: #49 "Orb Sign \"Impure Body Detection Mines V2\"",
            },
            {
                Easy | Normal: #51 "Orb Sign \"Shotgun Coronation of the Gods\"",
                Hard | Lunatic: #53 "Orb Sign \"Shining Shotgun Coronation of the Gods\"",
            },
            Easy | Normal | Hard | Lunatic: #55 "\"One-Winged White Heron\"",
        ]
    },
    S5: {
        Boss: [
            {
                Easy | Normal: #59 "Hell Sign \"Hell Eclipse\"",
                Hard | Lunatic: #61 "Hell Sign \"Eclipse of Hell\"",
            },
            {
                Easy | Normal: #63 "Hell Sign \"Flash and Stripe\"",
                Hard | Lunatic: #65 "Hell Sign \"Star and Stripe\"",
            },
            {
                Easy | Normal | Hard: #67 "Hellfire \"Graze Inferno\"",
                Lunatic: #70 "Hellfire \"Infernal Essence of Grazing\"",
            },
            Easy | Normal | Hard | Lunatic: #71 "Inferno \"Striped Abyss\"",
            {
                Easy | Normal: #75 "\"Fake Apollo\"",
                Hard | Lunatic: #77 "\"Apollo Hoax Theory\"",
            }
        ]
    },
    S6: {
        Boss: [
            Easy | Normal | Hard | Lunatic: #79 "\"Pure Light of the Palm\"",
            Easy | Normal | Hard | Lunatic: #83 "\"Murderous Lilies\"",
            {
                Easy | Normal: #87 "\"Primordial Divine Spirit World\"",
                Hard | Lunatic: #89 "\"Modern Divine Spirit World\"",
            },
            Easy | Normal | Hard | Lunatic: #91 "\"Trembling, Shivering Star\"",
            Easy | Normal | Hard | Lunatic: #95 "\"Pristine Lunacy\"",
            {
                Easy | Normal | Hard: #99 "\"Overflowing Blemishes\"",
                Lunatic: #102 "\"Refinement of Earthen Impurity\"",
            },
            {
                Easy | Normal: #103 "Pure Sign \"Purely Bullet Hell\"",
                Hard | Lunatic: #105 "Pure Sign \"A Pristine Danmaku Hell\"",
            }
        ]
    },
    Extra: {
        Midboss: [
            #107 "Butterfly \"Butterfly Supplantation\"",
            #108 "Super-Express \"Dream Express\"",
            #109 "Crawling Dream \"Creeping Bullet\"",
        ],
        Boss: [
            #110 "Otherworld \"Oumagatoki\"",
            #111 "Earth \"Impurity Within One's Body\"",
            #112 "Moon \"Apollo Reflection Mirror\"",
            #113 "\"Simple Danmaku for Cornering a Trapped Rat\"",
            #114 "Otherworld \"Hell's Non-Ideal Danmaku\"",
            #115 "Earth \"Rain Falling in Hell\"",
            #116 "Moon \"Lunatic Impact\"",
            #117 "\"Pristine Danmaku for Killing a Person\"",
            #118 "\"Trinitarian Rhapsody\"",
            #119 "\"First and Last Nameless Danmaku\""
        ]
    }
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpellId(NonZeroU16);

impl SpellId {
    pub fn card_info(&self) -> &'static SpellCardInfo<Touhou15> {
        &SPELL_CARDS[(self.0.get() - 1) as usize]
    }
}

impl From<SpellId> for u32 {
    fn from(value: SpellId) -> Self {
        value.0.get() as u32
    }
}

impl TryFrom<u32> for SpellId {
    type Error = InvalidCardId;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if let Ok(Some(value)) = <u16 as TryFrom<u32>>::try_from(value).map(NonZeroU16::new) {
            if value.get() <= (SPELL_CARDS.len() as u16) {
                return Ok(Self(value));
            }
        }

        Err(InvalidCardId::InvalidCard(
            GameId::LoLK,
            value,
            SPELL_CARDS.len() as u32,
        ))
    }
}

impl Display for SpellId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl GameValue for SpellId {
    type RawValue = u32;
    type ConversionError = InvalidCardId;

    fn game_id(&self) -> GameId {
        GameId::LoLK
    }

    fn raw_id(&self) -> u32 {
        (*self).into()
    }

    fn from_raw(id: u32, game: GameId) -> Result<Self, InvalidCardId> {
        if game == GameId::LoLK {
            id.try_into()
        } else {
            Err(InvalidCardId::UnexpectedGameId(game, GameId::LoLK))
        }
    }

    fn name(&self) -> &'static str {
        self.card_info().name
    }
}
