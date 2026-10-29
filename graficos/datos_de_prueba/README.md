# Datos de prueba

Mediciones **simuladas** de ocho experimentos de física, para probar FisGraf. Cada Excel trae los datos en la primera hoja y, en la hoja **Notas**, la descripción del experimento y los **valores reales** con que se generaron, para comparar con lo que da el programa.

Para abrirlos: **Archivo → Abrir datos de prueba** en FisGraf, o abre cualquier `.fisgraf.json` de `proyectos/`. En `graficos/` está el resultado esperado de cada uno (PDF y PNG).

| # | Experimento | Modelo | Qué opciones muestra |
|---|---|---|---|
| 01 | Caída libre (fotopuertas) | y = v₀t + ½gt² | Errores en x e y (varianza efectiva), **ecuación ajustada**, **R²** en el cuadro y en la leyenda, χ²_ν, residuos |
| 02 | Ley de Ohm (dos resistencias) | V = R·I | **Un ajuste aplicado a dos series**, marcadores distintos, línea discontinua, título por serie, leyenda debajo en 2 columnas, fuente Times |
| 03 | Carga de un condensador | V = ε(1 − e^(−t/τ)) | Incertidumbre por **bootstrap**, banda de confianza, función de referencia (asíntota), leyenda en posición personalizada |
| 04 | Oscilador amortiguado | x = A e^(−γt) cos(ωt + φ) | Modelo de 4 parámetros con **Nelder–Mead + LM**, envolventes de referencia, formato 16:9, cuadrícula secundaria |
| 05 | Enfriamiento de Newton | T = Tₐ + (T₀ − Tₐ)e^(−kt) | **Parámetro fijo** (Tₐ), error por **resolución** del termómetro, errores por **perfil de χ²**, fuente Palatino |
| 06 | Resonancia RLC | I = I₀/√(1 + Q²(f/f₀ − f₀/f)²) | **Eje x logarítmico** con marcas manuales, errores en **porcentaje**, fuente Charter |
| 07 | Efecto fotoeléctrico | V₀ = (h/e)ν − W/e | R², R² ajustado, **p-valor**, banda; en **Propagación**: h = (h/e)·e |
| 08 | Ley de Boyle | P = a/V | **ODR** (errores en x e y), s de los residuos, residuos, fuente Fira Sans; en **Propagación**: n = a/(RT) por **Monte Carlo** |

## Valores esperados

| # | Real | FisGraf (con estos datos) |
|---|---|---|
| 01 | g = 9,79 m/s² | g = 9,87 ± 0,12 m/s² |
| 02 | R₁ = 100 Ω, R₂ = 220 Ω | 99,6 ± 0,5 Ω y 220,2 ± 0,4 Ω |
| 03 | ε = 5,00 V, τ = 2,2 s | ε = 5,012 ± 0,006 V, τ = 2,213 ± 0,009 s |
| 04 | γ = 0,25 s⁻¹, ω = 3,1 rad/s | γ = 0,248 ± 0,005 s⁻¹, ω = 3,097 ± 0,005 rad/s |
| 05 | T₀ = 85 °C, k = 0,080 min⁻¹ | T₀ = 85,02 ± 0,20 °C, k = 0,0795 ± 0,0004 min⁻¹ |
| 06 | f₀ = 1000 Hz, Q = 4 | f₀ = 998,8 ± 2,5 Hz, Q = 3,95 ± 0,10 |
| 07 | h/e = 0,4136 V/(10¹⁴ Hz) | 0,4173 ± 0,0031 |
| 08 | nRT = 2452,6 kPa·mL | a = 2446 ± 8 kPa·mL |

Como en un laboratorio real, el ruido hace que el resultado no coincida exactamente con el valor real; lo que importa es que caiga dentro de su incertidumbre (casi siempre a menos de 2σ).

## Regenerar

```bash
cargo run --example generar_datos_prueba
```

Los datos salen siempre iguales (semilla fija). Para inventar otros, cambia los números de `noise(…)` en `examples/generar_datos_prueba.rs`.
