//! siunitx unit symbols and SI prefixes.

/// siunitx units: printed symbol.
pub fn unit(name: &str) -> Option<&'static str> {
    Some(match name {
        "meter" | "metre" => "m",
        "second" => "s",
        "gram" => "g",
        "kilogram" => "kg",
        "ampere" => "A",
        "kelvin" => "K",
        "mole" => "mol",
        "candela" => "cd",
        "newton" => "N",
        "joule" => "J",
        "watt" => "W",
        "pascal" => "Pa",
        "hertz" => "Hz",
        "coulomb" => "C",
        "volt" => "V",
        "ohm" => "Ω",
        "farad" => "F",
        "tesla" => "T",
        "weber" => "Wb",
        "henry" => "H",
        "siemens" => "S",
        "liter" | "litre" => "L",
        "hour" => "h",
        "minute" => "min",
        "day" => "d",
        "degreeCelsius" | "celsius" => "°C",
        "degree" => "°",
        "electronvolt" => "eV",
        "calorie" => "cal",
        "bar" => "bar",
        "atm" | "atmosphere" => "atm",
        "percent" => "%",
        "mmHg" => "mmHg",
        "hectare" => "ha",
        "tonne" => "t",
        "angstrom" => "Å",
        "radian" => "rad",
        "steradian" => "sr",
        "lumen" => "lm",
        "lux" => "lx",
        "becquerel" => "Bq",
        "gray" => "Gy",
        "sievert" => "Sv",
        "katal" => "kat",
        "dalton" => "Da",
        "atomicmassunit" => "u",
        "astronomicalunit" => "au",
        "arcminute" => "′",
        "arcsecond" => "″",
        "bel" => "B",
        "neper" => "Np",
        "barn" => "b",
        "knot" => "kn",
        "nauticalmile" => "M",
        "bit" => "bit",
        "byte" => "B",
        "molar" => "M",
        "torr" => "Torr",
        _ => return None,
    })
}

/// The name a unit is spoken by, for the spellings siunitx accepts twice.
pub fn canonical_unit(name: &str) -> &str {
    match name {
        "metre" => "meter",
        "litre" => "liter",
        "celsius" => "degreeCelsius",
        "atmosphere" => "atm",
        other => other,
    }
}

pub fn canonical_prefix(name: &str) -> &str {
    match name {
        "deka" => "deca",
        other => other,
    }
}

pub fn prefix(name: &str) -> Option<&'static str> {
    Some(match name {
        "yotta" => "Y",
        "zetta" => "Z",
        "exa" => "E",
        "peta" => "P",
        "tera" => "T",
        "giga" => "G",
        "mega" => "M",
        "kilo" => "k",
        "hecto" => "h",
        "deca" | "deka" => "da",
        "deci" => "d",
        "centi" => "c",
        "milli" => "m",
        "micro" => "µ",
        "nano" => "n",
        "pico" => "p",
        "femto" => "f",
        "atto" => "a",
        "zepto" => "z",
        "yocto" => "y",
        _ => return None,
    })
}
