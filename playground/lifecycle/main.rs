extern crate expense;
use expense::{Decision, decide};
struct Case {
    id: &'static str,
    amount: i64,
    currency: &'static str,
    memo: &'static str,
    limit: i64,
    expected: Decision,
}
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let count = match args.as_slice() {
        [_, mode] if mode == "baseline" => 12,
        [_, mode] if mode == "expanded" => 22,
        _ => return std::process::ExitCode::from(64),
    };
    let cases = [
        Case {
            id: "approved",
            amount: 2500,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Approved(2500),
        },
        Case {
            id: "negative",
            amount: -1,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Invalid("amount"),
        },
        Case {
            id: "over-limit",
            amount: 10001,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::NeedsReview(10001),
        },
        Case {
            id: "currency",
            amount: 2500,
            currency: "EUR",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Invalid("currency"),
        },
        Case {
            id: "empty-memo",
            amount: 2500,
            currency: "USD",
            memo: "",
            limit: 10000,
            expected: Decision::Invalid("memo"),
        },
        Case {
            id: "zero-policy",
            amount: 2500,
            currency: "USD",
            memo: "lunch",
            limit: 0,
            expected: Decision::Invalid("policy"),
        },
        Case {
            id: "zero-amount",
            amount: 0,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Invalid("amount"),
        },
        Case {
            id: "exact-limit",
            amount: 10000,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Approved(10000),
        },
        Case {
            id: "smallest-amount",
            amount: 1,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::Approved(1),
        },
        Case {
            id: "blank-memo",
            amount: 2500,
            currency: "USD",
            memo: " \t",
            limit: 10000,
            expected: Decision::Invalid("memo"),
        },
        Case {
            id: "negative-policy",
            amount: 2500,
            currency: "USD",
            memo: "lunch",
            limit: -1,
            expected: Decision::Invalid("policy"),
        },
        Case {
            id: "large-amount",
            amount: 2147483647,
            currency: "USD",
            memo: "lunch",
            limit: 10000,
            expected: Decision::NeedsReview(2147483647),
        },
        Case {
            id: "memo-200-ascii",
            amount: 2500,
            currency: "USD",
            memo: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            limit: 10000,
            expected: Decision::Approved(2500),
        },
        Case {
            id: "memo-201-ascii",
            amount: 2500,
            currency: "USD",
            memo: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            limit: 10000,
            expected: Decision::Invalid("memo-too-long"),
        },
        Case {
            id: "memo-200-astral",
            amount: 2500,
            currency: "USD",
            memo: "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀",
            limit: 10000,
            expected: Decision::Approved(2500),
        },
        Case {
            id: "memo-201-astral",
            amount: 2500,
            currency: "USD",
            memo: "😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀",
            limit: 10000,
            expected: Decision::Invalid("memo-too-long"),
        },
        Case {
            id: "memo-200-combining",
            amount: 2500,
            currency: "USD",
            memo: "éééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééé",
            limit: 10000,
            expected: Decision::Approved(2500),
        },
        Case {
            id: "memo-202-combining",
            amount: 2500,
            currency: "USD",
            memo: "ééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééé",
            limit: 10000,
            expected: Decision::Invalid("memo-too-long"),
        },
        Case {
            id: "long-blank-precedence",
            amount: 2500,
            currency: "USD",
            memo: "                                                                                                                                                                                                         ",
            limit: 10000,
            expected: Decision::Invalid("memo"),
        },
        Case {
            id: "bad-policy-precedence",
            amount: 2500,
            currency: "USD",
            memo: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            limit: 0,
            expected: Decision::Invalid("policy"),
        },
        Case {
            id: "bad-currency-precedence",
            amount: 2500,
            currency: "EUR",
            memo: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            limit: 10000,
            expected: Decision::Invalid("currency"),
        },
        Case {
            id: "bad-amount-precedence",
            amount: -1,
            currency: "USD",
            memo: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            limit: 10000,
            expected: Decision::Invalid("amount"),
        },
    ];
    let failures = cases
        .iter()
        .take(count)
        .map(|case| {
            let passed = decide(case.amount, case.currency, case.memo, case.limit) == case.expected;
            println!(
                "CASE\t{}\t{}",
                case.id,
                if passed { "PASS" } else { "FAIL" }
            );
            usize::from(!passed)
        })
        .sum::<usize>();
    std::process::ExitCode::from(u8::from(failures > 0))
}
