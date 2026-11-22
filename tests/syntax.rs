use rust_assay::inspect;

#[test]
fn ignores_comments_literals_patterns_and_similar_names() {
    let source = r##"
        //! .unwrap() std::env::var("CONFIG")
        /* std::net::TcpStream::connect("localhost") */
        use core::time::Duration;
        use crate::std::env as local_env;
        fn decide(values: &[i32], index: usize) -> Option<&i32> {
            let _ = "panic!(\"bad\") and values[index]";
            let _ = r#"println!("std::fs") .expect("value") std::time"#;
            let _ = br#"unimplemented!() std::process::exit(1)"#;
            let _ = [0; 4];
            let _ = values.first().copied().unwrap_or_default();
            if let [first, ..] = values { let _ = first; }
            custom_tokens!(values[index] std::env);
            values.get(index)
        }
    "##;
    assert!(inspect("pure.rs", source).expect("valid syntax").is_empty());
}

#[test]
fn recognizes_spaced_syntax_grouped_imports_and_multiple_occurrences() {
    let source = r#"use { std :: { env as config, time :: SystemTime, fs }, other::time };
fn decide(values: &mut [u8], i: usize) {
    let _ = ::std
        :: time :: Instant :: now ();
    let _ = std :: process :: id();
    let _ = Some(1)
        . r#unwrap
        ();
    let _ = Some(2). expect ("value"); let _ = Some(3).unwrap();
    core :: panic ! { "bad" };
    eprintln ! [ "bad" ];
    let _ = option_env ! ("BUILD_MODE");
    values
         [ i ] = values [ 0 ];
}
"#;
    let findings = inspect("violations.rs", source).expect("valid syntax");
    let observed = findings
        .iter()
        .map(|finding| (finding.rule_id, finding.line))
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        [
            ("RSA-F004", 1),
            ("RSA-F004", 1),
            ("RSA-F003", 1),
            ("RSA-F004", 3),
            ("RSA-F003", 5),
            ("RSA-F001", 7),
            ("RSA-F001", 9),
            ("RSA-F001", 9),
            ("RSA-F002", 10),
            ("RSA-F003", 11),
            ("RSA-F004", 12),
            ("RSA-F005", 14),
            ("RSA-F005", 14),
        ]
    );
}

#[test]
fn reports_one_based_character_columns_inside_standard_macro_arguments() {
    let source =
        "fn f(a: &[u8], i: usize) { let café = 0; assert_eq!(a[i], café); let _ = vec![a[i]; 2]; }";
    let indices = source
        .match_indices("[i]")
        .map(|(offset, _)| source[..offset].chars().count() + 1)
        .collect::<Vec<_>>();
    let findings = inspect("indices.rs", source).expect("valid syntax");
    assert_eq!(findings.len(), 2);
    for (finding, column) in findings.iter().zip(indices) {
        assert_eq!(
            (finding.rule_id, finding.line, finding.column),
            ("RSA-F005", 1, column)
        );
    }
}

#[test]
fn qualified_environment_macro_is_reported_once() {
    let source = r#"const MODE: &str = std::env!("BUILD_MODE");"#;
    let findings = inspect("config.rs", source).expect("valid syntax");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "RSA-F004");
}
