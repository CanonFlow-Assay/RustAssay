#[path = "../src/domain.rs"]
mod domain;

use domain::{DraftLine, ValidationError, create_order, total_cents};

#[test]
fn valid_order_has_a_deterministic_total() {
    let order = create_order(
        "order-42",
        &[DraftLine {
            sku: "book".into(),
            quantity: 2,
            unit_price_cents: 1_250,
        }],
    )
    .expect("valid test input");

    assert_eq!(total_cents(&order), 2_500);
}

#[test]
fn expected_validation_failure_is_a_value() {
    let outcome = create_order(" ", &[]);
    assert_eq!(outcome, Err(ValidationError::BlankOrderId));
}
