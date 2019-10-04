pub mod naive;

mod glushkov;
mod parse;

use super::automaton::Automaton;
use super::mapping;

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
pub fn compile_matches<'t>(automaton: Automaton, text: &'t str, jump_distance: usize) -> mapping::IndexedDag<'t> {
    mapping::IndexedDag::compile(
        automaton,
        text,
        mapping::indexed_dag::ToggleProgress::Disabled,
		jump_distance,
    )
}

pub fn compile_matches_progress<'t>(
    automaton: Automaton,
    text: &'t str,
	jump_distance: usize,
) -> mapping::IndexedDag<'t> {
    mapping::IndexedDag::compile(
        automaton,
        text,
        mapping::indexed_dag::ToggleProgress::Enabled,
		jump_distance,
    )
}


#[cfg(test)]
mod tests;
