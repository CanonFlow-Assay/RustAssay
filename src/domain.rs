#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DraftLine {
    pub sku: String,
    pub quantity: u32,
    pub unit_price_cents: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Order {
    pub id: String,
    pub lines: Vec<DraftLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    BlankOrderId,
    BlankSku,
    EmptyOrder,
    ZeroQuantity,
}

pub fn create_order(id: &str, lines: &[DraftLine]) -> Result<Order, ValidationError> {
    if id.trim().is_empty() {
        return Err(ValidationError::BlankOrderId);
    }
    if lines.is_empty() {
        return Err(ValidationError::EmptyOrder);
    }

    lines
        .iter()
        .try_fold(Vec::with_capacity(lines.len()), |mut valid, line| {
            if line.sku.trim().is_empty() {
                return Err(ValidationError::BlankSku);
            }
            if line.quantity == 0 {
                return Err(ValidationError::ZeroQuantity);
            }
            valid.push(line.clone());
            Ok(valid)
        })
        .map(|lines| Order {
            id: id.to_owned(),
            lines,
        })
}

pub fn total_cents(order: &Order) -> u64 {
    order.lines.iter().fold(0, |total, line| {
        total + u64::from(line.quantity) * line.unit_price_cents
    })
}
