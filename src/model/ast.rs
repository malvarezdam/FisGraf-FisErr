//! Árbol de sintaxis de las ecuaciones, con nombres (para LaTeX y derivadas).

use std::collections::BTreeSet;
use std::fmt;

/// Funciones de una variable disponibles en las ecuaciones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Exp,
    /// Logaritmo natural (`ln` o `log`).
    Ln,
    Log10,
    Sqrt,
    Abs,
    Sign,
}

impl Func {
    pub const ALL: [Func; 15] = [
        Func::Sin,
        Func::Cos,
        Func::Tan,
        Func::Asin,
        Func::Acos,
        Func::Atan,
        Func::Sinh,
        Func::Cosh,
        Func::Tanh,
        Func::Exp,
        Func::Ln,
        Func::Log10,
        Func::Sqrt,
        Func::Abs,
        Func::Sign,
    ];

    /// Nombre canónico en la sintaxis calculable.
    pub fn name(self) -> &'static str {
        match self {
            Func::Sin => "sin",
            Func::Cos => "cos",
            Func::Tan => "tan",
            Func::Asin => "asin",
            Func::Acos => "acos",
            Func::Atan => "atan",
            Func::Sinh => "sinh",
            Func::Cosh => "cosh",
            Func::Tanh => "tanh",
            Func::Exp => "exp",
            Func::Ln => "ln",
            Func::Log10 => "log10",
            Func::Sqrt => "sqrt",
            Func::Abs => "abs",
            Func::Sign => "sign",
        }
    }

    /// Acepta también alias habituales (`arcsin`, `log`, `sgn`).
    pub fn from_name(name: &str) -> Option<Func> {
        Some(match name {
            "sin" | "sen" => Func::Sin,
            "cos" => Func::Cos,
            "tan" | "tg" => Func::Tan,
            "asin" | "arcsin" => Func::Asin,
            "acos" | "arccos" => Func::Acos,
            "atan" | "arctan" => Func::Atan,
            "sinh" | "senh" => Func::Sinh,
            "cosh" => Func::Cosh,
            "tanh" => Func::Tanh,
            "exp" => Func::Exp,
            "ln" | "log" => Func::Ln,
            "log10" => Func::Log10,
            "sqrt" => Func::Sqrt,
            "abs" => Func::Abs,
            "sign" | "sgn" => Func::Sign,
            _ => return None,
        })
    }

    pub fn apply(self, v: f64) -> f64 {
        match self {
            Func::Sin => v.sin(),
            Func::Cos => v.cos(),
            Func::Tan => v.tan(),
            Func::Asin => v.asin(),
            Func::Acos => v.acos(),
            Func::Atan => v.atan(),
            Func::Sinh => v.sinh(),
            Func::Cosh => v.cosh(),
            Func::Tanh => v.tanh(),
            Func::Exp => v.exp(),
            Func::Ln => v.ln(),
            Func::Log10 => v.log10(),
            Func::Sqrt => v.sqrt(),
            Func::Abs => v.abs(),
            Func::Sign => {
                if v > 0.0 {
                    1.0
                } else if v < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            }
        }
    }
}

/// Nombres que no pueden usarse como variable, parámetro ni constante.
pub fn is_reserved(name: &str) -> bool {
    matches!(name, "pi" | "e" | "pow") || Func::from_name(name).is_some()
}

/// `true` si `name` es un identificador válido en la sintaxis calculable.
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Pi,
    E,
    /// Variable, parámetro o constante con nombre.
    Var(String),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Call(Func, Box<Expr>),
}

impl Expr {
    /// Nombres usados en la expresión (sin `pi` ni `e`).
    pub fn free_symbols(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        self.visit(&mut |e| {
            if let Expr::Var(n) = e {
                out.insert(n.clone());
            }
        });
        out
    }

    /// `true` si la expresión depende de `name`.
    pub fn contains(&self, name: &str) -> bool {
        let mut found = false;
        self.visit(&mut |e| found |= matches!(e, Expr::Var(n) if n == name));
        found
    }

    /// Recorre todos los nodos en preorden.
    pub fn visit(&self, f: &mut impl FnMut(&Expr)) {
        f(self);
        match self {
            Expr::Num(_) | Expr::Pi | Expr::E | Expr::Var(_) => {}
            Expr::Neg(a) | Expr::Call(_, a) => a.visit(f),
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
                a.visit(f);
                b.visit(f);
            }
        }
    }

    /// Evalúa buscando cada nombre con `lookup` (NaN si no existe). Más lenta
    /// que un [`crate::model::Model`] compilado; útil para expresiones sueltas.
    pub fn eval_with(&self, lookup: &dyn Fn(&str) -> Option<f64>) -> f64 {
        let ev = |e: &Expr| e.eval_with(lookup);
        match self {
            Expr::Num(v) => *v,
            Expr::Pi => std::f64::consts::PI,
            Expr::E => std::f64::consts::E,
            Expr::Var(n) => lookup(n).unwrap_or(f64::NAN),
            Expr::Neg(a) => -ev(a),
            Expr::Add(a, b) => ev(a) + ev(b),
            Expr::Sub(a, b) => ev(a) - ev(b),
            Expr::Mul(a, b) => ev(a) * ev(b),
            Expr::Div(a, b) => ev(a) / ev(b),
            Expr::Pow(a, b) => match **b {
                Expr::Num(n) if n.fract() == 0.0 && n.abs() <= 1024.0 => ev(a).powi(n as i32),
                _ => ev(a).powf(ev(b)),
            },
            Expr::Call(f, a) => f.apply(ev(a)),
        }
    }

    /// Precedencia para imprimir en sintaxis calculable.
    fn prec(&self) -> u8 {
        match self {
            Expr::Add(..) | Expr::Sub(..) => 1,
            Expr::Mul(..) | Expr::Div(..) => 2,
            Expr::Neg(_) => 3,
            Expr::Num(v) if *v < 0.0 => 3,
            Expr::Pow(..) => 4,
            _ => 5,
        }
    }
}

/// Imprime en sintaxis calculable, con los paréntesis mínimos para volver a
/// leerse igual.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let child = |f: &mut fmt::Formatter<'_>, e: &Expr, min: u8| {
            if e.prec() < min {
                write!(f, "({e})")
            } else {
                write!(f, "{e}")
            }
        };
        match self {
            Expr::Num(v) => write!(f, "{v}"),
            Expr::Pi => f.write_str("pi"),
            Expr::E => f.write_str("e"),
            Expr::Var(n) => f.write_str(n),
            // -(a·b) y (-a)·b valen lo mismo: basta con agrupar sumas.
            Expr::Neg(a) => {
                f.write_str("-")?;
                child(f, a, 2)
            }
            Expr::Add(a, b) => {
                child(f, a, 1)?;
                f.write_str(" + ")?;
                child(f, b, 2)
            }
            Expr::Sub(a, b) => {
                child(f, a, 1)?;
                f.write_str(" - ")?;
                child(f, b, 2)
            }
            Expr::Mul(a, b) => {
                child(f, a, 2)?;
                f.write_str("*")?;
                child(f, b, 4)
            }
            Expr::Div(a, b) => {
                child(f, a, 2)?;
                f.write_str("/")?;
                child(f, b, 4)
            }
            Expr::Pow(a, b) => {
                child(f, a, 5)?;
                f.write_str("^")?;
                child(f, b, 4)
            }
            Expr::Call(func, a) => write!(f, "{}({a})", func.name()),
        }
    }
}
