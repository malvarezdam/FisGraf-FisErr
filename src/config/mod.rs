//! Configuración completa de un proyecto. Se guarda como `.fisgraf.json`.
//!
//! Todos los structs usan `#[serde(default)]`, de modo que los proyectos
//! guardados con versiones anteriores siguen abriéndose al añadir campos.

pub mod axis;
pub mod export;
pub mod figure;
pub mod fit;
pub mod reference;
pub mod series;
pub mod style;

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub use axis::{AxisConfig, AxisScale, Notation, NumberFormat, TickConfig, TickDirection, TickSpacing};
pub use export::ExportConfig;
pub use figure::{
    AspectRatio, FigureConfig, FontChoice, GridConfig, LatexEngine, LegendConfig, LegendPosition,
    ResultsBoxConfig,
};
pub use fit::{
    BootstrapKind, CovarianceMode, FitConfig, FitMethod, FitSettings, ModelConfig, ParameterConfig,
    UncertaintyConfig, UncertaintyMethod,
};
pub use reference::{NamedValue, ReferenceFunction};
pub use series::{ErrorSpec, SeriesConfig};
pub use style::{DashPattern, LineStyle, Marker, Rgb};

/// Extensión de los archivos de proyecto.
pub const PROJECT_EXTENSION: &str = "fisgraf.json";

/// Versión del formato del archivo de proyecto.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    pub format_version: u32,
    /// Ruta del Excel. Se guarda relativa al archivo del proyecto cuando es posible.
    pub excel_path: Option<PathBuf>,
    pub series: Vec<SeriesConfig>,
    pub fits: Vec<FitConfig>,
    pub references: Vec<ReferenceFunction>,
    pub x_axis: AxisConfig,
    pub y_axis: AxisConfig,
    pub figure: FigureConfig,
    pub export: ExportConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            excel_path: None,
            series: Vec::new(),
            fits: Vec::new(),
            references: Vec::new(),
            x_axis: AxisConfig::default(),
            y_axis: AxisConfig::default(),
            figure: FigureConfig::default(),
            export: ExportConfig::default(),
        }
    }
}

impl ProjectConfig {
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let config: Self = serde_json::from_str(json).context("el archivo de proyecto no es válido")?;
        if config.format_version > FORMAT_VERSION {
            anyhow::bail!(
                "el proyecto usa el formato v{}, pero este programa solo entiende hasta v{}",
                config.format_version,
                FORMAT_VERSION
            );
        }
        Ok(config)
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Carga un proyecto. La ruta relativa del Excel se resuelve respecto al proyecto.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("no se pudo leer {}", path.display()))?;
        let mut config = Self::from_json(&json)?;
        if let (Some(excel), Some(dir)) = (&config.excel_path, path.parent())
            && excel.is_relative()
        {
            config.excel_path = Some(dir.join(excel));
        }
        Ok(config)
    }

    /// Guarda el proyecto. La ruta del Excel se guarda relativa al proyecto
    /// (con `..` si hace falta) cuando ambos están en la misma unidad, así la
    /// carpeta se puede mover o compartir entera.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let mut config = self.clone();
        if let (Some(excel), Some(dir)) = (&self.excel_path, path.parent())
            && let Some(relative) = relative_path(dir, excel)
        {
            config.excel_path = Some(relative);
        }
        std::fs::write(path, config.to_json()?)
            .with_context(|| format!("no se pudo escribir {}", path.display()))
    }

    /// Siguiente identificador libre, común a series, ajustes y referencias.
    pub fn next_id(&self) -> u32 {
        let series = self.series.iter().map(|s| s.id);
        let fits = self.fits.iter().map(|f| f.id);
        let refs = self.references.iter().map(|r| r.id);
        series.chain(fits).chain(refs).max().map_or(1, |m| m + 1)
    }

    /// Añade una serie con el siguiente color de la paleta y devuelve su id.
    pub fn add_series(&mut self) -> u32 {
        let id = self.next_id();
        let n = self.series.len();
        self.series.push(SeriesConfig {
            id,
            label: format!("Serie {}", n + 1),
            color: Rgb::palette(n),
            ..Default::default()
        });
        id
    }

    pub fn series_by_id(&self, id: u32) -> Option<&SeriesConfig> {
        self.series.iter().find(|s| s.id == id)
    }
}

/// Ruta de `target` vista desde la carpeta `base`. `None` si no comparten raíz
/// (otra unidad en Windows) o si alguna no es absoluta.
fn relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
    use std::path::Component;
    if !base.is_absolute() || !target.is_absolute() {
        return None;
    }
    let b: Vec<Component> = base.components().collect();
    let t: Vec<Component> = target.components().collect();
    // La raíz (unidad en Windows) debe coincidir.
    if b.first() != t.first() {
        return None;
    }
    let common = b.iter().zip(&t).take_while(|(x, y)| x == y).count();
    let mut rel = PathBuf::new();
    for _ in common..b.len() {
        rel.push("..");
    }
    for c in &t[common..] {
        rel.push(c.as_os_str());
    }
    Some(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths() {
        let root = std::env::temp_dir();
        let base = root.join("a").join("proyectos");
        let target = root.join("a").join("datos").join("x.xlsx");
        assert_eq!(relative_path(&base, &target), Some(PathBuf::from("..").join("datos").join("x.xlsx")));
        assert_eq!(relative_path(&base, &base.join("x.xlsx")), Some(PathBuf::from("x.xlsx")));
        assert_eq!(relative_path(Path::new("rel"), &target), None);
    }

    #[test]
    fn rgb_hex_roundtrip() {
        let c: Rgb = "#1F77b4".parse().unwrap();
        assert_eq!(c, Rgb(0x1f, 0x77, 0xb4));
        assert_eq!(c.to_hex(), "#1F77B4");
        assert!("1F77B4".parse::<Rgb>().is_err());
        assert!("#1F77B".parse::<Rgb>().is_err());
        assert!("#GG0000".parse::<Rgb>().is_err());
    }

    #[test]
    fn aspect_ratios() {
        assert!((AspectRatio::R4x3.height_over_width() - 0.75).abs() < 1e-12);
        assert!((AspectRatio::R9x16.height_over_width() - 16.0 / 9.0).abs() < 1e-12);
        let custom = AspectRatio::Custom { width: 2.0, height: 1.0 };
        assert!((custom.height_over_width() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn json_roundtrip() {
        let mut p = ProjectConfig::default();
        let a = p.add_series();
        let b = p.add_series();
        p.series[1].y_error = ErrorSpec::Range("Hoja1!C2:C10".into());
        p.fits.push(FitConfig {
            id: p.next_id(),
            series_ids: vec![a, b],
            method: FitMethod::Odr,
            ..Default::default()
        });
        p.x_axis.ticks.spacing = TickSpacing::Manual(vec![1.0, 2.5]);
        p.figure.aspect = AspectRatio::Custom { width: 5.0, height: 2.0 };

        let back = ProjectConfig::from_json(&p.to_json().unwrap()).unwrap();
        assert_eq!(back, p);
        assert_ne!(back.series[0].color, back.series[1].color);
        assert_eq!(p.next_id(), 4);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let p = ProjectConfig::from_json(r#"{ "series": [ { "id": 7 } ] }"#).unwrap();
        assert_eq!(p.series[0].id, 7);
        assert_eq!(p.series[0].marker, Marker::Circle);
        assert_eq!(p.figure.width_cm, 10.0);
    }

    #[test]
    fn rejects_newer_format() {
        assert!(ProjectConfig::from_json(r#"{ "format_version": 99 }"#).is_err());
    }

    #[test]
    fn save_and_load_relative_excel() {
        let dir = std::env::temp_dir().join(format!("fisgraf_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.join("p.fisgraf.json");
        let p = ProjectConfig {
            excel_path: Some(dir.join("datos.xlsx")),
            ..Default::default()
        };
        p.save(&project).unwrap();

        let raw = std::fs::read_to_string(&project).unwrap();
        assert!(raw.contains("\"datos.xlsx\""));
        assert_eq!(ProjectConfig::load(&project).unwrap().excel_path, p.excel_path);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
