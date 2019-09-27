pub mod naive;

mod glushkov;
mod parse;

use super::automaton::Automaton;
use super::mapping;

pub fn compile(regex: &str) -> Automaton {
    let regex = reformat(regex);
    let hir = parse::Hir::from_regex(&regex);
    glushkov::LocalLang::from_hir(hir, 0).into_automaton()
}

pub fn compile_raw(regex: &str) -> Automaton {
    let hir = parse::Hir::from_regex(&regex);
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

/// Reformat the regex to get a regex matching the whole regex in a group called
/// *match*. The new regex will allow any prefix or suffix to be matched before
/// the old regex, except if the input regex contains anchors at its begining or
/// end.
fn reformat(regex: &str) -> String {
    let mut regex = String::from(regex);

    let anchor_begin = Some(&b'^') == regex.as_bytes().first();
    let anchor_end = Some(&b'$') == regex.as_bytes().last();

    // Remove anchor characters
    if anchor_begin {
        regex.remove(0);
    }

    if anchor_end {
        regex.remove(regex.len() - 1);
    }

    // TODO: add a group only when necessary.
    //       The simplest way may still be to properly handle anchors and add the
    //       group to the regex's AST.
    regex = format!(r"(?P<match>{})", regex);

    // If there is no prefix anchor, allow any prefix and suffix
    if !anchor_begin {
        regex = format!(r"(.|\s)*{}", regex);
    }

    if !anchor_end {
        regex = format!(r"{}(.|\s)*", regex);
    }

    regex
}

#[cfg(test)]
mod tests;
