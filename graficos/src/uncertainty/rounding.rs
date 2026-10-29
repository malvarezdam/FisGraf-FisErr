//! Redondeo de resultados con su incertidumbre (GUM, §7.2.6) y formato.
//!
//! La incertidumbre se redondea a 1 o 2 cifras significativas y el valor a la
//! misma posición decimal. La regla automática es la del Particle Data Group:
//! se miran las tres primeras cifras de σ; de 100 a 354 se dejan 2 cifras,
//! de 355 a 949 se deja 1, y de 950 a 999 se sube a 1000 con 2 cifras.

use crate::model::number_latex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SigFigs {
    /// Regla del PDG.
    #[default]
    Auto,
    One,
    Two,
}

/// Valor e incertidumbre ya redondeados, expresados como mantisas × 10^`exponent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rounded {
    pub value: f64,
    pub sigma: f64,
    /// Decimales con que se escriben `value` y `sigma` (pueden ser 0).
    pub decimals: usize,
    /// Potencia de 10 común (0 = sin notación científica).
    pub exponent: i32,
}

/// Redondea `value ± sigma`. Con `scientific = None` se usa notación científica
/// solo si |valor| ≥ 10⁴ o < 10⁻³.
pub fn round_gum(value: f64, sigma: f64, sig: SigFigs, scientific: Option<bool>) -> Rounded {
    let reference = if sigma.is_finite() { value.abs().max(sigma.abs()) } else { value.abs() };
    let use_sci = scientific.unwrap_or(reference != 0.0 && !(1e-3..1e4).contains(&reference));
    let exponent = if use_sci && reference.is_finite() && reference > 0.0 {
        reference.log10().floor() as i32
    } else {
        0
    };
    let scale = 10f64.powi(exponent);
    let (v, s) = (value / scale, sigma / scale);

    if !(s.is_finite() && s > 0.0) {
        // Sin incertidumbre utilizable: 6 cifras significativas.
        let mag = if v != 0.0 { v.abs().log10().floor() as i32 } else { 0 };
        let decimals = (5 - mag).clamp(0, 15) as usize;
        return Rounded { value: round_to(v, decimals as i32), sigma: f64::NAN, decimals, exponent };
    }

    let mut e = s.log10().floor() as i32;
    let figures = match sig {
        SigFigs::One => 1,
        SigFigs::Two => 2,
        SigFigs::Auto => {
            let lead = (s / 10f64.powi(e - 2)).round();
            if lead <= 354.0 {
                2
            } else if lead <= 949.0 {
                1
            } else {
                // 950–999 → 1,0 × 10^(e+1)
                e += 1;
                2
            }
        }
    };
    let position = e - figures + 1; // potencia de 10 de la última cifra significativa
    let mut sigma_r = round_to(s, -position);
    // El redondeo puede subir de década (0,96 → 1,0): se recalcula la posición.
    if sigma_r >= 10f64.powi(e + 1) && sig != SigFigs::Auto {
        e += 1;
        sigma_r = round_to(s, -(e - figures + 1));
    }
    let position = e - figures + 1;
    let decimals = (-position).max(0) as usize;
    Rounded { value: round_to(v, -position), sigma: sigma_r, decimals, exponent }
}

/// Redondea a `decimals` cifras decimales (negativo = decenas, centenas…).
fn round_to(v: f64, decimals: i32) -> f64 {
    let f = 10f64.powi(decimals);
    (v * f).round() / f
}

fn fixed(v: f64, decimals: usize, decimal_comma: bool, latex: bool) -> String {
    let s = format!("{v:.decimals$}");
    let s = if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') { s[1..].to_owned() } else { s };
    match (decimal_comma, latex) {
        (true, true) => s.replace('.', "{,}"),
        (true, false) => s.replace('.', ","),
        _ => s,
    }
}

impl Rounded {
    /// Texto plano: `9,79 ± 0,02` o `(1,23 ± 0,05)×10^-3`.
    pub fn plain(&self, decimal_comma: bool) -> String {
        let v = fixed(self.value, self.decimals, decimal_comma, false);
        let body = if self.sigma.is_nan() {
            v
        } else {
            format!("{v} ± {}", fixed(self.sigma, self.decimals, decimal_comma, false))
        };
        match self.exponent {
            0 => body,
            e if self.sigma.is_nan() => format!("{body}×10^{e}"),
            e => format!("({body})×10^{e}"),
        }
    }

    /// LaTeX (modo matemático): `9{,}79 \pm 0{,}02` o
    /// `\left(1{,}23 \pm 0{,}05\right) \times 10^{-3}`.
    pub fn latex(&self, decimal_comma: bool) -> String {
        let v = fixed(self.value, self.decimals, decimal_comma, true);
        let body = if self.sigma.is_nan() {
            v
        } else {
            format!("{v} \\pm {}", fixed(self.sigma, self.decimals, decimal_comma, true))
        };
        match self.exponent {
            0 => body,
            e if self.sigma.is_nan() => format!("{body} \\times 10^{{{e}}}"),
            e => format!("\\left({body}\\right) \\times 10^{{{e}}}"),
        }
    }
}

/// LaTeX de un número suelto con el mismo criterio que las ecuaciones.
pub fn value_latex(v: f64, decimal_comma: bool) -> String {
    number_latex(v, decimal_comma)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(v: f64, s: f64) -> String {
        round_gum(v, s, SigFigs::Auto, None).plain(true)
    }

    #[test]
    fn pdg_rule() {
        assert_eq!(p(9.7934, 0.0213), "9,793 ± 0,021");
        assert_eq!(p(9.7934, 0.0456), "9,79 ± 0,05");
        assert_eq!(p(9.7934, 0.0976), "9,79 ± 0,10");
        assert_eq!(p(238.94212918, 2.7070075241), "238,9 ± 2,7");
        assert_eq!(p(238.94212918, 4.1), "239 ± 4");
        assert_eq!(p(1.07763517, 0.17070154742), "1,08 ± 0,17");
        assert_eq!(p(1234.4, 35.0), "1234 ± 35");
        assert_eq!(p(0.0, 0.0123), "0,000 ± 0,012");
        assert_eq!(p(-0.0004, 0.0123), "0,000 ± 0,012");
    }

    #[test]
    fn fixed_figures() {
        let r = round_gum(9.7934, 0.0456, SigFigs::Two, None);
        assert_eq!(r.plain(false), "9.793 ± 0.046");
        let r = round_gum(9.7934, 0.0213, SigFigs::One, None);
        assert_eq!(r.plain(false), "9.79 ± 0.02");
        let r = round_gum(9.7, 0.96, SigFigs::One, None);
        assert_eq!(r.plain(false), "10 ± 1");
    }

    #[test]
    fn scientific() {
        let r = round_gum(5.5015643181e-4, 7.2668688436e-6, SigFigs::Auto, None);
        assert_eq!(r.exponent, -4);
        assert_eq!(r.plain(true), "(5,50 ± 0,07)×10^-4");
        assert_eq!(r.latex(true), "\\left(5{,}50 \\pm 0{,}07\\right) \\times 10^{-4}");
        let r = round_gum(6.02214e23, 3.1e19, SigFigs::Auto, None);
        assert_eq!(r.plain(false), "(6.02214 ± 0.00031)×10^23");
        let r = round_gum(12.3, 0.4, SigFigs::Auto, Some(true));
        assert_eq!(r.latex(false), "\\left(1.23 \\pm 0.04\\right) \\times 10^{1}");
    }

    #[test]
    fn without_uncertainty() {
        assert_eq!(round_gum(9.80665, f64::NAN, SigFigs::Auto, None).plain(true), "9,80665");
        assert_eq!(round_gum(2.0, 0.0, SigFigs::Auto, None).latex(false), "2.00000");
    }
}
