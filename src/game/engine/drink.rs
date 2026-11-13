//! Drink choices for the Drink action. What each drink does is in `doses`.

/// Which drink the player picked for the current Drink action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum DrinkKind {
    Water,
    Cola,
    Beer,
    Wine,
    Whiskey,
}

impl DrinkKind {
    pub const ALL: [DrinkKind; 5] = [
        DrinkKind::Water,
        DrinkKind::Cola,
        DrinkKind::Beer,
        DrinkKind::Wine,
        DrinkKind::Whiskey,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DrinkKind::Water => "Water",
            DrinkKind::Cola => "Cola",
            DrinkKind::Beer => "Beer",
            DrinkKind::Wine => "Wine",
            DrinkKind::Whiskey => "Whiskey",
        }
    }

    /// Whether this drink raises `booze` at all (Water/Cola never do). Used
    /// to pick an accurate post-drink toast.
    pub fn is_alcoholic(self) -> bool {
        !matches!(self, DrinkKind::Water | DrinkKind::Cola)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_water_and_cola_are_non_alcoholic() {
        assert!(!DrinkKind::Water.is_alcoholic());
        assert!(!DrinkKind::Cola.is_alcoholic());
        assert!(DrinkKind::Beer.is_alcoholic());
        assert!(DrinkKind::Wine.is_alcoholic());
        assert!(DrinkKind::Whiskey.is_alcoholic());
    }
}
