#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    Accepted { quantity: u32 },
    Rejected { reason: &'static str },
}

pub fn decide(quantity: u32) -> Decision {
    if quantity == 0 {
        Decision::Rejected { reason: "quantity must be positive" }
    } else {
        Decision::Accepted { quantity }
    }
}
