//! Derivadas simbólicas.

use super::ast::{Expr, Func};
use super::simplify::{add, call, div, mul, neg, num, pow, sub};

/// Derivada de `e` respecto de `v`, ya simplificada.
pub fn diff(e: &Expr, v: &str) -> Expr {
    if !e.contains(v) {
        return num(0.0);
    }
    let d = |x: &Expr| diff(x, v);
    match e {
        Expr::Num(_) | Expr::Pi | Expr::E => num(0.0),
        Expr::Var(n) => num(if n == v { 1.0 } else { 0.0 }),
        Expr::Neg(a) => neg(d(a)),
        Expr::Add(a, b) => add(d(a), d(b)),
        Expr::Sub(a, b) => sub(d(a), d(b)),
        Expr::Mul(a, b) => add(mul(d(a), (**b).clone()), mul((**a).clone(), d(b))),
        Expr::Div(a, b) => {
            if b.contains(v) {
                // (a'b − ab') / b²
                let top = sub(mul(d(a), (**b).clone()), mul((**a).clone(), d(b)));
                div(top, pow((**b).clone(), num(2.0)))
            } else {
                div(d(a), (**b).clone())
            }
        }
        Expr::Pow(a, b) => {
            let (a, b) = (&**a, &**b);
            if !b.contains(v) {
                // b · a^(b−1) · a'
                let factor = mul(b.clone(), pow(a.clone(), sub(b.clone(), num(1.0))));
                mul(factor, d(a))
            } else if !a.contains(v) {
                // a^b · ln(a) · b'
                mul(mul(e.clone(), call(Func::Ln, a.clone())), d(b))
            } else {
                // a^b · (b'·ln(a) + b·a'/a)
                let inner = add(
                    mul(d(b), call(Func::Ln, a.clone())),
                    div(mul(b.clone(), d(a)), a.clone()),
                );
                mul(e.clone(), inner)
            }
        }
        Expr::Call(f, a) => {
            let a = &**a;
            mul(outer_derivative(*f, a), d(a))
        }
    }
}

/// f'(a) para cada función de una variable.
fn outer_derivative(f: Func, a: &Expr) -> Expr {
    let a = || a.clone();
    let one_minus_a2 = || sub(num(1.0), pow(a(), num(2.0)));
    match f {
        Func::Sin => call(Func::Cos, a()),
        Func::Cos => neg(call(Func::Sin, a())),
        Func::Tan => div(num(1.0), pow(call(Func::Cos, a()), num(2.0))),
        Func::Asin => div(num(1.0), call(Func::Sqrt, one_minus_a2())),
        Func::Acos => neg(div(num(1.0), call(Func::Sqrt, one_minus_a2()))),
        Func::Atan => div(num(1.0), add(num(1.0), pow(a(), num(2.0)))),
        Func::Sinh => call(Func::Cosh, a()),
        Func::Cosh => call(Func::Sinh, a()),
        Func::Tanh => sub(num(1.0), pow(call(Func::Tanh, a()), num(2.0))),
        Func::Exp => call(Func::Exp, a()),
        Func::Ln => div(num(1.0), a()),
        Func::Log10 => div(num(1.0), mul(num(std::f64::consts::LN_10), a())),
        Func::Sqrt => div(num(1.0), mul(num(2.0), call(Func::Sqrt, a()))),
        Func::Abs => call(Func::Sign, a()),
        Func::Sign => num(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parser::parse;

    fn d(src: &str, v: &str) -> String {
        diff(&parse(src).unwrap(), v).to_string()
    }

    #[test]
    fn readable_results() {
        assert_eq!(d("a*x + b", "a"), "x");
        assert_eq!(d("a*x + b", "b"), "1");
        assert_eq!(d("a*x + b", "x"), "a");
        assert_eq!(d("a*x^2", "x"), "2*a*x");
        assert_eq!(d("sin(w*t)", "w"), "cos(w*t)*t");
        assert_eq!(d("exp(-x/tau)", "x"), "-exp(-x/tau)/tau");
        assert_eq!(d("c", "x"), "0");
    }
}
