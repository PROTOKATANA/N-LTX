//! Las imágenes de los documentos: PNG, JPEG y SVG, leídas de carpetas concretas y decodificadas una
//! sola vez. `primitivas` y el pintor solo ven píxeles.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::SystemTime;

use documento::{Imagen, Imagenes};
use primitivas::MapaDeBits;
use resvg::{tiny_skia, usvg};
use zune_jpeg::JpegDecoder;

/// Lo que puede medir una textura en cualquier GPU razonable. Lo que pase se reduce al leerlo.
const LADO_MAXIMO: u32 = 8192;
/// Píxeles por bp de un SVG: se ve nítido hasta tres aumentos.
const DENSIDAD_SVG: f32 = 3.0;
/// Lo que puede ocupar un PNG al leerlo. El tope por defecto, 64 MiB, no llega a una foto de 25 Mpx.
const MEMORIA_PNG: usize = 1 << 30;
/// `\includegraphics{foto}` no dice la extensión.
const EXTENSIONES: [&str; 4] = ["png", "jpg", "jpeg", "svg"];
/// Imágenes recordadas, como mucho: es un tope de seguridad, un documento lleva muchas menos.
const RECORDADAS: usize = 256;

/// Una imagen ya leída, mientras su archivo no cambie.
struct Recordada {
    modificada: Option<SystemTime>,
    imagen: Result<Imagen, String>,
}

/// Lee imágenes de `carpetas`, en ese orden. La fuente de un documento puede venir de un modelo de
/// lenguaje: una ruta absoluta, con `..` o que salga de las carpetas por un enlace no se lee.
pub struct ImagenesEnCarpetas {
    carpetas: Vec<PathBuf>,
    memoria: Mutex<HashMap<PathBuf, Recordada>>,
}

impl ImagenesEnCarpetas {
    pub fn nuevas(carpetas: Vec<PathBuf>) -> ImagenesEnCarpetas {
        ImagenesEnCarpetas {
            carpetas,
            memoria: Mutex::default(),
        }
    }

    /// El archivo de `ruta`, si está dentro de alguna carpeta.
    fn buscar(&self, ruta: &str) -> Option<PathBuf> {
        let relativa = Path::new(ruta);
        let legal = relativa.components().all(|c| matches!(c, Component::Normal(_)));
        if !legal {
            return None;
        }
        self.carpetas.iter().find_map(|carpeta| {
            let raiz = carpeta.canonicalize().ok()?;
            let dentro = candidatos(relativa)
                .into_iter()
                .find_map(|c| raiz.join(c).canonicalize().ok())?;
            (dentro.starts_with(&raiz) && dentro.is_file()).then_some(dentro)
        })
    }

    fn leer(&self, archivo: PathBuf) -> Result<Imagen, String> {
        let modificada = std::fs::metadata(&archivo).and_then(|m| m.modified()).ok();
        let mut memoria = self.memoria.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(r) = memoria.get(&archivo).filter(|r| r.modificada == modificada) {
            return r.imagen.clone();
        }
        let imagen = std::fs::read(&archivo)
            .map_err(|e| e.to_string())
            .and_then(|b| decodificar(&b));
        if memoria.len() >= RECORDADAS {
            memoria.clear();
        }
        let recordada = Recordada {
            modificada,
            imagen: imagen.clone(),
        };
        memoria.insert(archivo, recordada);
        imagen
    }
}

/// `ruta` tal cual si dice su extensión; si no, con cada una de las que se leen.
fn candidatos(ruta: &Path) -> Vec<PathBuf> {
    if ruta.extension().is_some() {
        return vec![ruta.to_path_buf()];
    }
    EXTENSIONES.iter().map(|e| ruta.with_extension(e)).collect()
}

impl Imagenes for ImagenesEnCarpetas {
    fn imagen(&self, ruta: &str) -> Result<Imagen, String> {
        let archivo = self
            .buscar(ruta)
            .ok_or_else(|| format!("no se encuentra la imagen {ruta}"))?;
        self.leer(archivo).map_err(|e| format!("{ruta}: {e}"))
    }
}

/// Una imagen en memoria. El formato se mira en sus primeros bytes: la extensión puede mentir.
pub fn decodificar(bytes: &[u8]) -> Result<Imagen, String> {
    if bytes.starts_with(b"\x89PNG") {
        return de_png(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return de_jpeg(bytes);
    }
    if parece_svg(bytes) {
        return de_svg(bytes);
    }
    Err("formato no admitido: solo PNG, JPEG y SVG".into())
}

fn parece_svg(bytes: &[u8]) -> bool {
    let inicio = &bytes[..bytes.len().min(4096)];
    inicio.windows(4).any(|v| v == b"<svg")
}

/// Un píxel por bp, como `graphicx` con una imagen sin resolución.
fn imagen_de(ancho: u32, alto: u32, rgba: Vec<u8>) -> Imagen {
    let natural = (f64::from(ancho), f64::from(alto));
    let mapa = reducida(ancho, alto, rgba);
    Imagen {
        mapa: Arc::new(mapa),
        ancho: natural.0,
        alto: natural.1,
    }
}

fn identificada(ancho: u32, alto: u32, rgba: Vec<u8>) -> MapaDeBits {
    let mut h = DefaultHasher::new();
    (ancho, alto).hash(&mut h);
    rgba.hash(&mut h);
    MapaDeBits {
        id: h.finish(),
        ancho,
        alto,
        rgba,
    }
}

/// Si pasa de [`LADO_MAXIMO`], la media de cada bloque de píxeles.
fn reducida(ancho: u32, alto: u32, rgba: Vec<u8>) -> MapaDeBits {
    let paso = ancho.max(alto).div_ceil(LADO_MAXIMO).max(1);
    if paso == 1 {
        return identificada(ancho, alto, rgba);
    }
    let (w, h) = ((ancho / paso).max(1), (alto / paso).max(1));
    let mut salida = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut suma = [0u32; 4];
            for fila in y * paso..((y + 1) * paso).min(alto) {
                for columna in x * paso..((x + 1) * paso).min(ancho) {
                    let i = ((fila * ancho + columna) * 4) as usize;
                    for (s, v) in suma.iter_mut().zip(&rgba[i..i + 4]) {
                        *s += u32::from(*v);
                    }
                }
            }
            let cuenta = paso * paso;
            salida.extend(suma.map(|s| (s / cuenta) as u8));
        }
    }
    identificada(w, h, salida)
}

fn premultiplicar(rgba: &mut [u8]) {
    for p in rgba.chunks_exact_mut(4) {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

fn de_png(bytes: &[u8]) -> Result<Imagen, String> {
    let limites = png::Limits { bytes: MEMORIA_PNG };
    let mut decodificador = png::Decoder::new_with_limits(bytes, limites);
    decodificador.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut lector = decodificador.read_info().map_err(|e| e.to_string())?;
    let mut crudo = vec![0; lector.output_buffer_size()];
    let info = lector.next_frame(&mut crudo).map_err(|e| e.to_string())?;
    crudo.truncate(info.buffer_size());
    let mut rgba = match info.color_type {
        png::ColorType::Rgba => crudo,
        png::ColorType::GrayscaleAlpha => crudo.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        otro => return Err(format!("PNG en un color inesperado: {otro:?}")),
    };
    premultiplicar(&mut rgba);
    Ok(imagen_de(info.width, info.height, rgba))
}

fn de_jpeg(bytes: &[u8]) -> Result<Imagen, String> {
    let mut decodificador = JpegDecoder::new(bytes);
    let rgb = decodificador.decode().map_err(|e| e.to_string())?;
    let (ancho, alto) = decodificador.dimensions().ok_or("JPEG sin dimensiones")?;
    let rgba = rgb.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect();
    Ok(imagen_de(ancho as u32, alto as u32, rgba))
}

fn de_svg(bytes: &[u8]) -> Result<Imagen, String> {
    let arbol = usvg::Tree::from_data(bytes, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let tam = arbol.size();
    let densidad = DENSIDAD_SVG.min(LADO_MAXIMO as f32 / tam.width().max(tam.height()));
    let (ancho, alto) = (
        (tam.width() * densidad).ceil() as u32,
        (tam.height() * densidad).ceil() as u32,
    );
    let mut lienzo = tiny_skia::Pixmap::new(ancho, alto).ok_or("SVG sin tamaño")?;
    let escala = tiny_skia::Transform::from_scale(densidad, densidad);
    resvg::render(&arbol, escala, &mut lienzo.as_mut());
    Ok(Imagen {
        mapa: Arc::new(identificada(ancho, alto, lienzo.take())),
        ancho: f64::from(tam.width()),
        alto: f64::from(tam.height()),
    })
}

/// Un PNG de una figura. Una animación lo vuelve a pedir en cada fotograma: se decodifica una vez.
pub(crate) fn mapa_de_png(bytes: &[u8]) -> Result<Arc<MapaDeBits>, String> {
    static RECUERDOS: OnceLock<Mutex<HashMap<u64, Arc<MapaDeBits>>>> = OnceLock::new();
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    let clave = h.finish();
    let mut recuerdos = RECUERDOS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(mapa) = recuerdos.get(&clave) {
        return Ok(Arc::clone(mapa));
    }
    let mapa = de_png(bytes)?.mapa;
    if recuerdos.len() >= RECORDADAS {
        recuerdos.clear();
    }
    recuerdos.insert(clave, Arc::clone(&mapa));
    Ok(mapa)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un PNG RGBA de `ancho` × `alto` con todos los píxeles iguales.
    fn png_de(ancho: u32, alto: u32, pixel: [u8; 4]) -> Vec<u8> {
        let mut salida = Vec::new();
        let mut e = png::Encoder::new(&mut salida, ancho, alto);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&pixel.repeat((ancho * alto) as usize)).unwrap();
        w.finish().unwrap();
        salida
    }

    fn carpeta(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("nltx-imagenes-{}-{nombre}", std::process::id()));
        std::fs::create_dir_all(&c).unwrap();
        c
    }

    #[test]
    fn un_png_llega_con_el_alfa_multiplicado_y_a_un_pixel_por_bp() {
        let i = decodificar(&png_de(3, 2, [200, 100, 0, 128])).unwrap();
        assert_eq!((i.ancho, i.alto), (3.0, 2.0));
        assert_eq!((i.mapa.ancho, i.mapa.alto), (3, 2));
        assert_eq!(&i.mapa.rgba[..4], [100, 50, 0, 128]);
    }

    #[test]
    fn un_svg_se_rasteriza_con_mas_pixeles_que_bp() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20" fill="#ff0000"/></svg>"##;
        let i = decodificar(svg).unwrap();
        assert_eq!((i.ancho, i.alto), (10.0, 20.0));
        assert_eq!((i.mapa.ancho, i.mapa.alto), (30, 60));
        assert_eq!(&i.mapa.rgba[..4], [255, 0, 0, 255]);
    }

    #[test]
    fn un_jpeg_se_lee_opaco() {
        let i = decodificar(include_bytes!("../../ejemplo/assets/ondas.jpg")).unwrap();
        assert_eq!((i.mapa.ancho, i.mapa.alto), (640, 360));
        assert_eq!((i.ancho, i.alto), (640.0, 360.0));
        assert_eq!(i.mapa.rgba.len(), 640 * 360 * 4);
        assert!(i.mapa.rgba.chunks_exact(4).all(|p| p[3] == 255));
    }

    #[test]
    fn un_png_con_transparencia_y_un_svg_de_verdad() {
        let i = decodificar(include_bytes!("../../ejemplo/assets/julia.png")).unwrap();
        assert_eq!(i.mapa.rgba[3], 0, "la esquina es transparente");
        let i = decodificar(include_bytes!("../../ejemplo/assets/diagrama.svg")).unwrap();
        assert_eq!((i.ancho, i.alto), (320.0, 120.0));
        assert_eq!((i.mapa.ancho, i.mapa.alto), (960, 360));
    }

    #[test]
    fn lo_que_pasa_del_lado_maximo_se_reduce_y_conserva_su_tamano_natural() {
        let (ancho, alto) = (LADO_MAXIMO + 2, 4);
        let rgba = vec![10; (ancho * alto * 4) as usize];
        let i = imagen_de(ancho, alto, rgba);
        assert!(i.mapa.ancho <= LADO_MAXIMO, "{}", i.mapa.ancho);
        assert_eq!(i.ancho, f64::from(ancho));
        assert_eq!(i.mapa.rgba[0], 10);
    }

    #[test]
    fn solo_se_lee_dentro_de_las_carpetas() {
        let raiz = carpeta("permisos");
        let dentro = raiz.join("dentro");
        std::fs::create_dir_all(&dentro).unwrap();
        std::fs::write(raiz.join("fuera.png"), png_de(1, 1, [0, 0, 0, 255])).unwrap();
        std::fs::write(dentro.join("a.png"), png_de(1, 1, [0, 0, 0, 255])).unwrap();
        std::os::unix::fs::symlink(raiz.join("fuera.png"), dentro.join("enlace.png")).unwrap();
        let i = ImagenesEnCarpetas::nuevas(vec![dentro]);
        assert!(i.imagen("a.png").is_ok());
        assert!(i.imagen("a").is_ok(), "sin extensión");
        assert!(i.imagen("../fuera.png").is_err());
        assert!(i.imagen(raiz.join("fuera.png").to_str().unwrap()).is_err());
        assert!(i.imagen("enlace.png").is_err());
    }

    #[test]
    fn un_archivo_que_cambia_se_vuelve_a_leer() {
        let raiz = carpeta("cambios");
        let archivo = raiz.join("a.png");
        std::fs::write(&archivo, png_de(1, 1, [0, 0, 0, 255])).unwrap();
        let i = ImagenesEnCarpetas::nuevas(vec![raiz]);
        let antes = i.imagen("a.png").unwrap();
        assert!(Arc::ptr_eq(&antes.mapa, &i.imagen("a.png").unwrap().mapa));
        std::fs::write(&archivo, png_de(2, 1, [0, 0, 0, 255])).unwrap();
        let hora = SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&archivo)
            .unwrap()
            .set_modified(hora)
            .unwrap();
        assert_eq!(i.imagen("a.png").unwrap().mapa.ancho, 2);
    }

    #[test]
    fn un_pdf_se_rechaza_con_un_motivo() {
        let e = decodificar(b"%PDF-1.7").unwrap_err();
        assert!(e.contains("PNG"), "{e}");
    }
}
