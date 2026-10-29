//! Incrusta el ícono en el ejecutable de Windows (se ve en el Explorador y en
//! la barra de tareas). En otros sistemas no hace nada.

fn main() {
    println!("cargo:rerun-if-changed=assets/fiserr.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/fiserr.ico");
        res.set("ProductName", "FisErr");
        res.set("FileDescription", "FisErr: calculadora de propagación de errores");
        // Si no se encuentra el compilador de recursos, el programa funciona igual sin ícono.
        if let Err(e) = res.compile() {
            println!("cargo:warning=no se pudo incrustar el ícono: {e}");
        }
    }
}
