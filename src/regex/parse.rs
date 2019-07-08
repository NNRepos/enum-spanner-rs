use std::rc::Rc;

use regex_syntax;
use regex_syntax::hir::GroupKind as LibGroup;
use regex_syntax::hir::HirKind as LibHir;
use regex_syntax::hir::RepetitionKind as LibRepKind;
use regex_syntax::hir::RepetitionRange as LibRepRange;

use super::super::automaton::atom::Atom;
use super::super::automaton::Label;
use super::super::mapping::{Marker, Variable};

/// A simple Hir, with branchements of arity at most 2 and at least redondancy
/// as possible.
#[derive(Clone, Debug)]
pub enum Hir {
    /// Empty langage
    Empty,
    /// Langage of words of length 1
    Label(Rc<Label>), // embeded into an Rc to avoid duplicating heavy complex literals
    /// Concatenation of two langages
    Concat(Box<Hir>, Box<Hir>),
    /// Union of two langages
    Alternation(Box<Hir>, Box<Hir>),
    /// Either epsilon, either a word of the langage
    Option(Box<Hir>),
    /// Langage of repetitions of **at least** one word of the input langage
    Closure(Box<Hir>),
}

impl Hir {
    pub fn from_regex(regex: &str) -> Hir {
        let lib_hir = regex_syntax::Parser::new()
            .parse(regex)
            .expect("Invalid regexp syntax");
        let (_, hir) = Hir::from_lib_hir(lib_hir, 0);
        hir
    }

    /// Construct an Hir from regex_syntax's Hir format.
    ///
    /// It also takes as an input the counter of already created variables and
    /// return the count of variables that have been created in the generated
    /// Hir.
    fn from_lib_hir(hir: regex_syntax::hir::Hir, nb_ext_vars: u64) -> (u64, Hir) {
        match hir.into_kind() {
            LibHir::Empty => (0, Hir::epsilon()),

            LibHir::Literal(lit) => (0, Hir::label(Label::Atom(Atom::Literal(lit)))),

            LibHir::Class(class) => (0, Hir::label(Label::Atom(Atom::Class(class)))),

            LibHir::Repetition(rep) => {
                let (nb_in_vars, hir) = Hir::from_lib_hir(*rep.hir, nb_ext_vars);
                let new_hir = match rep.kind {
                    LibRepKind::ZeroOrOne => Hir::option(hir),
                    LibRepKind::ZeroOrMore => Hir::option(Hir::closure(hir)),
                    LibRepKind::OneOrMore => Hir::closure(hir),
                    LibRepKind::Range(range) => Hir::repetition(hir, range),
                };
                (nb_in_vars, new_hir)
            }

            LibHir::Group(group) => {
                let (mut nb_in_vars, subtree) = Hir::from_lib_hir(*group.hir, nb_ext_vars);
                let new_hir = match group.kind {
                    LibGroup::NonCapturing | LibGroup::CaptureIndex(_) => subtree,
                    LibGroup::CaptureName { name, index: _ } => {
                        let var = Rc::new(Variable::new(name, nb_ext_vars + nb_in_vars));
                        let marker_open = Label::Assignation(Marker::Open(var.clone()));
                        let marker_close = Label::Assignation(Marker::Close(var));
                        nb_in_vars += 1;

                        Hir::concat(
                            Hir::Concat(Box::new(Hir::label(marker_open)), Box::new(subtree)),
                            Hir::label(marker_close),
                        )
                    }
                };

                (nb_in_vars, new_hir)
            }

            LibHir::Concat(sub) => sub.into_iter().fold((0, Hir::epsilon()), |acc, branch| {
                let (acc_vars, acc_hir) = acc;
                let (add_vars, add_hir) = Hir::from_lib_hir(branch, nb_ext_vars + acc_vars);
                (acc_vars + add_vars, Hir::concat(acc_hir, add_hir))
            }),

            LibHir::Alternation(sub) => sub.into_iter().fold((0, Hir::Empty), |acc, branch| {
                let (acc_vars, acc_hir) = acc;
                let (add_vars, add_hir) = Hir::from_lib_hir(branch, nb_ext_vars + acc_vars);
                (acc_vars + add_vars, Hir::alternation(acc_hir, add_hir))
            }),

            other => panic!("Not implemented: {:?}", other),
        }
    }

    fn epsilon() -> Hir {
        Hir::option(Hir::Empty)
    }

    fn label(label: Label) -> Hir {
        Hir::Label(Rc::new(label))
    }

    fn option(hir: Hir) -> Hir {
        Hir::Option(Box::new(hir))
    }

    fn concat(hir1: Hir, hir2: Hir) -> Hir {
        Hir::Concat(Box::new(hir1), Box::new(hir2))
    }

    fn alternation(hir1: Hir, hir2: Hir) -> Hir {
        Hir::Alternation(Box::new(hir1), Box::new(hir2))
    }

    fn closure(hir: Hir) -> Hir {
        Hir::Closure(Box::new(hir))
    }

    fn repetition(hir: Hir, range: LibRepRange) -> Hir {
        let (min, max) = match range {
            LibRepRange::Exactly(n) => (n, Some(n)),
            LibRepRange::AtLeast(n) => (n, None),
            LibRepRange::Bounded(m, n) => (m, Some(n)),
        };

        let mut result = Hir::epsilon();

        for i in 0..min {
            if i == min - 1 && max == None {
                // If the repetition has no upper bound, the last repetition
                // of the input langage is replaced with a closure. It avoids
                // a few states to do it here.
                result = Hir::concat(result, Hir::closure(hir.clone()));
            } else {
                result = Hir::concat(result, hir.clone());
            }
        }

        if let Some(max) = max {
            let mut optionals = Hir::epsilon();

            for _ in min..max {
                optionals = Hir::option(Hir::concat(hir.clone(), optionals));
            }

            result = Hir::concat(result, optionals);
        }

        result
    }
}
