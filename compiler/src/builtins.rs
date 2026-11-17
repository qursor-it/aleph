//! Builtin function table shared by the parser, checker and interpreter
//! (SPEC §5.1).
//!
//! Generic restrictions such as `sort`'s "T is primitive" are expressed as
//! [`Bound`]s on the type variables in [`BuiltinSpec::bounds`], not by
//! replacing the variables in the parameter and return types.

use crate::ast::Capability;

/// Type variables used by the generic builtins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TyVar {
    T,
    A,
    B,
}

/// A builtin signature type. This is deliberately smaller than [`crate::ast`]'s
/// type tree: builtins only need primitives, constructors and type variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    I64,
    F64,
    Bool,
    Str,
    Unit,
    List(&'static Ty),
    Opt(&'static Ty),
    Res(&'static Ty, &'static Ty),
    /// `dict K V` (ADR-0032).
    Dict(&'static Ty, &'static Ty),
    /// `json` (ADR-0033).
    Json,
    Var(TyVar),
    Fn(&'static [&'static Ty], &'static Ty),
}

/// A restriction on a builtin's type variable (SPEC §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Any of `i64`, `f64`, `bool`, `str`.
    Primitive,
    /// A primitive, a list of a primitive, or `json` (ADR-0033).
    Showable,
    /// A dict key type: `i64`, `bool` or `str` (ADR-0032).
    Key,
    /// A type `json x` accepts: `unit`, `bool`, `i64`, `f64`, `str`, `json`,
    /// `list json` or `dict str json` (ADR-0033).
    Jsonable,
}

/// Whether a builtin is pure or has observable effects (SPEC §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Effectful,
}

/// One builtin signature, effect and capability requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub params: &'static [&'static Ty],
    pub ret: &'static Ty,
    /// Restrictions on the type variables used by `params` and `ret`.
    pub bounds: &'static [(TyVar, Bound)],
    pub effect: Effect,
    pub capability: Option<Capability>,
    /// Index of the argument holding the filesystem path, when one is checked.
    pub path_arg: Option<usize>,
}

impl BuiltinSpec {
    /// Number of arguments, i.e. the expression arity of the builtin.
    pub const fn arity(&self) -> usize {
        self.params.len()
    }
}

static I64: Ty = Ty::I64;
static F64: Ty = Ty::F64;
static BOOL: Ty = Ty::Bool;
static STR: Ty = Ty::Str;
static UNIT: Ty = Ty::Unit;
static T: Ty = Ty::Var(TyVar::T);
static A: Ty = Ty::Var(TyVar::A);
static B: Ty = Ty::Var(TyVar::B);
static LIST_T: Ty = Ty::List(&T);
static LIST_A: Ty = Ty::List(&A);
static LIST_B: Ty = Ty::List(&B);
static LIST_I64: Ty = Ty::List(&I64);
static LIST_STR: Ty = Ty::List(&STR);
static OPT_T: Ty = Ty::Opt(&T);
static RES_I64_STR: Ty = Ty::Res(&I64, &STR);
static RES_F64_STR: Ty = Ty::Res(&F64, &STR);
static RES_STR_STR: Ty = Ty::Res(&STR, &STR);
static RES_UNIT_STR: Ty = Ty::Res(&UNIT, &STR);
static MAP_FN: Ty = Ty::Fn(&[&A], &B);
static FILTER_FN: Ty = Ty::Fn(&[&A], &BOOL);
static FOLD_FN: Ty = Ty::Fn(&[&B, &A], &B);
static DICT_AB: Ty = Ty::Dict(&A, &B);
static OPT_B: Ty = Ty::Opt(&B);
static JSON: Ty = Ty::Json;
static RES_JSON_STR: Ty = Ty::Res(&JSON, &STR);
static OPT_BOOL: Ty = Ty::Opt(&BOOL);
static OPT_I64: Ty = Ty::Opt(&I64);
static OPT_F64: Ty = Ty::Opt(&F64);
static OPT_STR: Ty = Ty::Opt(&STR);
static LIST_JSON: Ty = Ty::List(&JSON);
static OPT_LIST_JSON: Ty = Ty::Opt(&LIST_JSON);
static DICT_STR_JSON: Ty = Ty::Dict(&STR, &JSON);
static OPT_DICT_STR_JSON: Ty = Ty::Opt(&DICT_STR_JSON);

/// All builtins (SPEC §5.1), sorted by name.
pub static BUILTINS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "args",
        params: &[],
        ret: &LIST_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "at",
        params: &[&LIST_T, &I64],
        ret: &OPT_T,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "cat",
        params: &[&STR, &STR],
        ret: &STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "contains",
        params: &[&STR, &STR],
        ret: &BOOL,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "count",
        params: &[&STR],
        ret: &I64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "del",
        params: &[&DICT_AB, &A],
        ret: &DICT_AB,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "eprint",
        params: &[&STR],
        ret: &UNIT,
        bounds: &[],
        effect: Effect::Effectful,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "filter",
        params: &[&FILTER_FN, &LIST_A],
        ret: &LIST_A,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "float",
        params: &[&STR],
        ret: &RES_F64_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "fold",
        params: &[&FOLD_FN, &B, &LIST_A],
        ret: &B,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "get",
        params: &[&DICT_AB, &A],
        ret: &OPT_B,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "int",
        params: &[&STR],
        ret: &RES_I64_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jbool",
        params: &[&JSON],
        ret: &OPT_BOOL,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jdict",
        params: &[&JSON],
        ret: &OPT_DICT_STR_JSON,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jint",
        params: &[&JSON],
        ret: &OPT_I64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jlist",
        params: &[&JSON],
        ret: &OPT_LIST_JSON,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jnum",
        params: &[&JSON],
        ret: &OPT_F64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "join",
        params: &[&LIST_STR, &STR],
        ret: &STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jparse",
        params: &[&STR],
        ret: &RES_JSON_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "json",
        params: &[&T],
        ret: &JSON,
        bounds: &[(TyVar::T, Bound::Jsonable)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "jstr",
        params: &[&JSON],
        ret: &OPT_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "keys",
        params: &[&DICT_AB],
        ret: &LIST_A,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "len",
        params: &[&LIST_T],
        ret: &I64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "lower",
        params: &[&STR],
        ret: &STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "map",
        params: &[&MAP_FN, &LIST_A],
        ret: &LIST_B,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "print",
        params: &[&STR],
        ret: &UNIT,
        bounds: &[],
        effect: Effect::Effectful,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "push",
        params: &[&LIST_T, &T],
        ret: &LIST_T,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "put",
        params: &[&DICT_AB, &A, &B],
        ret: &DICT_AB,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "range",
        params: &[&I64, &I64],
        ret: &LIST_I64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "read",
        params: &[&STR],
        ret: &RES_STR_STR,
        bounds: &[],
        effect: Effect::Effectful,
        capability: Some(Capability::FsRead),
        path_arg: Some(0),
    },
    BuiltinSpec {
        name: "rev",
        params: &[&LIST_T],
        ret: &LIST_T,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "show",
        params: &[&T],
        ret: &STR,
        bounds: &[(TyVar::T, Bound::Showable)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "size",
        params: &[&DICT_AB],
        ret: &I64,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "slice",
        params: &[&LIST_T, &I64, &I64],
        ret: &LIST_T,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "sort",
        params: &[&LIST_T],
        ret: &LIST_T,
        bounds: &[(TyVar::T, Bound::Primitive)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "split",
        params: &[&STR, &STR],
        ret: &LIST_STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "sum",
        params: &[&LIST_I64],
        ret: &I64,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "trim",
        params: &[&STR],
        ret: &STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "upper",
        params: &[&STR],
        ret: &STR,
        bounds: &[],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "vals",
        params: &[&DICT_AB],
        ret: &LIST_B,
        bounds: &[(TyVar::A, Bound::Key)],
        effect: Effect::Pure,
        capability: None,
        path_arg: None,
    },
    BuiltinSpec {
        name: "write",
        params: &[&STR, &STR],
        ret: &RES_UNIT_STR,
        bounds: &[],
        effect: Effect::Effectful,
        capability: Some(Capability::FsWrite),
        path_arg: Some(0),
    },
];

/// Look up a builtin by name.
pub fn lookup(name: &str) -> Option<&'static BuiltinSpec> {
    BUILTINS.iter().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon;

    #[test]
    fn names_match_the_lexicon_exactly() {
        let mut expected: Vec<&str> = lexicon::builtin_names().to_vec();
        expected.sort_unstable();

        let actual: Vec<&str> = BUILTINS.iter().map(|spec| spec.name).collect();
        assert_eq!(actual, expected);
        assert!(BUILTINS.windows(2).all(|pair| pair[0].name < pair[1].name));
        assert!(actual.iter().all(|name| lookup(name).is_some()));
        assert!(lookup("nil").is_none());
        assert!(lookup("empty").is_none());
        assert!(lookup("none").is_none());
    }

    #[test]
    fn arities_match_spec_5_1() {
        let expected = [
            ("len", 1),
            ("at", 2),
            ("push", 2),
            ("slice", 3),
            ("rev", 1),
            ("sort", 1),
            ("range", 2),
            ("sum", 1),
            ("map", 2),
            ("filter", 2),
            ("fold", 3),
            ("cat", 2),
            ("count", 1),
            ("split", 2),
            ("join", 2),
            ("trim", 1),
            ("upper", 1),
            ("lower", 1),
            ("contains", 2),
            ("show", 1),
            ("int", 1),
            ("float", 1),
            ("args", 0),
            ("print", 1),
            ("eprint", 1),
            ("read", 1),
            ("write", 2),
            // SPEC §5.2 (ADR-0032)
            ("put", 3),
            ("get", 2),
            ("del", 2),
            ("keys", 1),
            ("vals", 1),
            ("size", 1),
            // ADR-0033
            ("json", 1),
            ("jparse", 1),
            ("jbool", 1),
            ("jint", 1),
            ("jnum", 1),
            ("jstr", 1),
            ("jlist", 1),
            ("jdict", 1),
        ];
        assert_eq!(expected.len(), BUILTINS.len());
        for (name, arity) in expected {
            let spec = lookup(name).unwrap_or_else(|| panic!("missing builtin `{name}`"));
            assert_eq!(spec.arity(), arity, "arity of `{name}`");
        }
    }

    #[test]
    fn sort_and_show_use_bounds_on_shared_type_variables() {
        let sort = lookup("sort").unwrap();
        let Ty::List(param_elem) = *sort.params[0] else {
            panic!("sort parameter should be a list");
        };
        let Ty::List(ret_elem) = *sort.ret else {
            panic!("sort return should be a list");
        };
        assert_eq!(param_elem, ret_elem);
        assert_eq!(*param_elem, Ty::Var(TyVar::T));
        assert_eq!(sort.bounds, &[(TyVar::T, Bound::Primitive)]);

        let show = lookup("show").unwrap();
        assert_eq!(*show.params[0], Ty::Var(TyVar::T));
        assert_eq!(*show.ret, Ty::Str);
        assert_eq!(show.bounds, &[(TyVar::T, Bound::Showable)]);
    }

    #[test]
    fn effectful_builtins_without_capability_are_only_print_and_eprint() {
        use std::collections::BTreeSet;

        let names: BTreeSet<&str> = BUILTINS
            .iter()
            .filter(|spec| spec.effect == Effect::Effectful && spec.capability.is_none())
            .map(|spec| spec.name)
            .collect();
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["eprint", "print"]);
    }

    #[test]
    fn path_args_are_within_arity() {
        for spec in BUILTINS {
            if let Some(index) = spec.path_arg {
                assert!(
                    index < spec.arity(),
                    "`{}` path_arg {} >= arity {}",
                    spec.name,
                    index,
                    spec.arity()
                );
            }
        }
    }

    #[test]
    fn effects_and_capabilities_match_spec_5_1() {
        let read = lookup("read").unwrap();
        assert_eq!(read.effect, Effect::Effectful);
        assert_eq!(read.capability, Some(Capability::FsRead));
        assert_eq!(read.path_arg, Some(0));

        let write = lookup("write").unwrap();
        assert_eq!(write.effect, Effect::Effectful);
        assert_eq!(write.capability, Some(Capability::FsWrite));
        assert_eq!(write.path_arg, Some(0));

        for name in ["print", "eprint"] {
            let spec = lookup(name).unwrap();
            assert_eq!(spec.effect, Effect::Effectful, "effect of `{name}`");
            assert_eq!(spec.capability, None, "capability of `{name}`");
        }

        for spec in BUILTINS {
            if let Some(capability) = spec.capability {
                assert_eq!(spec.effect, Effect::Effectful);
                assert_eq!(
                    capability,
                    match spec.name {
                        "read" => Capability::FsRead,
                        "write" => Capability::FsWrite,
                        other => panic!("unexpected capability on `{other}`"),
                    }
                );
            }
        }
    }
}
