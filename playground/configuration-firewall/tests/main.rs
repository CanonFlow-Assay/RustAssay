#[path = "../domain/firewall.rs"]
mod firewall;

use firewall::{canonical_configuration, load_configuration};

fn main() {
    let cases: [(&str, &[(&str, &str)], &str); 6] = [
        (
            "valid",
            &[
                ("Service:Endpoint", "https://api.example.test"),
                ("Service:RetryLimit", "3"),
                ("Service:Mode", "read-only"),
            ],
            "loaded|https://api.example.test/|3|ReadOnly",
        ),
        (
            "missing",
            &[
                ("Service:Endpoint", "https://api.example.test"),
                ("Service:RetryLimit", "3"),
            ],
            "rejected|missing:Service:Mode",
        ),
        (
            "bad-uri",
            &[
                ("Service:Endpoint", "not-a-uri"),
                ("Service:RetryLimit", "3"),
                ("Service:Mode", "write"),
            ],
            "rejected|invalid-uri:Service:Endpoint",
        ),
        (
            "bad-limit",
            &[
                ("Service:Endpoint", "https://api.example.test"),
                ("Service:RetryLimit", "0"),
                ("Service:Mode", "write"),
            ],
            "rejected|invalid-range:Service:RetryLimit",
        ),
        (
            "bad-mode",
            &[
                ("Service:Endpoint", "https://api.example.test"),
                ("Service:RetryLimit", "3"),
                ("Service:Mode", "admin"),
            ],
            "rejected|invalid-mode:Service:Mode",
        ),
        (
            "multiple-issues",
            &[
                ("Service:Endpoint", "ftp://bad.example.test"),
                ("Service:RetryLimit", "99"),
            ],
            "rejected|invalid-uri:Service:Endpoint;invalid-range:Service:RetryLimit;missing:Service:Mode",
        ),
    ];

    for (name, entries, expected) in &cases {
        let actual = canonical_configuration(&load_configuration(entries));
        if actual != *expected {
            eprintln!("{name}: expected {expected}, got {actual}");
            std::process::exit(1);
        }
    }
    println!("Rust configuration firewall: {} cases passed.", cases.len());
}
