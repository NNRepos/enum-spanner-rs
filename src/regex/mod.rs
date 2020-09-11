mod glushkov;
mod parse;

use super::automaton::Automaton;

pub fn compile(regex: &str) -> Automaton {
    let hir = parse::Hir::from_regex(&regex, false);

    glushkov::LocalLang::from_hir(hir, 0).into_automaton()
}

pub fn compile_raw(regex: &str) -> Automaton {
    let hir = parse::Hir::from_regex(&regex, true);

    glushkov::LocalLang::from_hir(hir, 0).into_automaton()
}

#[cfg(test)]
pub fn is_match(regex: &str, text: &str) -> bool {
    let automaton = compile(&regex);
    let matches = compile_matches(automaton, text, 1);

    let ret = matches.iter().next().is_some();
    ret
}

#[cfg(test)]
mod tests;
