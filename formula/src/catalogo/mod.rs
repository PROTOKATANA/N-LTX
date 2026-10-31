//! Pieza «catálogo»: qué significa cada comando y cada carácter (símbolo,
//! clase TeX, operadores grandes, funciones, acentos, delimitadores) y cómo
//! se escribe una letra en cada alfabeto matemático. El lector lo recibe
//! inyectado; otro catálogo (más símbolos, otra convención) encaja sin tocarlo.

mod latex;

pub use latex::CatalogoLatex;

use crate::modelo::{Alfabeto, Clase};

pub trait Catalogo {
    /// Comando de símbolo (`alpha`, `leq`…) → carácter y clase.
    fn simbolo(&self, nombre: &str) -> Option<(char, Clase)>;
    /// Operador grande (`sum`, `int`…) → carácter y si lleva límites encima
    /// y debajo en display.
    fn gran_operador(&self, nombre: &str) -> Option<(char, bool)>;
    /// Función con nombre (`sin`, `lim`…) → si lleva límites en display.
    fn funcion(&self, nombre: &str) -> Option<bool>;
    /// Acento (`hat`, `vec`…) → carácter combinante.
    fn acento(&self, nombre: &str) -> Option<char>;
    /// Delimitador tras \left, \right o \big (`(`, `\langle`…); `Some(None)`
    /// es «.» (sin delimitador).
    fn delimitador(&self, token: &str) -> Option<Option<char>>;
    /// Clase TeX de un carácter escrito tal cual.
    fn clase_caracter(&self, c: char) -> Clase;
    /// Una letra o dígito en un alfabeto matemático.
    fn estilizar(&self, c: char, alfabeto: Alfabeto) -> char;
}
