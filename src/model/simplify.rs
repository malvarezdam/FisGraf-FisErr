//! Constructores que simplifican al construir: evalúan constantes numéricas y
//! eliminan neutros (`x + 0`, `1·x`, `x^1`…). Las derivadas se arman con ellos
//! para no crecer sin control.

use super::ast::{Expr, Func};

pub fn num(v: f64) -> Expr {
    Expr::Num(v)
}

fn is_num(e: &Expr, v: f64) -> bool {
    matches!(e, Expr::Num(x) if *x == v)
}

/// Resultado numérico solo si es finito; si no, se deja simbólico.
fn folded(v: f64) -> Option<Expr> {
    v.is_finite().then_some(Expr::Num(v))
}

pub fn neg(a: Expr) -> Expr {
    match a {
        Expr::Num(v) => Expr::Num(-v),
        Expr::Neg(b) => *b,
        Expr::Sub(x, y) => Expr::Sub(y, x),
        other => Expr::Neg(Box::new(other)),
    }
}

pub fn add(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => Expr::Num(x + y),
        (a, b) if is_num(&a, 0.0) => b,
        (a, b) if is_num(&b, 0.0) => a,
        (a, Expr::Neg(b)) => sub(a, *b),
        (a, Expr::Num(y)) if y < 0.0 => sub(a, Expr::Num(-y)),
        (Expr::Neg(a), b) => sub(b, *a),
        (a, b) => Expr::Add(Box::new(a), Box::new(b)),
    }
}

pub fn sub(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => Expr::Num(x - y),
        (a, b) if is_num(&b, 0.0) => a,
        (a, b) if is_num(&a, 0.0) => neg(b),
        (a, b) if a == b => num(0.0),
        (a, Expr::Neg(b)) => add(a, *b),
        (a, Expr::Num(y)) if y < 0.0 => add(a, Expr::Num(-y)),
        (a, b) => Expr::Sub(Box::new(a), Box::new(b)),
    }
}

pub fn mul(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => Expr::Num(x * y),
        (a, b) if is_num(&a, 0.0) || is_num(&b, 0.0) => num(0.0),
        (a, b) if is_num(&a, 1.0) => b,
        (a, b) if is_num(&b, 1.0) => a,
        (a, b) if is_num(&a, -1.0) => neg(b),
        (a, b) if is_num(&b, -1.0) => neg(a),
        (Expr::Neg(a), b) => neg(mul(*a, b)),
        (a, Expr::Neg(b)) => neg(mul(a, *b)),
        // Los números van a la izquierda: x·2 → 2·x.
        (a, Expr::Num(y)) => mul(Expr::Num(y), a),
        (Expr::Num(x), Expr::Mul(b, c)) if matches!(*b, Expr::Num(_)) => {
            let Expr::Num(y) = *b else { unreachable!() };
            mul(Expr::Num(x * y), *c)
        }
        // a·(1/c) → a/c
        (a, Expr::Div(b, c)) if is_num(&b, 1.0) => div(a, *c),
        // a·(2·c) → 2·a·c
        (a, Expr::Mul(b, c)) if matches!(*b, Expr::Num(_)) => mul(mul(*b, a), *c),
        (a, b) if a == b => pow(a, num(2.0)),
        (a, b) => Expr::Mul(Box::new(a), Box::new(b)),
    }
}

pub fn div(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) if y != 0.0 && (x / y).fract() == 0.0 => Expr::Num(x / y),
        (a, b) if is_num(&a, 0.0) && !is_num(&b, 0.0) => num(0.0),
        (a, b) if is_num(&b, 1.0) => a,
        (a, b) if is_num(&b, -1.0) => neg(a),
        (a, b) if a == b => num(1.0),
        (Expr::Neg(a), b) => neg(div(*a, b)),
        (a, Expr::Neg(b)) => neg(div(a, *b)),
        (Expr::Num(x), b) if x < 0.0 => neg(div(Expr::Num(-x), b)),
        (a, b) => Expr::Div(Box::new(a), Box::new(b)),
    }
}

pub fn pow(a: Expr, b: Expr) -> Expr {
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) if folded(x.powf(y)).is_some() => Expr::Num(x.powf(y)),
        (_, b) if is_num(&b, 0.0) => num(1.0),
        (a, b) if is_num(&b, 1.0) => a,
        (a, _) if is_num(&a, 1.0) => num(1.0),
        // (x^a)^n = x^(a·n) con n entero.
        (Expr::Pow(x, a), Expr::Num(n)) if n.fract() == 0.0 && matches!(*a, Expr::Num(_)) => {
            let Expr::Num(a) = *a else { unreachable!() };
            pow(*x, num(a * n))
        }
        (a, b) => Expr::Pow(Box::new(a), Box::new(b)),
    }
}

pub fn call(f: Func, a: Expr) -> Expr {
    if let Expr::Num(v) = a
        && let Some(r) = folded(f.apply(v))
    {
        return r;
    }
    Expr::Call(f, Box::new(a))
}

/// Reconstruye toda la expresión con los constructores simplificadores.
pub fn simplify(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Pi | Expr::E | Expr::Var(_) => e.clone(),
        Expr::Neg(a) => neg(simplify(a)),
        Expr::Add(a, b) => add(simplify(a), simplify(b)),
        Expr::Sub(a, b) => sub(simplify(a), simplify(b)),
        Expr::Mul(a, b) => mul(simplify(a), simplify(b)),
        Expr::Div(a, b) => div(simplify(a), simplify(b)),
        Expr::Pow(a, b) => pow(simplify(a), simplify(b)),
        Expr::Call(f, a) => call(*f, simplify(a)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parser::parse;

    fn s(src: &str) -> String {
        simplify(&parse(src).unwrap()).to_string()
    }

    #[test]
    fn rules() {
        assert_eq!(s("0 + x*1"), "x");
        assert_eq!(s("x - 0"), "x");
        assert_eq!(s("0 - x"), "-x");
        assert_eq!(s("x - x"), "0");
        assert_eq!(s("x*0 + 3"), "3");
        assert_eq!(s("x*2*3"), "6*x");
        assert_eq!(s("--x"), "x");
        assert_eq!(s("a + -b"), "a - b");
        assert_eq!(s("a - -b"), "a + b");
        assert_eq!(s("x^1 + y^0"), "x + 1");
        assert_eq!(s("x*x"), "x^2");
        assert_eq!(s("(T^2)^2"), "T^4");
        assert_eq!(s("(x^0.5)^2"), "x");
        assert_eq!(s("2^3 + sqrt(4)"), "10");
        assert_eq!(s("1/3"), "1/3");
        assert_eq!(s("6/3"), "2");
        assert_eq!(s("1/0"), "1/0");
        assert_eq!(s("(-a)*b/(-c)"), "a*b/c");
    }
}
