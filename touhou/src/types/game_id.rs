use std::error::Error;
use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub struct InvalidGameId(u8);

impl Display for InvalidGameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid game ID {}", self.0)
    }
}

impl Error for InvalidGameId {}

macro_rules! define_game_info {
    {
        $(
            $id:ident : {
                id_number: $id_num:literal,
                numbered_name: $num_name:literal,
                abbreviation: $abbr:literal,
                full_name: $full_name:literal
            }
        ),*
    } => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
        )]
        #[serde(try_from = "u8", into = "u8")]
        pub enum GameId {
            $(
                $id
            ),*
        }

        impl GameId {
            pub const fn numbered_name(&self) -> &'static str {
                match *self {
                    $(
                        Self::$id => $num_name
                    ),*
                }
            }

            pub const fn abbreviation(&self) -> &'static str {
                match *self {
                    $(
                        Self::$id => $abbr
                    ),*
                }
            }

            pub const fn full_name(&self) -> &'static str {
                match *self {
                    $(
                        Self::$id => $full_name
                    ),*
                }
            }
        }

        impl From<GameId> for u8 {
            fn from(value: GameId) -> Self {
                match value {
                    $(
                        GameId::$id => $id_num
                    ),*
                }
            }
        }

        impl TryFrom<u8> for GameId {
            type Error = InvalidGameId;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $id_num => Ok(Self::$id),
                    )*
                    v => Err(InvalidGameId(v)),
                }
            }
        }
    };
}

define_game_info! {
    PCB: {
        id_number: 7,
        numbered_name: "Touhou 7",
        abbreviation: "PCB",
        full_name: "Perfect Cherry Blossom"
    },
    IN: {
        id_number: 8,
        numbered_name: "Touhou 8",
        abbreviation: "IN",
        full_name: "Imperishable Night"
    }
}

impl serde::de::Expected for GameId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(self.abbreviation())
    }
}

impl From<GameId> for u16 {
    fn from(value: GameId) -> Self {
        <GameId as Into<u8>>::into(value) as u16
    }
}

impl TryFrom<u16> for GameId {
    type Error = anyhow::Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let v: u8 = value.try_into()?;
        GameId::try_from(v).map_err(|e| e.into())
    }
}

impl Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.abbreviation())
    }
}