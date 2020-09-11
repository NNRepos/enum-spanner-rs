use std::collections::HashSet;

use super::super::automaton::Automaton;
use super::super::regex;
use super::{naive, Mapping};

/// Build a HashSet collecting results of naive algorithm.
fn naive_results<'t>(regex: &Automaton, text: &'t str) -> HashSet<Mapping<'t>> {
    naive::NaiveEnum::new(regex, text).collect()
}

/// Build a HashSet collecting results of default algorithm.
fn default_results<'t>(regex: &Automaton, text: &'t str) -> HashSet<Mapping<'t>> {
    regex::compile_matches(regex.clone(), text, 1)
        .iter()
        .collect()
}

#[test]
fn block_a() {
    let regex = regex::compile(r"^(.*[^a])?(?P<block_a>a+)([^a].*)?$");
    let texts = ["a", "aaaaaaaaaaaaa", "bbbabb", "aaaabbaaababbbb"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}

#[test]
fn sep_email() {
    let regex = regex::compile(r"\w+@\w+");
    let texts = ["a bba a@b b@a aaa@bab abbababaa@@@babbabb"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}

#[test]
fn substrings() {
    let regex = regex::compile(r".*");
    let texts = ["abcdefghijklmnopqrstuvwxyz"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}

#[test]
fn ordered_blocks() {
    let regex =
        regex::compile(r"^(.*[^a])?(?P<block_a>a+)([^a].*[^b]|[^ab])?(?P<block_b>b+)([^b].*)?$");
    let texts = ["ab", "aaaabbbb", "bbbaaababaaaaaabbbbabbbababbababbabb"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}

#[test]
fn mixed_emails() {
    let regex = regex::compile(r"(?P<login>\w+(\.\w+)*)@(?P<server>\w+\.\w+)");
    let texts = ["aaaa@aaa.aa", "aa@aa a@a.a@a.a.a@a.a.a.a@a.a.a.a.a"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}

#[test]
fn some_utf8() {
    let regex = regex::compile(r"e{3}|ê{3}");
    let texts = ["êêeeeêê", "êê", "êêêêê", "eêêêeêêêe", "eeeêeee", "eeeêêeee"];

    for text in texts.into_iter() {
        assert_eq!(naive_results(&regex, text), default_results(&regex, text));
    }
}
