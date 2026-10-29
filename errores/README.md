# FisErr: calculadora de errores

![Logo de FisErr](assets/logo.png)

Calcula el error de una magnitud a partir de mediciones. Escribes la **ecuación teórica**, indicas qué nombres son **variables medidas** (con su valor y su error) y cuáles son **constantes**, y obtienes **la función final con su error**.

Usa exactamente la misma lógica que la pestaña Propagación de FisGraf (`fisgraf::propagation`), así que los resultados coinciden siempre.

## Abrir

Doble clic en **`FisErr.exe`**. Si no existe, doble clic en **`crear_ejecutable.bat`** para compilarlo. Desde la terminal:

```bash
cargo run --release -p fiserr
```

## Cómo se usa

1. Escribe la **ecuación**, p. ej. `4*pi^2*L/T^2`, y pulsa **Detectar variables**.
2. En la tabla, marca cada nombre como **Variable medida** (valor, σ y distribución) o como **Constante** (sin error).
   - σ es la incertidumbre estándar. Si solo conoces la resolución r del instrumento, usa σ = r/(2√3) con distribución **uniforme**.
   - Si dos mediciones no son independientes, indica su correlación ρ en **Correlaciones**.
3. Elige el **método**:

   | Método | Cuándo |
   |---|---|
   | Lineal (derivadas parciales, GUM) | Casi siempre: σ_f = √(Σ (∂f/∂xᵢ)² σᵢ²). |
   | Segundo orden (GUM §5.1.2) | f muy curva o errores relativos grandes. |
   | Monte Carlo (GUM Suplemento 1) | El más fiable sin suponer linealidad; da intervalos asimétricos. |
   | Kragten | Equivalente numérico del lineal (método de EURACHEM). |
   | Cota máxima | El «error máximo» Σ\|∂f/∂xᵢ\|σᵢ de los cursos introductorios; es una cota, no una σ. |

4. **Resultado:**
   - el valor con su error, redondeado según la GUM (p. ej. `g = 9,80 ± 0,05 m/s²`), con el error relativo y la incertidumbre expandida;
   - la **función final con su error**: f(x) y σ_f(x) en forma simbólica, p. ej. `σ_g = √((4π²/T²)²σ_L² + (−8π²L/T³)²σ_T²)`;
   - la comparación de los cinco métodos;
   - el **presupuesto de incertidumbre**, que indica cuánto aporta cada medición (mejora primero la de mayor %);
   - el histograma de Monte Carlo y las derivadas parciales;
   - botones para copiar el resultado, la fórmula de propagación o σ_f en LaTeX.

**Ejemplos incluidos** (menú *Ejemplos*): péndulo, densidad de un cilindro, resistencias en paralelo, energía cinética, ley de Snell y constante de un resorte.

Los cálculos se guardan como `.fiserr.json` (*Archivo → Guardar*). *Ver → Coma decimal* cambia entre `9,80` y `9.80`.

## Sintaxis de la ecuación

- Operadores `+ - * / ^`.
- Funciones `sin cos tan asin acos atan sinh cosh tanh exp ln log10 sqrt abs`; constantes `pi` y `e`.
- `log` es el logaritmo natural.
- Los ángulos van en radianes: para grados, escribe `sin(t*pi/180)`.
