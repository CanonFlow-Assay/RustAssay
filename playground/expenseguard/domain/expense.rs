#[derive(Debug, Eq, PartialEq)]
pub enum Decision {
    Approved(i64),
    NeedsReview(i64),
    Invalid(&'static str),
}

pub fn decide(amount: i64, currency: &str, memo: &str, limit: i64) -> Decision {
    if limit <= 0 {
        Decision::Invalid("policy")
    } else if amount <= 0 {
        Decision::Invalid("amount")
    } else if currency != "USD" {
        Decision::Invalid("currency")
    } else if memo.trim().is_empty() {
        Decision::Invalid("memo")
    } else if memo.chars().count() > 200 {
        Decision::Invalid("memo-too-long")
    } else if amount > limit {
        Decision::NeedsReview(amount)
    } else {
        Decision::Approved(amount)
    }
}
