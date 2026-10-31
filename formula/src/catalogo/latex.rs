//! El catálogo de LaTeX/KaTeX: tablas de comandos → carácter Unicode y clase,
//! y los alfabetos matemáticos de Unicode (cursiva, negrita, pizarra…).

use super::Catalogo;
use crate::modelo::{Alfabeto, Clase};

/// Catálogo estándar (el subconjunto de KaTeX/MathJax que usan Wikipedia y
/// arXiv, más mhchem y los símbolos de AMS).
#[derive(Clone, Copy, Debug, Default)]
pub struct CatalogoLatex;

impl Catalogo for CatalogoLatex {
    fn simbolo(&self, nombre: &str) -> Option<(char, Clase)> {
        simbolo(nombre)
    }
    fn gran_operador(&self, nombre: &str) -> Option<(char, bool)> {
        gran_operador(nombre)
    }
    fn funcion(&self, nombre: &str) -> Option<bool> {
        funcion(nombre)
    }
    fn acento(&self, nombre: &str) -> Option<char> {
        acento(nombre)
    }
    fn delimitador(&self, token: &str) -> Option<Option<char>> {
        delimitador(token)
    }
    fn clase_caracter(&self, c: char) -> Clase {
        clase_caracter(c)
    }
    fn estilizar(&self, c: char, alfabeto: Alfabeto) -> char {
        estilizar(c, alfabeto)
    }
}

/// Carácter de una letra o dígito ASCII en un alfabeto matemático Unicode
/// (bloque «Mathematical Alphanumeric Symbols», con sus huecos históricos
/// que remiten a «Letterlike Symbols»).
fn estilizar(c: char, alfabeto: Alfabeto) -> char {
    let desde = |base: u32, de: char| char::from_u32(base + (c as u32 - de as u32));
    let mayus = c.is_ascii_uppercase();
    let minus = c.is_ascii_lowercase();
    let digito = c.is_ascii_digit();
    let r = match alfabeto {
        Alfabeto::Recta => None,
        Alfabeto::Cursiva => {
            if c == 'h' {
                Some('ℎ')
            } else if mayus {
                desde(0x1D434, 'A')
            } else if minus {
                desde(0x1D44E, 'a')
            } else {
                griega_cursiva(c)
            }
        }
        Alfabeto::Negrita => {
            if mayus {
                desde(0x1D400, 'A')
            } else if minus {
                desde(0x1D41A, 'a')
            } else if digito {
                desde(0x1D7CE, '0')
            } else {
                None
            }
        }
        Alfabeto::NegritaCursiva => {
            if mayus {
                desde(0x1D468, 'A')
            } else if minus {
                desde(0x1D482, 'a')
            } else if digito {
                desde(0x1D7CE, '0')
            } else {
                None
            }
        }
        Alfabeto::Pizarra => match c {
            'C' => Some('ℂ'),
            'H' => Some('ℍ'),
            'N' => Some('ℕ'),
            'P' => Some('ℙ'),
            'Q' => Some('ℚ'),
            'R' => Some('ℝ'),
            'Z' => Some('ℤ'),
            _ if mayus => desde(0x1D538, 'A'),
            _ if minus => desde(0x1D552, 'a'),
            _ if digito => desde(0x1D7D8, '0'),
            _ => None,
        },
        Alfabeto::Caligrafica => match c {
            'B' => Some('ℬ'),
            'E' => Some('ℰ'),
            'F' => Some('ℱ'),
            'H' => Some('ℋ'),
            'I' => Some('ℐ'),
            'L' => Some('ℒ'),
            'M' => Some('ℳ'),
            'R' => Some('ℛ'),
            'e' => Some('ℯ'),
            'g' => Some('ℊ'),
            'o' => Some('ℴ'),
            _ if mayus => desde(0x1D49C, 'A'),
            _ if minus => desde(0x1D4B6, 'a'),
            _ => None,
        },
        Alfabeto::Fraktur => match c {
            'C' => Some('ℭ'),
            'H' => Some('ℌ'),
            'I' => Some('ℑ'),
            'R' => Some('ℜ'),
            'Z' => Some('ℨ'),
            _ if mayus => desde(0x1D504, 'A'),
            _ if minus => desde(0x1D51E, 'a'),
            _ => None,
        },
        Alfabeto::SinSerifa => {
            if mayus {
                desde(0x1D5A0, 'A')
            } else if minus {
                desde(0x1D5BA, 'a')
            } else if digito {
                desde(0x1D7E2, '0')
            } else {
                None
            }
        }
        Alfabeto::Mono => {
            if mayus {
                desde(0x1D670, 'A')
            } else if minus {
                desde(0x1D68A, 'a')
            } else if digito {
                desde(0x1D7F6, '0')
            } else {
                None
            }
        }
    };
    r.unwrap_or(c)
}

/// Griega minúscula → cursiva matemática (en TeX las minúsculas griegas son
/// cursivas y las mayúsculas rectas).
fn griega_cursiva(c: char) -> Option<char> {
    match c {
        'α'..='ω' => char::from_u32(0x1D6FC + (c as u32 - 'α' as u32)),
        'ϵ' => Some('𝜖'),
        'ϑ' => Some('𝜗'),
        'ϰ' => Some('𝜘'),
        'ϕ' => Some('𝜙'),
        'ϱ' => Some('𝜚'),
        'ϖ' => Some('𝜛'),
        _ => None,
    }
}

/// Clase TeX de un carácter escrito directamente en la fórmula.
fn clase_caracter(c: char) -> Clase {
    match c {
        '+' | '−' | '*' | '∗' | '±' | '∓' | '×' | '÷' | '⋅' | '∘' | '∙' | '∩' | '∪' | '∧' | '∨' | '⊕' | '⊗' | '⊖'
        | '⊙' => Clase::Bin,
        '=' | '<' | '>' | ':' | '≤' | '≥' | '≠' | '≈' | '≡' | '∼' | '→' | '←' | '↔' | '⇒' | '⇐' | '⇔' | '↦' | '∈'
        | '∉' | '⊂' | '⊃' | '⊆' | '⊇' => Clase::Rel,
        '(' | '[' | '{' | '⟨' | '⌊' | '⌈' => Clase::Open,
        ')' | ']' | '}' | '⟩' | '⌋' | '⌉' | '!' | '?' => Clase::Close,
        ',' | ';' => Clase::Punct,
        _ => Clase::Ord,
    }
}

/// Símbolo de un comando sin argumentos: (carácter, clase).
fn simbolo(nombre: &str) -> Option<(char, Clase)> {
    use Clase::*;
    // griegas: minúsculas (se pasan a cursiva al usarlas) y mayúsculas rectas
    let griega = match nombre {
        "alpha" => Some('α'),
        "beta" => Some('β'),
        "gamma" => Some('γ'),
        "delta" => Some('δ'),
        "epsilon" => Some('ϵ'),
        "varepsilon" => Some('ε'),
        "zeta" => Some('ζ'),
        "eta" => Some('η'),
        "theta" => Some('θ'),
        "vartheta" => Some('ϑ'),
        "iota" => Some('ι'),
        "kappa" => Some('κ'),
        "varkappa" => Some('ϰ'),
        "lambda" => Some('λ'),
        "mu" => Some('μ'),
        "nu" => Some('ν'),
        "xi" => Some('ξ'),
        "omicron" => Some('ο'),
        "pi" => Some('π'),
        "varpi" => Some('ϖ'),
        "rho" => Some('ρ'),
        "varrho" => Some('ϱ'),
        "sigma" => Some('σ'),
        "varsigma" => Some('ς'),
        "tau" => Some('τ'),
        "upsilon" => Some('υ'),
        "phi" => Some('ϕ'),
        "varphi" => Some('φ'),
        "chi" => Some('χ'),
        "psi" => Some('ψ'),
        "omega" => Some('ω'),
        "Gamma" => Some('Γ'),
        "Delta" => Some('Δ'),
        "Theta" => Some('Θ'),
        "Lambda" => Some('Λ'),
        "Xi" => Some('Ξ'),
        "Pi" => Some('Π'),
        "Sigma" => Some('Σ'),
        "Upsilon" => Some('Υ'),
        "Phi" => Some('Φ'),
        "Psi" => Some('Ψ'),
        "Omega" => Some('Ω'),
        _ => None,
    };
    if let Some(c) = griega {
        return Some((c, Ord));
    }
    Some(match nombre {
        // binarios
        "pm" => ('±', Bin),
        "mp" => ('∓', Bin),
        "times" => ('×', Bin),
        "div" => ('÷', Bin),
        "cdot" => ('⋅', Bin),
        "ast" => ('∗', Bin),
        "star" => ('⋆', Bin),
        "circ" => ('∘', Bin),
        "bullet" => ('∙', Bin),
        "cap" => ('∩', Bin),
        "cup" => ('∪', Bin),
        "wedge" | "land" => ('∧', Bin),
        "vee" | "lor" => ('∨', Bin),
        "setminus" => ('∖', Bin),
        "oplus" => ('⊕', Bin),
        "ominus" => ('⊖', Bin),
        "otimes" => ('⊗', Bin),
        "oslash" => ('⊘', Bin),
        "odot" => ('⊙', Bin),
        "dagger" => ('†', Bin),
        "ddagger" => ('‡', Bin),
        "uplus" => ('⊎', Bin),
        "sqcap" => ('⊓', Bin),
        "sqcup" => ('⊔', Bin),
        "amalg" => ('⨿', Bin),
        "triangleright" => ('▷', Bin),
        "triangleleft" => ('◁', Bin),
        "smallsetminus" => ('∖', Bin),
        "boxtimes" => ('⊠', Bin),
        "boxplus" => ('⊞', Bin),
        "boxminus" => ('⊟', Bin),
        "boxdot" => ('⊡', Bin),
        "intercal" => ('⊺', Bin),
        "rtimes" => ('⋊', Bin),
        "ltimes" => ('⋉', Bin),
        "wr" => ('≀', Bin),
        "divideontimes" => ('⋇', Bin),
        "dotplus" => ('∔', Bin),
        "centerdot" => ('⋅', Bin),
        // relaciones
        "leq" | "le" => ('≤', Rel),
        "geq" | "ge" => ('≥', Rel),
        "leqslant" => ('⩽', Rel),
        "geqslant" => ('⩾', Rel),
        "neq" | "ne" => ('≠', Rel),
        "approx" => ('≈', Rel),
        "equiv" => ('≡', Rel),
        "sim" => ('∼', Rel),
        "simeq" => ('≃', Rel),
        "cong" => ('≅', Rel),
        "propto" => ('∝', Rel),
        "ll" => ('≪', Rel),
        "gg" => ('≫', Rel),
        "in" => ('∈', Rel),
        "notin" => ('∉', Rel),
        "ni" | "owns" => ('∋', Rel),
        "subset" => ('⊂', Rel),
        "supset" => ('⊃', Rel),
        "subseteq" => ('⊆', Rel),
        "supseteq" => ('⊇', Rel),
        "subsetneq" => ('⊊', Rel),
        "supsetneq" => ('⊋', Rel),
        "to" | "rightarrow" => ('→', Rel),
        "leftarrow" | "gets" => ('←', Rel),
        "leftrightarrow" => ('↔', Rel),
        "Rightarrow" => ('⇒', Rel),
        "Leftarrow" => ('⇐', Rel),
        "Leftrightarrow" | "iff" => ('⇔', Rel),
        "implies" | "Longrightarrow" => ('⟹', Rel),
        "impliedby" | "Longleftarrow" => ('⟸', Rel),
        "longrightarrow" => ('⟶', Rel),
        "longleftarrow" => ('⟵', Rel),
        "longleftrightarrow" => ('⟷', Rel),
        "mapsto" => ('↦', Rel),
        "longmapsto" => ('⟼', Rel),
        "hookrightarrow" => ('↪', Rel),
        "rightleftharpoons" => ('⇌', Rel),
        "uparrow" => ('↑', Rel),
        "downarrow" => ('↓', Rel),
        "updownarrow" => ('↕', Rel),
        "nearrow" => ('↗', Rel),
        "searrow" => ('↘', Rel),
        "perp" => ('⊥', Rel),
        "parallel" => ('∥', Rel),
        "mid" => ('∣', Rel),
        "nmid" => ('∤', Rel),
        "vdash" => ('⊢', Rel),
        "dashv" => ('⊣', Rel),
        "models" => ('⊨', Rel),
        "prec" => ('≺', Rel),
        "succ" => ('≻', Rel),
        "preceq" => ('⪯', Rel),
        "succeq" => ('⪰', Rel),
        "doteq" => ('≐', Rel),
        "asymp" => ('≍', Rel),
        "bowtie" => ('⋈', Rel),
        "triangleq" => ('≜', Rel),
        "coloneqq" => ('≔', Rel),
        "lessgtr" => ('≶', Rel),
        "lesssim" => ('≲', Rel),
        "gtrsim" => ('≳', Rel),
        "Vdash" => ('⊩', Rel),
        "vDash" => ('⊨', Rel),
        "Vvdash" => ('⊪', Rel),
        "rightrightarrows" => ('⇉', Rel),
        "leftleftarrows" => ('⇇', Rel),
        "Longleftrightarrow" => ('⟺', Rel),
        "gtrdot" => ('⋗', Rel),
        "lessdot" => ('⋖', Rel),
        "therefore" => ('∴', Rel),
        "because" => ('∵', Rel),
        "upharpoonright" => ('↾', Rel),
        "upharpoonleft" => ('↿', Rel),
        "rightharpoonup" => ('⇀', Rel),
        "rightharpoondown" => ('⇁', Rel),
        "leftharpoonup" => ('↼', Rel),
        "leftharpoondown" => ('↽', Rel),
        "sqsubseteq" => ('⊑', Rel),
        "sqsupseteq" => ('⊒', Rel),
        "sqsubset" => ('⊏', Rel),
        "sqsupset" => ('⊐', Rel),
        "twoheadrightarrow" => ('↠', Rel),
        "twoheadleftarrow" => ('↞', Rel),
        "rightsquigarrow" => ('⇝', Rel),
        "leadsto" => ('⇝', Rel),
        "Uparrow" => ('⇑', Rel),
        "Downarrow" => ('⇓', Rel),
        "Updownarrow" => ('⇕', Rel),
        "nleftrightarrow" => ('↮', Rel),
        "nrightarrow" => ('↛', Rel),
        "nleftarrow" => ('↚', Rel),
        "nRightarrow" => ('⇏', Rel),
        "nLeftrightarrow" => ('⇎', Rel),
        "nsubseteq" => ('⊈', Rel),
        "nsupseteq" => ('⊉', Rel),
        "ncong" => ('≇', Rel),
        "nsim" => ('≁', Rel),
        "nleq" => ('≰', Rel),
        "ngeq" => ('≱', Rel),
        "nless" => ('≮', Rel),
        "ngtr" => ('≯', Rel),
        "trianglelefteq" => ('⊴', Rel),
        "trianglerighteq" => ('⊵', Rel),
        "vartriangleleft" => ('⊲', Rel),
        "vartriangleright" => ('⊳', Rel),
        "multimap" => ('⊸', Rel),
        "curvearrowright" => ('↷', Rel),
        "circlearrowleft" => ('↺', Rel),
        "circlearrowright" => ('↻', Rel),
        // ordinarios
        "infty" => ('∞', Ord),
        "partial" => ('𝜕', Ord),
        "nabla" => ('∇', Ord),
        "forall" => ('∀', Ord),
        "exists" => ('∃', Ord),
        "nexists" => ('∄', Ord),
        "emptyset" | "varnothing" => ('∅', Ord),
        "neg" | "lnot" => ('¬', Ord),
        "hbar" | "hslash" => ('ℏ', Ord),
        "ell" => ('ℓ', Ord),
        "Re" => ('ℜ', Ord),
        "Im" => ('ℑ', Ord),
        "aleph" => ('ℵ', Ord),
        "beth" => ('ℶ', Ord),
        "wp" => ('℘', Ord),
        "prime" => ('′', Ord),
        "angle" => ('∠', Ord),
        "triangle" => ('△', Ord),
        "top" => ('⊤', Ord),
        "bot" => ('⊥', Ord),
        "ldots" | "dots" | "dotsc" | "dotso" => ('…', Inner),
        "cdots" | "dotsb" | "dotsm" => ('⋯', Inner),
        "vdots" => ('⋮', Ord),
        "ddots" => ('⋱', Inner),
        "degree" => ('°', Ord),
        "checkmark" => ('✓', Ord),
        "square" | "Box" => ('□', Ord),
        "blacksquare" => ('■', Ord),
        "diamond" => ('⋄', Bin),
        "imath" => ('𝚤', Ord),
        "jmath" => ('𝚥', Ord),
        "backslash" => ('∖', Ord),
        "surd" => ('√', Ord),
        "flat" => ('♭', Ord),
        "sharp" => ('♯', Ord),
        "natural" => ('♮', Ord),
        // símbolos de modo texto que aparecen en metadatos de editoriales
        "texttimes" => ('×', Bin),
        "textpm" => ('±', Bin),
        "textdegree" => ('°', Ord),
        "textdagger" => ('†', Ord),
        "textdaggerdbl" => ('‡', Ord),
        "textperiodcentered" => ('·', Ord),
        "textbullet" => ('•', Ord),
        "textendash" => ('–', Ord),
        "textemdash" => ('—', Ord),
        "textasciitilde" => ('~', Ord),
        "textbackslash" => ('\\', Ord),
        "textless" => ('<', Rel),
        "textgreater" => ('>', Rel),
        "textbar" => ('|', Ord),
        "textmu" => ('μ', Ord),
        "textohm" => ('Ω', Ord),
        "textregistered" => ('®', Ord),
        "textcopyright" => ('©', Ord),
        "texttrademark" => ('™', Ord),
        "textellipsis" => ('…', Ord),
        "curvearrowleft" => ('↶', Rel),
        "leqq" => ('≦', Rel),
        "geqq" => ('≧', Rel),
        "leftrightharpoons" => ('⇋', Rel),
        "smallfrown" => ('⌢', Rel),
        "smallsmile" => ('⌣', Rel),
        "downharpoonleft" => ('⇃', Rel),
        "downharpoonright" => ('⇂', Rel),
        "subsetneqq" => ('⫋', Rel),
        "supsetneqq" => ('⫌', Rel),
        "ngeqslant" => ('≱', Rel),
        "nleqslant" => ('≰', Rel),
        "npreceq" => ('⋠', Rel),
        "nsucceq" => ('⋡', Rel),
        "gtrless" => ('≷', Rel),
        "textvisiblespace" => ('␣', Ord),
        "succcurlyeq" => ('≽', Rel),
        "preccurlyeq" => ('≼', Rel),
        "precsim" => ('≾', Rel),
        "succsim" => ('≿', Rel),
        "thicksim" => ('∼', Rel),
        "thickapprox" => ('≈', Rel),
        "backepsilon" => ('϶', Rel),
        "approxeq" => ('≊', Rel),
        "smile" => ('⌣', Rel),
        "frown" => ('⌢', Rel),
        "backsim" => ('∽', Rel),
        "backsimeq" => ('⋍', Rel),
        "shortparallel" => ('∥', Rel),
        "shortmid" => ('∣', Rel),
        "vartriangle" => ('△', Rel),
        "nprec" => ('⊀', Rel),
        "nsucc" => ('⊁', Rel),
        "Rrightarrow" => ('⇛', Rel),
        "Lleftarrow" => ('⇚', Rel),
        "varpropto" => ('∝', Rel),
        "gtrapprox" => ('⪆', Rel),
        "lessapprox" => ('⪅', Rel),
        "curlyeqprec" => ('⋞', Rel),
        "curlyeqsucc" => ('⋟', Rel),
        "eqslantless" => ('⪕', Rel),
        "eqslantgtr" => ('⪖', Rel),
        "lesseqgtr" => ('⋚', Rel),
        "gtreqless" => ('⋛', Rel),
        "doteqdot" => ('≑', Rel),
        "Doteq" => ('≑', Rel),
        "risingdotseq" => ('≓', Rel),
        "fallingdotseq" => ('≒', Rel),
        "eqcirc" => ('≖', Rel),
        "circeq" => ('≗', Rel),
        "bumpeq" => ('≏', Rel),
        "Bumpeq" => ('≎', Rel),
        "between" => ('≬', Rel),
        "pitchfork" => ('⋔', Rel),
        "nparallel" => ('∦', Rel),
        "nvdash" => ('⊬', Rel),
        "nvDash" => ('⊭', Rel),
        "nVdash" => ('⊮', Rel),
        "nVDash" => ('⊯', Rel),
        "ntriangleleft" => ('⋪', Rel),
        "ntriangleright" => ('⋫', Rel),
        "lneq" => ('⪇', Rel),
        "gneq" => ('⪈', Rel),
        "lneqq" => ('≨', Rel),
        "gneqq" => ('≩', Rel),
        "lnsim" => ('⋦', Rel),
        "gnsim" => ('⋧', Rel),
        "precnsim" => ('⋨', Rel),
        "succnsim" => ('⋩', Rel),
        "Subset" => ('⋐', Rel),
        "Supset" => ('⋑', Rel),
        "eqsim" => ('≂', Rel),
        "leftrightarrows" => ('⇆', Rel),
        "rightleftarrows" => ('⇄', Rel),
        "upuparrows" => ('⇈', Rel),
        "downdownarrows" => ('⇊', Rel),
        "Lsh" => ('↰', Rel),
        "Rsh" => ('↱', Rel),
        "looparrowleft" => ('↫', Rel),
        "looparrowright" => ('↬', Rel),
        "leftarrowtail" => ('↢', Rel),
        "rightarrowtail" => ('↣', Rel),
        "hookleftarrow" => ('↩', Rel),
        "swarrow" => ('↙', Rel),
        "nwarrow" => ('↖', Rel),
        "dashrightarrow" => ('⇢', Rel),
        "dashleftarrow" => ('⇠', Rel),
        "leftrightsquigarrow" => ('↭', Rel),
        "barwedge" => ('⊼', Bin),
        "veebar" => ('⊻', Bin),
        "bigtriangledown" => ('▽', Bin),
        "bigtriangleup" => ('△', Bin),
        "bigcirc" => ('◯', Bin),
        "And" => ('&', Bin),
        "curlywedge" => ('⋏', Bin),
        "curlyvee" => ('⋎', Bin),
        "doublebarwedge" => ('⩞', Bin),
        "circledast" => ('⊛', Bin),
        "circledcirc" => ('⊚', Bin),
        "circleddash" => ('⊝', Bin),
        "leftthreetimes" => ('⋋', Bin),
        "rightthreetimes" => ('⋌', Bin),
        "Cap" => ('⋒', Bin),
        "Cup" => ('⋓', Bin),
        "doublecap" => ('⋒', Bin),
        "doublecup" => ('⋓', Bin),
        "triangledown" => ('▿', Ord),
        "diagdown" => ('╲', Ord),
        "diagup" => ('╱', Ord),
        "stigma" => ('ϛ', Ord),
        "Stigma" => ('Ϛ', Ord),
        "AA" => ('Å', Ord),
        "aa" => ('å', Ord),
        "measuredangle" => ('∡', Ord),
        "sphericalangle" => ('∢', Ord),
        "gimel" => ('ℷ', Ord),
        "daleth" => ('ℸ', Ord),
        "varGamma" => ('𝛤', Ord),
        "varDelta" => ('𝛥', Ord),
        "varTheta" => ('𝛩', Ord),
        "varLambda" => ('𝛬', Ord),
        "varXi" => ('𝛯', Ord),
        "varPi" => ('𝛱', Ord),
        "varSigma" => ('𝛴', Ord),
        "varUpsilon" => ('𝛶', Ord),
        "varPhi" => ('𝛷', Ord),
        "varPsi" => ('𝛹', Ord),
        "varOmega" => ('𝛺', Ord),
        "blacktriangle" => ('▴', Ord),
        "blacktriangledown" => ('▾', Ord),
        "blacktriangleleft" => ('◂', Ord),
        "blacktriangleright" => ('▸', Ord),
        "circledR" => ('®', Ord),
        "circledS" => ('Ⓢ', Ord),
        "backprime" => ('‵', Ord),
        "Bbbk" => ('𝕜', Ord),
        "koppa" => ('ϟ', Ord),
        "Koppa" => ('Ϟ', Ord),
        "sampi" => ('ϡ', Ord),
        "Sampi" => ('Ϡ', Ord),
        "Digamma" => ('Ϝ', Ord),
        "ae" => ('æ', Ord),
        "AE" => ('Æ', Ord),
        "oe" => ('œ', Ord),
        "OE" => ('Œ', Ord),
        "o" => ('ø', Ord),
        "O" => ('Ø', Ord),
        "ss" => ('ß', Ord),
        "l" => ('ł', Ord),
        "L" => ('Ł', Ord),
        "i" => ('ı', Ord),
        "j" => ('ȷ', Ord),
        "Diamond" => ('◇', Ord),
        "complement" => ('∁', Ord),
        "mho" => ('℧', Ord),
        "eth" => ('ð', Ord),
        "Finv" => ('Ⅎ', Ord),
        "Game" => ('⅁', Ord),
        "digamma" => ('ϝ', Ord),
        "lozenge" => ('◊', Ord),
        "blacklozenge" => ('⧫', Ord),
        "bigstar" => ('★', Ord),
        "maltese" => ('✠', Ord),
        "S" => ('§', Ord),
        "P" => ('¶', Ord),
        "pounds" => ('£', Ord),
        "yen" => ('¥', Ord),
        "euro" => ('€', Ord),
        "copyright" => ('©', Ord),
        "dag" => ('†', Ord),
        "ddag" => ('‡', Ord),
        "ulcorner" => ('⌜', Open),
        "urcorner" => ('⌝', Close),
        "llcorner" => ('⌞', Open),
        "lrcorner" => ('⌟', Close),
        "lbrack" => ('[', Open),
        "rbrack" => (']', Close),
        "lgroup" => ('⟮', Open),
        "rgroup" => ('⟯', Close),
        "clubsuit" => ('♣', Ord),
        "spadesuit" => ('♠', Ord),
        "heartsuit" => ('♡', Ord),
        "diamondsuit" => ('♢', Ord),
        // delimitadores usados sueltos
        "lbrace" | "{" => ('{', Open),
        "rbrace" | "}" => ('}', Close),
        "langle" => ('⟨', Open),
        "rangle" => ('⟩', Close),
        "lfloor" => ('⌊', Open),
        "rfloor" => ('⌋', Close),
        "lceil" => ('⌈', Open),
        "rceil" => ('⌉', Close),
        "lvert" => ('|', Open),
        "rvert" => ('|', Close),
        "lVert" => ('‖', Open),
        "rVert" => ('‖', Close),
        "vert" => ('|', Ord),
        "Vert" | "|" => ('‖', Ord),
        // caracteres escapados
        "#" => ('#', Ord),
        "$" => ('$', Ord),
        "%" => ('%', Ord),
        "&" => ('&', Ord),
        "_" => ('_', Ord),
        "colon" => (':', Punct),
        _ => return None,
    })
}

/// Operadores grandes: (carácter, límites encima/debajo en display).
fn gran_operador(nombre: &str) -> Option<(char, bool)> {
    Some(match nombre {
        "sum" => ('∑', true),
        "prod" => ('∏', true),
        "coprod" => ('∐', true),
        "bigcup" => ('⋃', true),
        "bigcap" => ('⋂', true),
        "bigvee" => ('⋁', true),
        "bigwedge" => ('⋀', true),
        "bigoplus" => ('⨁', true),
        "bigotimes" => ('⨂', true),
        "bigodot" => ('⨀', true),
        "biguplus" => ('⨄', true),
        "bigsqcup" => ('⨆', true),
        "int" => ('∫', false),
        "iint" => ('∬', false),
        "iiint" => ('∭', false),
        "iiiint" => ('⨌', false),
        "oint" => ('∮', false),
        "oiint" => ('∯', false),
        _ => return None,
    })
}

/// Funciones con nombre (se escriben rectas): (nombre, límites en display).
fn funcion(nombre: &str) -> Option<bool> {
    Some(match nombre {
        "lim" | "liminf" | "limsup" | "sup" | "inf" | "min" | "max" | "det" | "gcd" | "Pr" | "argmax" | "argmin" => {
            true
        }
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "arcsin" | "arccos" | "arctan" | "sinh" | "cosh" | "tanh"
        | "coth" | "log" | "ln" | "lg" | "exp" | "dim" | "ker" | "hom" | "arg" | "deg" | "lcm" | "mod" | "bmod"
        | "sgn" | "tr" | "rank" | "span" | "erf" => false,
        _ => return None,
    })
}

/// Acentos: comando → carácter combinante del acento.
fn acento(nombre: &str) -> Option<char> {
    Some(match nombre {
        "hat" | "widehat" => '\u{0302}',
        "check" | "widecheck" => '\u{030C}',
        "tilde" | "widetilde" => '\u{0303}',
        "acute" => '\u{0301}',
        "grave" => '\u{0300}',
        "dot" => '\u{0307}',
        "ddot" => '\u{0308}',
        "breve" => '\u{0306}',
        "bar" => '\u{0304}',
        "vec" | "overrightarrow" => '\u{20D7}',
        "overleftarrow" => '\u{20D6}',
        "mathring" => '\u{030A}',
        _ => return None,
    })
}

/// Delimitador tras \left, \right o \big…: None = «.» (sin delimitador).
fn delimitador(token: &str) -> Option<Option<char>> {
    Some(match token {
        "." => None,
        "(" | ")" | "[" | "]" | "|" | "/" => token.chars().next(),
        "\\{" | "\\lbrace" => Some('{'),
        "\\}" | "\\rbrace" => Some('}'),
        "\\langle" | "<" => Some('⟨'),
        "\\rangle" | ">" => Some('⟩'),
        "\\lfloor" => Some('⌊'),
        "\\rfloor" => Some('⌋'),
        "\\lceil" => Some('⌈'),
        "\\rceil" => Some('⌉'),
        "\\lbrack" => Some('['),
        "\\rbrack" => Some(']'),
        "\\Uparrow" => Some('⇑'),
        "\\Downarrow" => Some('⇓'),
        "\\Updownarrow" => Some('⇕'),
        "\\updownarrow" => Some('↕'),
        "\\ulcorner" => Some('⌜'),
        "\\urcorner" => Some('⌝'),
        "\\llcorner" => Some('⌞'),
        "\\lrcorner" => Some('⌟'),
        "\\lgroup" => Some('⟮'),
        "\\rgroup" => Some('⟯'),
        "\\|" | "\\Vert" | "\\lVert" | "\\rVert" => Some('‖'),
        "\\vert" | "\\lvert" | "\\rvert" => Some('|'),
        "\\uparrow" => Some('↑'),
        "\\downarrow" => Some('↓'),
        "\\backslash" => Some('\\'),
        _ => return None,
    })
}
