# FisGraf y FisErr

Análisis de datos de física con calidad de publicación: **gráficos con ajustes y barras de error en LaTeX**, y **propagación de errores** según la GUM. La lógica numérica está escrita en Rust y validada con los datos certificados del NIST.

| | Para qué |
|---|---|
| ![FisGraf](graficos/assets/logo.png) **FisGraf** | Gráficos desde Excel o CSV: varias series, ajustes de modelos (seis métodos), incertidumbres de los parámetros (cinco métodos), R², residuos y salida en LaTeX/pgfplots (PDF, PNG, SVG, TikZ). Incluye una calculadora de propagación de errores. |
| ![FisErr](errores/assets/logo.png) **FisErr** | Calculadora de errores: ecuación teórica, mediciones con su error y constantes → valor final con su error y la función del error, con cinco métodos (derivadas parciales, segundo orden, Monte Carlo, Kragten y cota máxima). |
| **fisgraf-cli** | Las mismas funciones por línea de comandos (JSON) y como **servidor MCP**, para que una IA las use como herramienta. |

<p align="center"><img src="graficos/datos_de_prueba/graficos/01_caida_libre.png" width="560" alt="Ejemplo de gráfico de FisGraf: caída libre con ajuste y barras de error"></p>

## ⬇️ Descargas

Todo está en la **[última versión publicada (Releases)](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest)**. Son programas para Windows 10 u 11 que no necesitan instalación: descomprimes y haces doble clic.

| Archivo | Qué es |
|---|---|
| **[FisGraf-portable.zip](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/FisGraf-portable.zip)** | FisGraf (gráficos y errores) con su manual, proyectos de ejemplo y 8 experimentos de prueba. |
| **[FisErr-portable.zip](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/FisErr-portable.zip)** | FisErr (solo errores) con su manual. |
| **[FisGraf-para-Claude.zip](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/FisGraf-para-Claude.zip)** | La skill para Claude con la herramienta incluida (ver [Para IA](#-para-ia)). |
| **[fisgraf-cli.exe](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/fisgraf-cli.exe)** | La herramienta de línea de comandos y servidor MCP, sola. |

- FisGraf necesita [MiKTeX](https://miktex.org) (o TeX Live) solo para la vista LaTeX y para exportar los gráficos. FisErr no lo necesita.
- Si Windows muestra «Windows protegió su PC», pulsa *Más información → Ejecutar de todas formas* (los `.exe` no están firmados).

**📖 Manuales:** [MANUAL.pdf](MANUAL.pdf) (completo), [MANUAL_FisErr.pdf](MANUAL_FisErr.pdf) (solo FisErr) y [MANUAL.md](MANUAL.md) (para leer en GitHub).

## 🤖 Para IA

Una IA puede usar FisGraf como herramienta para calcular errores, ajustar modelos y crear gráficos. Obtiene los mismos números que las aplicaciones, redondeados según la GUM, y los gráficos con el mismo estilo LaTeX. Hay tres formas de conectarla:

### Claude Code: plugin (recomendado)

En Claude Code (terminal, o pestaña *Code* de la app de escritorio):

```text
/plugin marketplace add malvarezdam/FisGraf-FisErr
/plugin install fisgraf@fisgraf-fiserr
```

Instala la skill **fisgraf**. La primera vez que Claude la use, descarga `fisgraf-cli.exe` (2,8 MB) en `%USERPROFILE%\.fisgraf\`. Después basta con pedirle cosas como:

- «calcula el error de g = 4π²L/T² con L = 1,000 ± 0,001 m y T = 2,007 ± 0,005 s»
- «ajusta los datos de medidas.xlsx a una exponencial y dame τ con su error»
- «haz un gráfico con barras de error y R² de este CSV»

**Alternativa sin plugin:** descomprime [FisGraf-para-Claude.zip](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/FisGraf-para-Claude.zip) en la raíz de un proyecto (queda `.claude\skills\fisgraf\`, con el `.exe` incluido).

### Claude Desktop, Cursor, VS Code y otras apps: servidor MCP

`fisgraf-cli mcp` es un servidor [MCP](https://modelcontextprotocol.io) con cinco herramientas: `propagar_errores`, `ajustar_modelo`, `generar_grafico` (devuelve también el PNG para que la IA lo vea), `plantilla` y `referencia_proyecto`. Funciona también en el chat de Claude Desktop, donde una skill no puede ejecutar programas.

Instalación automática, en PowerShell:

```powershell
irm https://raw.githubusercontent.com/malvarezdam/FisGraf-FisErr/main/instalar.ps1 | iex
```

[`instalar.ps1`](instalar.ps1) hace lo siguiente:

1. Descarga `fisgraf-cli.exe` en `%USERPROFILE%\.fisgraf\`.
2. Si tienes Claude Desktop, lo registra en su configuración (guarda antes una copia `.bak`). Reinicia Claude Desktop después.
3. Muestra la configuración para otras aplicaciones.

Instalación manual: descarga [fisgraf-cli.exe](https://github.com/malvarezdam/FisGraf-FisErr/releases/latest/download/fisgraf-cli.exe) y agrega a la configuración MCP de tu aplicación:

```json
{
  "mcpServers": {
    "fisgraf": { "command": "C:\\ruta\\a\\fisgraf-cli.exe", "args": ["mcp"] }
  }
}
```

En Claude Code también se puede registrar así: `claude mcp add --scope user fisgraf -- C:\ruta\a\fisgraf-cli.exe mcp`.

### Cualquier agente: línea de comandos

```bash
fisgraf-cli errores  entrada.json            # propagación de errores
fisgraf-cli ajustar  entrada.json            # ajuste de un modelo (datos en JSON, CSV o Excel)
fisgraf-cli grafico  proyecto.fisgraf.json --formatos pdf,png
fisgraf-cli plantilla errores|ajustar|grafico
```

Entrada y salida en JSON. El formato está en [SKILL.md](plugin/skills/fisgraf/SKILL.md) y en [referencia.md](plugin/skills/fisgraf/referencia.md). [AGENTS.md](AGENTS.md) resume cómo usarlo para agentes de programación (Codex, Copilot, Cursor…).

## 🛠️ Compilar desde el código

Requisitos: [Rust](https://rustup.rs), y [MiKTeX](https://miktex.org) o TeX Live para los gráficos.

```bash
git clone https://github.com/malvarezdam/FisGraf-FisErr.git
```

```bash
cargo run --release -p fisgraf
```

```bash
cargo run --release -p fiserr
```

| Script (doble clic) | Qué hace |
|---|---|
| `graficos\crear_ejecutable.bat` · `errores\crear_ejecutable.bat` | Compila `FisGraf.exe` / `FisErr.exe` en su carpeta. |
| `crear_portable.bat` | Carpeta `portable\` y `dist\FisGraf-portable.zip`, `dist\FisErr-portable.zip`. |
| `crear_skill_portable.bat` | `dist\FisGraf-para-Claude.zip` y `dist\fisgraf-cli.exe`. |
| `crear_distribucion.bat` | Todo lo anterior: los archivos de una Release. |
| `manual\compilar.bat` | Los manuales en PDF. |

| Carpeta | Contenido |
|---|---|
| `graficos/` | FisGraf: la biblioteca con toda la lógica, la app y `fisgraf-cli` ([arquitectura e historial](graficos/PLAN.md)). |
| `errores/` | FisErr; usa la lógica de `graficos/`. |
| `plugin/` | Plugin de Claude Code con la skill `fisgraf`; `.claude-plugin/marketplace.json` publica este repositorio como marketplace. |
| `manual/` | Fuente LaTeX de los manuales. |

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets
```
