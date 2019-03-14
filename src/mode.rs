use std::fmt;
use std::str::FromStr;
use crate::errors::TarotErrorKind;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, EnumIter)]
pub enum Mode {
    Three = 3,
    Four = 4,
    Five = 5
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Three => write!(f, "{} players, 1 vs 2", Mode::Three as usize),
            Mode::Four  => write!(f, "{} players, 1 vs 3", Mode::Four as usize),
            Mode::Five  => write!(f, "{} players, 2 vs 3", Mode::Five as usize),
        }
    }
}

impl FromStr for Mode {
    type Err = TarotErrorKind;
    fn from_str(s: &str) -> Result<Mode, TarotErrorKind> {
        match s {
            "3" => Ok(Mode::Three),
            "4" => Ok(Mode::Four),
            "5" => Ok(Mode::Five),
            _ => Err(TarotErrorKind::InvalidPlayers),
        }
    }
}

impl Mode {
    pub fn dog_size(self) -> usize {
        match self {
            Mode::Five => 3,
            _ => 6
        }
    }
    pub fn cards_per_turn(self) -> usize {
        match self {
            Mode::Three=> 4,
            _ => 3
        }
    }
    pub fn cards_per_player(self) -> usize {
        match self {
            Mode::Three => 24,
            Mode::Four  => 18,
            Mode::Five  => 15,
        }
    }
}

impl Default for Mode {
    fn default() -> Mode { Mode::Four }
}

#[test]
fn mode_tests() {
    let mode = Mode::default();
    println!("mode: {}", &mode);

    let three = Mode::from_str("3");
    assert!(three == Ok(Mode::Three));

    let four = Mode::from_str("4");
    assert!(four == Ok(Mode::Four));

    let five = Mode::from_str("5");
    assert!(five == Ok(Mode::Five));
}
