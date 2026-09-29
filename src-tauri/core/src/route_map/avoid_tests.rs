//! Tests for the avoid value rules.

use super::avoid::{merge_options, normalise_avoid, toll_options};

#[test]
fn normalise_accepts_tolls_values_and_sorts_them() {
    let got = normalise_avoid(vec!["svk:tolls".into(), " CZE:tolls ".into(), "svk:tolls".into()]).unwrap();
    assert_eq!(got, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

#[test]
fn normalise_accepts_an_empty_list() {
    assert_eq!(normalise_avoid(vec![]).unwrap(), Vec::<String>::new());
}

#[test]
fn normalise_rejects_other_avoid_types() {
    // `highways` and `country` are real Sygic values, but this app offers
    // `tolls` only (01-task.md, decision 1).
    for bad in ["cze:highways", "cze:country", "tolls", "cz:tolls", "cze:tolls|svk:tolls", "cze:tolls&key=x", ""] {
        let err = normalise_avoid(vec![bad.to_string()]).expect_err(bad);
        assert!(err.contains("avoid"), "error should name the avoid value, got: {err}");
    }
}

#[test]
fn toll_options_keeps_only_tolls_and_adds_the_requested_values() {
    // Response for BA -> Brno with avoid=cze:tolls: `cze:tolls` is gone from
    // what Sygic offers, but the user still needs the checkbox to uncheck it.
    let possible = ["svk:highways", "svk:tolls", "svk:country", "cze:highways", "cze:country"];
    let got = toll_options(possible, &["cze:tolls".to_string()]);
    assert_eq!(got, vec!["cze:tolls".to_string(), "svk:tolls".to_string()]);
}

#[test]
fn merge_options_is_a_sorted_union() {
    let a = vec!["svk:tolls".to_string()];
    let b = vec!["cze:tolls".to_string(), "svk:tolls".to_string()];
    assert_eq!(
        merge_options([a.as_slice(), b.as_slice()]),
        vec!["cze:tolls".to_string(), "svk:tolls".to_string()]
    );
}
