//! Árbol con índices en lugar de nombres, para evaluar rápido dentro de los
//! ajustes (miles de evaluaciones por iteración en bootstrap y Monte Carlo).

use super::ast::{Expr, Func};

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Num(f64),
    X,
    Param(usize),
    Neg(Box<Node>),
    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),
    Pow(Box<Node>, Box<Node>),
    /// Potencia entera: exacta y válida con bases negativas.
    PowI(Box<Node>, i32),
    Call(Func, Box<Node>),
}

/// Resuelve los nombres. Supone que la expresión ya fue validada: un nombre
/// desconocido se convierte en NaN.
pub(crate) fn compile(e: &Expr, variable: &str, params: &[String], constants: &[(String, f64)]) -> Node {
    let c = |x: &Expr| Box::new(compile(x, variable, params, constants));
    match e {
        Expr::Num(v) => Node::Num(*v),
        Expr::Pi => Node::Num(std::f64::consts::PI),
        Expr::E => Node::Num(std::f64::consts::E),
        Expr::Var(n) if n == variable => Node::X,
        Expr::Var(n) => {
            if let Some(i) = params.iter().position(|p| p == n) {
                Node::Param(i)
            } else {
                let v = constants.iter().find(|(k, _)| k == n).map_or(f64::NAN, |(_, v)| *v);
                Node::Num(v)
            }
        }
        Expr::Neg(a) => Node::Neg(c(a)),
        Expr::Add(a, b) => Node::Add(c(a), c(b)),
        Expr::Sub(a, b) => Node::Sub(c(a), c(b)),
        Expr::Mul(a, b) => Node::Mul(c(a), c(b)),
        Expr::Div(a, b) => Node::Div(c(a), c(b)),
        Expr::Pow(a, b) => match **b {
            Expr::Num(n) if n.fract() == 0.0 && n.abs() <= 1024.0 => Node::PowI(c(a), n as i32),
            _ => Node::Pow(c(a), c(b)),
        },
        Expr::Call(f, a) => Node::Call(*f, c(a)),
    }
}

impl Node {
    pub(crate) fn eval(&self, x: f64, p: &[f64]) -> f64 {
        match self {
            Node::Num(v) => *v,
            Node::X => x,
            Node::Param(i) => p[*i],
            Node::Neg(a) => -a.eval(x, p),
            Node::Add(a, b) => a.eval(x, p) + b.eval(x, p),
            Node::Sub(a, b) => a.eval(x, p) - b.eval(x, p),
            Node::Mul(a, b) => a.eval(x, p) * b.eval(x, p),
            Node::Div(a, b) => a.eval(x, p) / b.eval(x, p),
            Node::Pow(a, b) => a.eval(x, p).powf(b.eval(x, p)),
            Node::PowI(a, n) => a.eval(x, p).powi(*n),
            Node::Call(f, a) => f.apply(a.eval(x, p)),
        }
    }
}
