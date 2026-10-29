//! Ejemplos incluidos en el programa (no dependen de archivos externos).

use fisgraf::config::{InputDistribution, PropagationConfig, PropagationMethod, PropagationVariable};

fn measured(name: &str, value: f64, sigma: f64, unit: &str) -> PropagationVariable {
    PropagationVariable { name: name.into(), value, sigma, unit: unit.into(), ..Default::default() }
}

fn constant(name: &str, value: f64, unit: &str) -> PropagationVariable {
    PropagationVariable { name: name.into(), value, constant: true, unit: unit.into(), ..Default::default() }
}

fn calc(name: &str, unit: &str, expression: &str, variables: Vec<PropagationVariable>) -> PropagationConfig {
    PropagationConfig { name: name.into(), unit: unit.into(), expression: expression.into(), variables, ..Default::default() }
}

/// Gravedad con un péndulo: g = 4π²L/T² (regla de 1 mm, cronómetro).
pub fn pendulo() -> PropagationConfig {
    let mut l = measured("L", 1.000, 0.001 / (2.0 * 3f64.sqrt()), "m");
    l.distribution = InputDistribution::Uniform;
    calc("g", "m/s^2", "4*pi^2*L/T^2", vec![l, measured("T", 2.007, 0.005, "s")])
}

/// Densidad de un cilindro: ρ = 4m/(π d² h) (balanza y pie de metro).
pub fn densidad() -> PropagationConfig {
    let mut c = calc(
        "rho",
        "g/cm^3",
        "4*m/(pi*d^2*h)",
        vec![measured("m", 52.37, 0.01, "g"), measured("d", 2.012, 0.002, "cm"), measured("h", 6.04, 0.01, "cm")],
    );
    c.latex = "\\rho".into();
    c
}

/// Resistencias en paralelo: R = R₁R₂/(R₁ + R₂) (tolerancia del 1 %).
pub fn paralelo() -> PropagationConfig {
    let mut c = calc(
        "R",
        "\\ohm",
        "R1*R2/(R1 + R2)",
        vec![measured("R1", 1000.0, 10.0, "Ω"), measured("R2", 2200.0, 22.0, "Ω")],
    );
    for v in &mut c.variables {
        v.distribution = InputDistribution::Uniform;
    }
    c.method = PropagationMethod::MonteCarlo;
    c
}

/// Energía cinética: E = ½ m v².
pub fn energia() -> PropagationConfig {
    calc("E", "J", "0.5*m*v^2", vec![measured("m", 0.250, 0.001, "kg"), measured("v", 3.42, 0.05, "m/s")])
}

/// Índice de refracción por la ley de Snell, con ángulos en grados.
pub fn snell() -> PropagationConfig {
    let mut c = calc(
        "n",
        "",
        "n1*sin(t1*pi/180)/sin(t2*pi/180)",
        vec![constant("n1", 1.000293, ""), measured("t1", 40.0, 0.5, "°"), measured("t2", 25.3, 0.5, "°")],
    );
    c.variables[0].latex = "n_1".into();
    c.variables[1].latex = "\\theta_1".into();
    c.variables[2].latex = "\\theta_2".into();
    c.method = PropagationMethod::SecondOrder;
    c
}

/// Constante de un resorte por oscilaciones: k = 4π²m/T².
pub fn resorte() -> PropagationConfig {
    calc("k", "N/m", "4*pi^2*m/T^2", vec![measured("m", 0.200, 0.0005, "kg"), measured("T", 0.712, 0.004, "s")])
}

type Maker = fn() -> PropagationConfig;

pub const ALL: [(&str, Maker); 6] = [
    ("Péndulo: g = 4π²L/T²", pendulo),
    ("Densidad de un cilindro", densidad),
    ("Resistencias en paralelo", paralelo),
    ("Energía cinética", energia),
    ("Ley de Snell", snell),
    ("Constante de un resorte", resorte),
];
