/// Implementation of Glushkov's algorithm to build a
/// linearized language out of a regexp's HIR, and finaly convert this
/// expression to a variable NFA.
use std::collections::LinkedList;
use std::rc::Rc;

use super::super::automaton::Automaton;
use super::super::automaton::Label;
use super::parse::Hir;

#[derive(Clone, Debug)]
pub struct GlushkovFactors {
    /// The list of terms that are a prefix of a word of language.
    p: LinkedList<GlushkovTerm>,
    /// The list of terms that are a suffix of a word of language.
    d: LinkedList<GlushkovTerm>,
    /// The list of factors of size two that appear in a word of the language.
    f: LinkedList<(GlushkovTerm, GlushkovTerm)>, //
    /// Wether the empty word belongs two the language or not.
    g: bool,
}

/// A linearized (distinguishable from others in the input regex) label, the
/// language of words over this alphabet is local, however if it is unlinearized
/// it is possible to get back to the original regular language.
#[derive(Clone, Debug)]
pub struct GlushkovTerm {
    id: usize,
    label: Rc<Label>,
}

#[derive(Clone, Debug)]
pub struct LocalLang {
    nb_terms: usize,
    factors: GlushkovFactors,
}

/// A local language is a regular language that can be identified with only its
/// factors of size 2, its prefixes and suffixes and wether it contains the
/// empty word or not.
impl LocalLang {
    /// Create an automaton that recognise the same langage.
    pub fn into_automaton(self) -> Automaton {
        let iner_transitions = self
            .factors
            .f
            .into_iter()
            .map(|(source, target)| (source.id + 1, target.label, target.id + 1));
        let pref_transitions = self
            .factors
            .p
            .into_iter()
            .map(|target| (0, target.label, target.id + 1));

        let transitions = iner_transitions.chain(pref_transitions);
        let mut finals: Vec<usize> = self.factors.d.into_iter().map(|x| x.id + 1).collect();

        if self.factors.g {
            finals.push(0);
        }

        Automaton::new(self.nb_terms + 1, transitions, finals.into_iter())
    }

    /// Return a language representing the input Hir.
    pub fn from_hir(hir: Hir, id_offset: usize) -> LocalLang {
        match hir {
            Hir::Empty => LocalLang::empty(),
            Hir::Label(label) => LocalLang::label(label, id_offset),
            Hir::Concat(hir1, hir2) => {
                let lang1 = LocalLang::from_hir(*hir1, id_offset);
                let lang2 = LocalLang::from_hir(*hir2, id_offset + lang1.nb_terms);
                LocalLang::concatenation(lang1, lang2)
            }
            Hir::Alternation(hir1, hir2) => {
                let lang1 = LocalLang::from_hir(*hir1, id_offset);
                let lang2 = LocalLang::from_hir(*hir2, id_offset + lang1.nb_terms);
                LocalLang::alternation(lang1, lang2)
            }
            Hir::Option(hir) => LocalLang::optional(LocalLang::from_hir(*hir, id_offset)),
            Hir::Closure(hir) => LocalLang::closure(LocalLang::from_hir(*hir, id_offset)),
        }
    }

    /// Register a new atom in the local language and return the associated
    /// term.
    fn register_label(&mut self, label: Rc<Label>, id_offset: usize) -> GlushkovTerm {
        self.nb_terms += 1;
        GlushkovTerm {
            id: self.nb_terms + id_offset - 1,
            label,
        }
    }

    /// Return a local language representing an expression containing a single
    /// term.
    fn label(label: Rc<Label>, id_offset: usize) -> LocalLang {
        let mut lang = LocalLang::empty();
        let term = lang.register_label(label, id_offset);
        lang.factors.p.push_back(term.clone());
        lang.factors.d.push_back(term);
        lang
    }

    /// Return an empty local language.
    fn empty() -> LocalLang {
        LocalLang {
            nb_terms: 0,
            factors: GlushkovFactors {
                p: LinkedList::new(),
                d: LinkedList::new(),
                f: LinkedList::new(),
                g: false,
            },
        }
    }

    /// Return a local language containing the concatenation of words from the
    /// first and second input languages.
    fn concatenation(lang1: LocalLang, lang2: LocalLang) -> LocalLang {
        let nb_terms = lang1.nb_terms + lang2.nb_terms;
        let mut factors = GlushkovFactors {
            p: lang1.factors.p,
            d: lang2.factors.d,
            f: lang1.factors.f,
            g: lang1.factors.g && lang2.factors.g,
        };

        {
            let mut owned_lang2_factors = lang2.factors.f;
            factors.f.append(&mut owned_lang2_factors);
        }

        for x in &lang1.factors.d {
            for y in &lang2.factors.p {
                factors.f.push_back((x.clone(), y.clone()));
            }
        }

        if lang1.factors.g {
            let mut owned_lang2_p = lang2.factors.p;
            factors.p.append(&mut owned_lang2_p);
        }

        if lang2.factors.g {
            let mut owned_lang1_d = lang1.factors.d;
            factors.d.append(&mut owned_lang1_d);
        }

        LocalLang { nb_terms, factors }
    }

    /// Return a local language containing words from the first or the second
    /// input languages.
    fn alternation(lang1: LocalLang, mut lang2: LocalLang) -> LocalLang {
        let nb_terms = lang1.nb_terms + lang2.nb_terms;
        let mut factors = lang1.factors;

        factors.p.append(&mut lang2.factors.p);
        factors.d.append(&mut lang2.factors.d);
        factors.f.append(&mut lang2.factors.f);
        factors.g = factors.g || lang2.factors.g;

        LocalLang { nb_terms, factors }
    }

    /// Return a local language containing the empty word and the input
    /// language.
    fn optional(mut lang: LocalLang) -> LocalLang {
        lang.factors.g = true;
        lang
    }

    /// Return a local language containing words made of one or more repetitions
    /// of words of the input language.
    fn closure(mut lang: LocalLang) -> LocalLang {
        for x in &lang.factors.d {
            for y in &lang.factors.p {
                lang.factors.f.push_back((x.clone(), y.clone()));
            }
        }

        lang
    }
}
