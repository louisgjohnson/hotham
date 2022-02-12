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
