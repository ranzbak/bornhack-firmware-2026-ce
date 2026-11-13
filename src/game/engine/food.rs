//! Food choices for the Feed action. What each food does is in `doses`.

/// Which food the player picked for the current Feed action.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "embassy-core", derive(defmt::Format))]
pub enum FoodKind {
    Salad,
    Apple,
    Frikandel,
    Pizza,
    Cake,
}

impl FoodKind {
    pub const ALL: [FoodKind; 5] = [
        FoodKind::Salad,
        FoodKind::Apple,
        FoodKind::Frikandel,
        FoodKind::Pizza,
        FoodKind::Cake,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FoodKind::Salad => "Salad",
            FoodKind::Apple => "Apple",
            FoodKind::Frikandel => "Frikandel spec",
            FoodKind::Pizza => "Pizza",
            FoodKind::Cake => "Cake",
        }
    }

    /// The food's whole name, for the menu title (`label` is the short form
    /// that fits a row with its price).
    pub fn full_name(self) -> &'static str {
        match self {
            FoodKind::Frikandel => "Frikandel speciaal",
            other => other.label(),
        }
    }

    /// HAX price to feed this food. Healthy costs more than junk.
    /// In Hard (US) mode healthy food is pricier still (20 vs 15);
    /// unhealthy food is unaffected by `hard`.
    pub fn hax_price(self, hard: bool) -> u32 {
        match self {
            FoodKind::Salad | FoodKind::Apple => {
                if hard {
                    20
                } else {
                    15
                }
            }
            FoodKind::Frikandel | FoodKind::Pizza | FoodKind::Cake => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Healthy foods (Salad, Apple) cost 15 HAX in normal mode; unhealthy
    /// ones (Burger, Pizza, Cake) cost 10 — junk food is the cheap trap.
    #[test]
    fn hax_price_matches_health_tier() {
        assert_eq!(FoodKind::Salad.hax_price(false), 15);
        assert_eq!(FoodKind::Apple.hax_price(false), 15);
        assert_eq!(FoodKind::Frikandel.hax_price(false), 10);
        assert_eq!(FoodKind::Pizza.hax_price(false), 10);
        assert_eq!(FoodKind::Cake.hax_price(false), 10);
    }

    /// In Hard (US) mode, healthy food (Salad, Apple) jumps to 20 HAX —
    /// 2x the unhealthy price. Unhealthy food (Burger, Pizza, Cake) is
    /// unaffected by hard mode.
    #[test]
    fn hax_price_hard_mode_raises_healthy_food_only() {
        assert_eq!(FoodKind::Salad.hax_price(true), 20);
        assert_eq!(FoodKind::Apple.hax_price(true), 20);
        assert_eq!(FoodKind::Frikandel.hax_price(true), 10);
        assert_eq!(FoodKind::Pizza.hax_price(true), 10);
        assert_eq!(FoodKind::Cake.hax_price(true), 10);
    }
}
