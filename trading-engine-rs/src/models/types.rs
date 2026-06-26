// Aliases
pub type OrderId = u64;

// Enums
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Unknown,
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Replaced,
    Rejected,
    Unknown
    // Pending
}

// Price
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(i64);
impl Price {
    pub const SCALE: i64 = 10000; // 4dp should be fine
    pub fn ticks(self) -> i64 { self.0 }

    pub fn from_int<T: Into<i64>>(value: T) -> Self {
        Self(value.into() * Self::SCALE)
    }

    pub fn from_float<T: Into<f64>>(value: T) -> Self {
        Self((value.into() * Self::SCALE as f64).round() as i64)
    }

    pub fn to_f64(self) -> f64 { // could probably do with either aliasing f64 to have set in one place, or generic impl
        self.0 as f64 / Self::SCALE as f64
    }
}

impl std::ops::Sub for Price {
    type Output = Price;
    fn sub(self, rhs: Price) -> Price {
        Price(self.0 - rhs.0)
    }
}

impl<T> From<T> for Price where T: Into<f64> {
    fn from(value: T) -> Self {
        Self::from_float(value)
    }
}