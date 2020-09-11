use std::collections::HashMap;
use std::rc::Rc;

use regex_syntax;
use regex_syntax::hir::GroupKind as LibGroup;
use regex_syntax::hir::HirKind as LibHir;
use regex_syntax::hir::RepetitionKind as LibRepKind;
use regex_syntax::hir::RepetitionRange as LibRepRange;

use super::super::automaton::atom::Atom;
use super::super::automaton::Label;
use super::super::mapping::{Marker, Variable};

/// A simple Hir, with branchements of arity at most 2 and at little redundancy
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
    pub fn from_regex(regex: &str, raw: bool) -> Hir {
        let (anchor_begin, anchor_end, regex) = if raw {
            (true, true, regex.to_string())
        } else {
            Hir::reformat(regex)
        };

        let mut variables = HashMap::new();

        let lib_hir = regex_syntax::ParserBuilder::new()
            .dot_matches_new_line(true)
            .build()
            .parse(&regex)
            .expect("Invalid regexp syntax");
        let hir = Hir::from_lib_hir(lib_hir, &mut variables);

        if raw {
            return hir;
        }

        let hir = match variables.len() {
            0 => {
                let var = Rc::new(Variable::new("match".to_string(), 0));
                let marker_open = Label::Assignation(Marker::Open(var.clone()));
                let marker_close = Label::Assignation(Marker::Close(var));

                Hir::concat(
                    Hir::Concat(Box::new(Hir::label(marker_open)), Box::new(hir)),
                    Hir::label(marker_close),
                )
            }
            _ => hir,
        };

        let any = match regex_syntax::hir::Hir::any(false).into_kind() {
            LibHir::Class(x) => x,
            _ => panic!("LibHir broken!"),
        };

        let hir = match anchor_begin {
            true => hir,
            false => Hir::concat(
                Hir::option(Hir::closure(Hir::label(Label::Atom(Atom::Class(
                    any.clone(),
                ))))),
                hir,
            ),
        };

        match anchor_end {
            true => hir,
            false => Hir::concat(
                hir,
                Hir::option(Hir::closure(Hir::label(Label::Atom(Atom::Class(any))))),
            ),
        }
    }

    /// Construct an Hir from regex_syntax's Hir format.
    ///
    /// It also takes as an input the counter of already created variables and
    /// return the count of variables that have been created in the generated
    /// Hir.
    fn from_lib_hir(
        hir: regex_syntax::hir::Hir,
        variables: &mut HashMap<String, Rc<Variable>>,
    ) -> Hir {
        match hir.into_kind() {
            LibHir::Empty => Hir::epsilon(),

            LibHir::Literal(lit) => Hir::label(Label::Atom(Atom::Literal(lit))),

            LibHir::Class(class) => Hir::label(Label::Atom(Atom::Class(class))),

            LibHir::Repetition(rep) => {
                let hir = Hir::from_lib_hir(*rep.hir, variables);
                let new_hir = match rep.kind {
                    LibRepKind::ZeroOrOne => Hir::option(hir),
                    LibRepKind::ZeroOrMore => Hir::option(Hir::closure(hir)),
                    LibRepKind::OneOrMore => Hir::closure(hir),
                    LibRepKind::Range(range) => Hir::repetition(hir, range),
                };
                new_hir
            }

            LibHir::Group(group) => {
                let subtree = Hir::from_lib_hir(*group.hir, variables);
                let new_hir = match group.kind {
                    LibGroup::NonCapturing | LibGroup::CaptureIndex(_) => subtree,
                    LibGroup::CaptureName { name, index: _ } => {
                        let real_name = match name.find("__") {
                            None => name.clone(),
                            Some(i) => name[0..i].to_string(),
                        };

                        let var =
                            variables
                                .get(&real_name)
                                .map(|v| v.clone())
                                .unwrap_or_else(|| {
                                    let x =
                                        Rc::new(Variable::new(real_name.clone(), variables.len()));
                                    variables.insert(real_name, x.clone());

                                    x
                                });

                        let marker_open = Label::Assignation(Marker::Open(var.clone()));
                        let marker_close = Label::Assignation(Marker::Close(var));

                        Hir::concat(
                            Hir::Concat(Box::new(Hir::label(marker_open)), Box::new(subtree)),
                            Hir::label(marker_close),
                        )
                    }
                };

                new_hir
            }

            LibHir::Concat(sub) => sub.into_iter().fold(Hir::epsilon(), |acc, branch| {
                let add_hir = Hir::from_lib_hir(branch, variables);
                Hir::concat(acc, add_hir)
            }),

            LibHir::Alternation(sub) => sub.into_iter().fold(Hir::Empty, |acc, branch| {
                let add_hir = Hir::from_lib_hir(branch, variables);
                Hir::alternation(acc, add_hir)
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
                // of the input langage is replaced with a closure. It saves
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

    fn reformat(regex: &str) -> (bool, bool, String) {
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

        (anchor_begin, anchor_end, regex)
    }
}
