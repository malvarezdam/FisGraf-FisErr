//! Compilación con LuaLaTeX o pdfLaTeX y lectura de errores del `.log`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::LatexEngine;

use super::RenderError;

/// Busca un ejecutable en el PATH.
pub fn find_program(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".bat", ".cmd", ""] } else { &[""] };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            exts.iter().map(|e| dir.join(format!("{name}{e}"))).find(|p| p.is_file())
        })
    })
}

pub fn engine_program(engine: LatexEngine) -> &'static str {
    match engine {
        LatexEngine::LuaLatex => "lualatex",
        LatexEngine::PdfLatex => "pdflatex",
    }
}

/// Ejecuta un programa sin abrir ventanas de consola y con límite de tiempo.
pub(crate) fn run(program: &str, args: &[&str], dir: &Path, timeout: Duration) -> Result<bool, RenderError> {
    let exe = find_program(program).ok_or_else(|| RenderError::MissingProgram(program.to_owned()))?;
    let mut cmd = Command::new(exe);
    cmd.args(args).current_dir(dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| RenderError::Io(format!("no se pudo ejecutar {program}: {e}")))?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| RenderError::Io(e.to_string()))? {
            return Ok(status.success());
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RenderError::Timeout { program: program.to_owned(), seconds: timeout.as_secs() });
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// Compila `tex` en `dir/{stem}.tex` y devuelve la ruta del PDF.
pub fn compile_pdf(tex: &str, engine: LatexEngine, dir: &Path, stem: &str, timeout: Duration) -> Result<PathBuf, RenderError> {
    std::fs::create_dir_all(dir).map_err(|e| RenderError::Io(e.to_string()))?;
    let tex_path = dir.join(format!("{stem}.tex"));
    let pdf = dir.join(format!("{stem}.pdf"));
    let log = dir.join(format!("{stem}.log"));
    std::fs::write(&tex_path, tex).map_err(|e| RenderError::Io(e.to_string()))?;
    let _ = std::fs::remove_file(&pdf);

    let file = format!("{stem}.tex");
    let ok = run(
        engine_program(engine),
        &["-interaction=nonstopmode", "-halt-on-error", "-file-line-error", &file],
        dir,
        timeout,
    )?;
    if ok && pdf.is_file() {
        return Ok(pdf);
    }
    let log_text = std::fs::read(&log).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    Err(match parse_log(&log_text) {
        Some((line, message, context)) => RenderError::Latex {
            line,
            message,
            source_line: line.and_then(|l| tex.lines().nth(l.saturating_sub(1))).map(|s| shorten(s.trim())),
            context,
        },
        None => RenderError::Latex {
            line: None,
            message: "la compilación falló sin un mensaje reconocible; revisa el archivo .log".into(),
            source_line: None,
            context: String::new(),
        },
    })
}

fn shorten(s: &str) -> String {
    if s.chars().count() > 160 { format!("{}…", s.chars().take(160).collect::<String>()) } else { s.to_owned() }
}

/// Primer error del log: (línea, mensaje, contexto).
pub fn parse_log(log: &str) -> Option<(Option<usize>, String, String)> {
    let lines: Vec<&str> = log.lines().collect();
    let (idx, line, message) = lines.iter().enumerate().find_map(|(i, l)| {
        // Formato -file-line-error: ./grafico.tex:42: Undefined control sequence.
        if let Some(pos) = l.find(".tex:") {
            let rest = &l[pos + 5..];
            let (num, msg) = rest.split_once(": ")?;
            let n: usize = num.parse().ok()?;
            return Some((i, Some(n), msg.trim().to_owned()));
        }
        l.strip_prefix("! ").map(|m| (i, None, m.trim().to_owned()))
    })?;
    // Contexto: hasta la línea "l.<n> ..." que muestra dónde se detuvo TeX.
    let mut context = Vec::new();
    let mut line = line;
    for l in lines.iter().skip(idx + 1).take(12) {
        if l.trim().is_empty() {
            continue;
        }
        context.push(l.trim_end());
        if let Some(rest) = l.strip_prefix("l.") {
            if line.is_none() {
                line = rest.split_whitespace().next().and_then(|n| n.parse().ok());
            }
            break;
        }
    }
    Some((line, message, context.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_line_errors() {
        let log = "This is LuaHBTeX\n./grafico.tex:42: Undefined control sequence.\nl.42 \\addlegendentry{$\\foo$}\n\n";
        let (line, msg, ctx) = parse_log(log).unwrap();
        assert_eq!(line, Some(42));
        assert_eq!(msg, "Undefined control sequence.");
        assert!(ctx.contains("\\foo"));

        let log = "! Missing $ inserted.\n<inserted text>\n$\nl.17 x_1\n";
        let (line, msg, _) = parse_log(log).unwrap();
        assert_eq!((line, msg.as_str()), (Some(17), "Missing $ inserted."));
        assert!(parse_log("todo bien").is_none());
    }
}
