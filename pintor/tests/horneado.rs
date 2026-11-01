//! Lo horneado debe corresponder al HLSL: si no, cada arranque vuelve a compilarlo, sin avisar.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use pintor::puente_lienzo::ffi::TipoTinta;

const HLSL: &str = include_str!("../nucleo/sombreadores_lienzo.h");
const HORNEADO: &str = include_str!("../nucleo/sombreadores_horneados.h");

fn fuente(nombre: &str) -> &'static str {
    let (_, resto) = HLSL.split_once(&format!("{nombre} = R\"(")).unwrap();
    resto.split_once(")\"").unwrap().0
}

fn huella(texto: &str) -> u64 {
    texto.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Lo que hace `con_tipos`, en `recursos_lienzo.cc`.
fn con_tipos(texto: &str) -> String {
    let tipos = [
        ("TIPO_GLIFO", TipoTinta::Glifo),
        ("TIPO_LINEAL", TipoTinta::Lineal),
        ("TIPO_RADIAL", TipoTinta::Radial),
        ("TIPO_TRAZO", TipoTinta::Trazo),
    ];
    let definidos: String = tipos.iter().map(|(n, t)| format!("#define {n} {}\n", t.repr)).collect();
    definidos + texto
}

#[test]
fn lo_horneado_corresponde_al_hlsl() {
    let textos = [
        fuente("hlsl_lista_vs").to_string(),
        con_tipos(fuente("hlsl_lista_ps")),
        fuente("hlsl_marca_ps").to_string(),
        fuente("hlsl_imagen_ps").to_string(),
        fuente("hlsl_pantalla_vs").to_string(),
        fuente("hlsl_capa_ps").to_string(),
        fuente("hlsl_mezcla_ps").to_string(),
        fuente("hlsl_copia_ps").to_string(),
    ];
    for texto in &textos {
        let clave = format!("Horneado{{ 0x{:x}ull ,", huella(texto));
        assert!(
            HORNEADO.contains(&clave),
            "falta {clave}: hay que volver a hornear (pintor/README.md)"
        );
    }
}
