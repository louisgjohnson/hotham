use hotham::components::hand::Handedness;

#[derive(Debug, Clone, PartialEq, Copy, PartialOrd)]
pub enum Colour {
    Red,
    Blue,
}

impl From<Handedness> for Colour {
    fn from(handedness: Handedness) -> Self {
        match handedness {
            Handedness::Left => Colour::Red,
            Handedness::Right => Colour::Blue,
        }
    }
}

impl Into<Handedness> for Colour {
    fn into(self) -> Handedness {
        match self {
            Colour::Red => Handedness::Left,
            Colour::Blue => Handedness::Right,
        }
    }
}
