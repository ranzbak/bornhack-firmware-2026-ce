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

    /// HAX price to buy this drink. Only Water is "healthy" (pricier).
    pub fn hax_price(self) -> u32 {
        match self {
            DrinkKind::Water => 15,
            DrinkKind::Cola | DrinkKind::Beer | DrinkKind::Wine | DrinkKind::Whiskey => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Water (the only "healthy" drink) costs 15 HAX; every other drink
    /// (non-alcoholic Cola included) costs 10.
    #[test]
    fn hax_price_matches_health_tier() {
        assert_eq!(DrinkKind::Water.hax_price(), 15);
        assert_eq!(DrinkKind::Cola.hax_price(), 10);
        assert_eq!(DrinkKind::Beer.hax_price(), 10);
        assert_eq!(DrinkKind::Wine.hax_price(), 10);
        assert_eq!(DrinkKind::Whiskey.hax_price(), 10);
    }

    #[test]
    fn only_water_and_cola_are_non_alcoholic() {
        assert!(!DrinkKind::Water.is_alcoholic());
        assert!(!DrinkKind::Cola.is_alcoholic());
        assert!(DrinkKind::Beer.is_alcoholic());
        assert!(DrinkKind::Wine.is_alcoholic());
        assert!(DrinkKind::Whiskey.is_alcoholic());
    }
}
