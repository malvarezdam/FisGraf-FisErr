//! Aproximación en texto plano de etiquetas LaTeX, para la vista rápida
//! (el gráfico final sí se compone con LaTeX).

const SYMBOLS: [(&str, &str); 44] = [
    ("alpha", "α"), ("beta", "β"), ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("varepsilon", "ε"),
    ("zeta", "ζ"), ("eta", "η"), ("theta", "θ"), ("vartheta", "ϑ"), ("iota", "ι"), ("kappa", "κ"),
    ("lambda", "λ"), ("mu", "μ"), ("nu", "ν"), ("xi", "ξ"), ("pi", "π"), ("rho", "ρ"), ("sigma", "σ"),
    ("tau", "τ"), ("upsilon", "υ"), ("phi", "ϕ"), ("varphi", "φ"), ("chi", "χ"), ("psi", "ψ"), ("omega", "ω"),
    ("Gamma", "Γ"), ("Delta", "Δ"), ("Theta", "Θ"), ("Lambda", "Λ"), ("Xi", "Ξ"), ("Pi", "Π"), ("Sigma", "Σ"),
    ("Phi", "Φ"), ("Psi", "Ψ"), ("Omega", "Ω"), ("pm", "±"), ("times", "×"), ("cdot", "·"), ("infty", "∞"),
    ("degree", "°"), ("circ", "°"), ("approx", "≈"), ("sqrt", "√"),
];

const SUPERSCRIPTS: [(char, char); 13] = [
    ('0', '⁰'), ('1', '¹'), ('2', '²'), ('3', '³'), ('4', '⁴'), ('5', '⁵'), ('6', '⁶'),
    ('7', '⁷'), ('8', '⁸'), ('9', '⁹'), ('-', '⁻'), ('+', '⁺'), ('n', 'ⁿ'),
];

const SUBSCRIPTS: [(char, char); 12] = [
    ('0', '₀'), ('1', '₁'), ('2', '₂'), ('3', '₃'), ('4', '₄'), ('5', '₅'), ('6', '₆'),
    ('7', '₇'), ('8', '₈'), ('9', '₉'), ('-', '₋'), ('+', '₊'),
];

/// Lee un grupo `{…}` o un solo carácter a partir de `chars[i]`.
fn group(chars: &[char], i: &mut usize) -> String {
    if chars.get(*i) == Some(&'{') {
        let mut depth = 0;
        let start = *i + 1;
        while *i < chars.len() {
            match chars[*i] {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        *i += 1;
                        return chars[start..*i - 1].iter().collect();
                    }
                }
                _ => {}
            }
            *i += 1;
        }
        chars[start..].iter().collect()
    } else if let Some(&c) = chars.get(*i) {
        *i += 1;
        c.to_string()
    } else {
        String::new()
    }
}

fn script(text: &str, table: &[(char, char)], fallback: char) -> String {
    if text.chars().all(|c| table.iter().any(|(a, _)| *a == c)) {
        text.chars().map(|c| table.iter().find(|(a, _)| *a == c).expect("comprobado").1).collect()
    } else {
        format!("{fallback}{text}")
    }
}

/// `Periodo $T_{\mathrm{teo}}$ [\unit{m/s^2}]` → `Periodo T_teo [m/s²]`.
pub fn latex_to_plain(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '$' | '{' | '}' => i += 1,
            '~' => {
                out.push(' ');
                i += 1;
            }
            '^' => {
                i += 1;
                let g = latex_to_plain(&group(&chars, &mut i));
                out.push_str(&script(&g, &SUPERSCRIPTS, '^'));
            }
            '_' => {
                i += 1;
                let g = latex_to_plain(&group(&chars, &mut i));
                out.push_str(&script(&g, &SUBSCRIPTS, '_'));
            }
            '\\' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                if name.is_empty() {
                    // \, \; \! \{ etc.
                    if let Some(&c) = chars.get(i) {
                        i += 1;
                        match c {
                            ',' | ';' | ' ' => out.push(' '),
                            '{' | '}' | '%' | '$' | '&' | '_' => out.push(c),
                            _ => {}
                        }
                    }
                } else if let Some((_, s)) = SYMBOLS.iter().find(|(n, _)| *n == name) {
                    out.push_str(s);
                } else if name == "frac" {
                    let a = latex_to_plain(&group(&chars, &mut i));
                    let b = latex_to_plain(&group(&chars, &mut i));
                    out.push_str(&format!("{a}/{b}"));
                } else if matches!(name.as_str(), "quad" | "qquad") {
                    out.push(' ');
                }
                // Otros comandos (\mathrm, \unit, \left, \text…) se omiten y su
                // argumento se procesa como texto normal.
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out.replace("{,}", ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions() {
        assert_eq!(latex_to_plain("Periodo $T$ [\\unit{s}]"), "Periodo T [s]");
        assert_eq!(latex_to_plain("Modelo $T_{\\mathrm{teo}}$"), "Modelo T_teo");
        assert_eq!(latex_to_plain("$0{,}987\\,T_{\\mathrm{teo}}$"), "0,987 T_teo");
        assert_eq!(latex_to_plain("[\\unit{m/s^2}]"), "[m/s²]");
        assert_eq!(latex_to_plain("$\\omega_0 = 2\\pi f$"), "ω₀ = 2π f");
        assert_eq!(latex_to_plain("$x^{-1}$ y $10^{n}$"), "x⁻¹ y 10ⁿ");
        assert_eq!(latex_to_plain("$\\frac{a}{b}$"), "a/b");
        assert_eq!(latex_to_plain("Ajuste $T = 2\\pi\\sqrt{L/g}$"), "Ajuste T = 2π√L/g");
        assert_eq!(latex_to_plain("Sin LaTeX"), "Sin LaTeX");
    }
}
