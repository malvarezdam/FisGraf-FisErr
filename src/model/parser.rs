//! Parser de descenso recursivo para la sintaxis calculable.
//!
//! Gramática (de menor a mayor precedencia):
//! ```text
//! expr    := term (('+' | '-') term)*
//! term    := unary (('*' | '/') unary | implícita power)*
//! unary   := ('-' | '+') unary | power
//! power   := primary ('^' unary)?          // asociativa a la derecha
//! primary := número | nombre | función '(' args ')' | '(' expr ')'
//! ```
//! La multiplicación implícita se admite delante de un nombre o un paréntesis:
//! `2pi`, `2(x + 1)`, `a(x - b)`. `**` equivale a `^`.

use super::ast::{Expr, Func};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message} (posición {})", .position + 1)]
pub struct ParseError {
    pub message: String,
    /// Índice del carácter (base 0) donde se detectó el error.
    pub position: usize,
}

fn error<T>(message: impl Into<String>, position: usize) -> Result<T, ParseError> {
    Err(ParseError {
        message: message.into(),
        position,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    pos: usize,
}

fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        let digit_at = |j: usize| chars.get(j).is_some_and(|c| c.is_ascii_digit());

        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let tok = if c.is_ascii_digit() || (c == '.' && digit_at(i + 1)) {
            while digit_at(i) {
                i += 1;
            }
            if chars.get(i) == Some(&'.') {
                i += 1;
                while digit_at(i) {
                    i += 1;
                }
            }
            if chars.get(i) == Some(&'.') {
                return error("número mal escrito: tiene dos puntos decimales", i);
            }
            // Exponente solo si sigue un dígito: `2e` es 2·e, `2e-3` es 0,002.
            if matches!(chars.get(i), Some('e' | 'E'))
                && (digit_at(i + 1) || (matches!(chars.get(i + 1), Some('+' | '-')) && digit_at(i + 2)))
            {
                i += 2;
                while digit_at(i) {
                    i += 1;
                }
            }
            let text: String = chars[start..i].iter().collect();
            match text.parse() {
                Ok(v) => Tok::Num(v),
                Err(_) => return error(format!("número inválido «{text}»"), start),
            }
        } else if c.is_alphabetic() || c == '_' {
            while chars.get(i).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
                i += 1;
            }
            Tok::Ident(chars[start..i].iter().collect())
        } else {
            i += 1;
            match c {
                '+' => Tok::Plus,
                '-' | '−' => Tok::Minus,
                '*' | '·' | '×' if chars.get(i) == Some(&'*') => {
                    i += 1;
                    Tok::Caret
                }
                '*' | '·' | '×' => Tok::Star,
                '/' | '÷' => Tok::Slash,
                '^' => Tok::Caret,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                ',' | ';' => Tok::Comma,
                _ => return error(format!("carácter inesperado «{c}»"), start),
            }
        };
        out.push(Token { tok, pos: start });
    }
    out.push(Token {
        tok: Tok::End,
        pos: chars.len(),
    });
    Ok(out)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.i].tok
    }

    fn pos(&self) -> usize {
        self.toks[self.i].pos
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.i].tok.clone();
        if self.i + 1 < self.toks.len() {
            self.i += 1;
        }
        t
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.term()?;
        loop {
            match self.peek() {
                Tok::Plus => {
                    self.bump();
                    lhs = Expr::Add(Box::new(lhs), Box::new(self.term()?));
                }
                Tok::Minus => {
                    self.bump();
                    lhs = Expr::Sub(Box::new(lhs), Box::new(self.term()?));
                }
                _ => return Ok(lhs),
            }
        }
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        loop {
            match self.peek() {
                Tok::Star => {
                    self.bump();
                    lhs = Expr::Mul(Box::new(lhs), Box::new(self.unary()?));
                }
                Tok::Slash => {
                    self.bump();
                    lhs = Expr::Div(Box::new(lhs), Box::new(self.unary()?));
                }
                Tok::Ident(_) | Tok::LParen => {
                    lhs = Expr::Mul(Box::new(lhs), Box::new(self.power()?));
                }
                _ => return Ok(lhs),
            }
        }
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Tok::Plus => {
                self.bump();
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Expr, ParseError> {
        let base = self.primary()?;
        if *self.peek() == Tok::Caret {
            self.bump();
            let exp = self.unary()?;
            return Ok(Expr::Pow(Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let pos = self.pos();
        match self.bump() {
            Tok::Num(v) => Ok(Expr::Num(v)),
            Tok::Ident(name) => {
                let is_call = *self.peek() == Tok::LParen;
                if name == "pow" {
                    let mut args = self.args(&name, 2, pos)?;
                    let b = args.pop().expect("2 argumentos");
                    let a = args.pop().expect("2 argumentos");
                    return Ok(Expr::Pow(Box::new(a), Box::new(b)));
                }
                if let Some(func) = Func::from_name(&name) {
                    if !is_call {
                        return error(format!("la función «{name}» necesita paréntesis: {name}(…)"), pos);
                    }
                    let arg = self.args(&name, 1, pos)?.pop().expect("1 argumento");
                    return Ok(Expr::Call(func, Box::new(arg)));
                }
                Ok(match name.as_str() {
                    "pi" | "π" => Expr::Pi,
                    "e" => Expr::E,
                    _ => Expr::Var(name),
                })
            }
            Tok::LParen => {
                let inner = self.expr()?;
                if *self.peek() != Tok::RParen {
                    return error("falta cerrar un paréntesis «)»", self.pos());
                }
                self.bump();
                Ok(inner)
            }
            Tok::End => error("la expresión termina de forma inesperada", pos),
            Tok::RParen => error("paréntesis «)» sin abrir o vacío", pos),
            _ => error("se esperaba un número, un nombre o «(»", pos),
        }
    }

    fn args(&mut self, name: &str, count: usize, pos: usize) -> Result<Vec<Expr>, ParseError> {
        if *self.peek() != Tok::LParen {
            return error(format!("la función «{name}» necesita paréntesis"), pos);
        }
        self.bump();
        let mut args = vec![self.expr()?];
        while *self.peek() == Tok::Comma {
            self.bump();
            args.push(self.expr()?);
        }
        if *self.peek() != Tok::RParen {
            return error("falta cerrar un paréntesis «)»", self.pos());
        }
        self.bump();
        if args.len() != count {
            return error(
                format!("«{name}» recibe {count} argumento(s), no {}", args.len()),
                pos,
            );
        }
        Ok(args)
    }
}

/// Convierte texto en un árbol de expresión.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let toks = lex(src)?;
    if toks.len() == 1 {
        return error("la expresión está vacía", 0);
    }
    let mut p = Parser { toks, i: 0 };
    let e = p.expr()?;
    match p.peek() {
        Tok::End => Ok(e),
        Tok::RParen => error("paréntesis «)» sin abrir", p.pos()),
        _ => error("se esperaba un operador (+ - * / ^)", p.pos()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(src: &str) -> String {
        parse(src).unwrap().to_string()
    }

    #[test]
    fn precedence() {
        assert_eq!(s("1 + 2*3"), "1 + 2*3");
        assert_eq!(s("(1 + 2)*3"), "(1 + 2)*3");
        assert_eq!(s("-x^2"), "-x^2");
        assert_eq!(s("(-x)^2"), "(-x)^2");
        assert_eq!(s("2^3^2"), "2^3^2");
        assert_eq!(s("(2^3)^2"), "(2^3)^2");
        assert_eq!(s("a - (b - c)"), "a - (b - c)");
        assert_eq!(s("a/(b*c)"), "a/(b*c)");
        assert_eq!(s("2^-x"), "2^(-x)");
        assert_eq!(s("x**2"), "x^2");
    }

    #[test]
    fn implicit_multiplication_and_names() {
        assert_eq!(s("2pi"), "2*pi");
        assert_eq!(s("2(x + 1)"), "2*(x + 1)");
        assert_eq!(s("a(x - b)"), "a*(x - b)");
        assert_eq!(s("1/2x"), "1/2*x");
        assert_eq!(s("2e"), "2*e");
        assert_eq!(s("2e-3"), "0.002");
        assert_eq!(s("1.5E3 + .5"), "1500 + 0.5");
        assert_eq!(s("T_teo * ω0"), "T_teo*ω0");
        assert_eq!(s("pow(x, 3) + log(x)"), "x^3 + ln(x)");
        assert_eq!(s("arcsin(x)·2"), "asin(x)*2");
    }

    #[test]
    fn errors_with_position() {
        let e = |src: &str| parse(src).unwrap_err();
        assert_eq!(e("").message, "la expresión está vacía");
        assert_eq!(e("2*").position, 2);
        assert!(e("(x").message.contains("falta cerrar"));
        assert!(e("x)").message.contains("sin abrir"));
        assert_eq!(e("x)").position, 1);
        assert!(e("sin x").message.contains("paréntesis"));
        assert!(e("sin()").message.contains("vacío"));
        assert!(e("pow(2)").message.contains("2 argumento"));
        assert!(e("2..3").message.contains("dos puntos"));
        assert!(e("2 3").message.contains("operador"));
        assert_eq!(e("x $ 2").position, 2);
        assert_eq!(e("x $ 2").to_string(), "carácter inesperado «$» (posición 3)");
    }
}
