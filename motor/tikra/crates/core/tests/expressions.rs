//! pgfmath expressions through the public API: functions, constants, units and errors.

use tikra::expr::{eval, eval_length, eval_string, set_seed, unit_cm, Variables};

fn close(text: &str, expected: f64) {
    let mut v = Variables::new();
    v.insert("x".into(), 2.0);
    let r = eval(text, &v).unwrap_or_else(|e| panic!("{text}: {e}"));
    assert!((r - expected).abs() < 1e-9, "{text} = {r}, expected {expected}");
}

#[test]
fn functions_constants_and_units() {
    for (t, e) in [
        ("cos(60)", 0.5),
        ("tan(45)", 1.0),
        ("asin(1)", 90.0),
        ("acos(0)", 90.0),
        ("atan(1)", 45.0),
        ("atan2(1,1)", 45.0),
        ("atan2(1)", 0.0),
        ("sqrt(9)", 3.0),
        ("abs(-2)", 2.0),
        ("exp(0)", 1.0),
        ("ln(e)", 1.0),
        ("log10(100)", 2.0),
        ("floor(1.7)", 1.0),
        ("ceil(1.2)", 2.0),
        ("round(1.5)", 2.0),
        ("deg(pi)", 180.0),
        ("rad(180)", std::f64::consts::PI),
        ("min(3, 1, 2)", 1.0),
        ("max(3, 1, 2)", 3.0),
        ("pow(2, 10)", 1024.0),
        ("pow(5)", 5.0),
        ("mod(7, 3)", 1.0),
        ("mod(7)", 0.0),
        // as TeX computes them: never negative
        ("Mod(-7, 3)", 2.0),
        ("mod(-7, 3)", -1.0),
        ("Mod(7, -3)", 1.0),
        ("Mod(-7.5, 2)", 0.5),
        ("veclen(3, 4)", 5.0),
        ("veclen(3)", 3.0),
        ("\\pi", std::f64::consts::PI),
        ("+2", 2.0),
        ("{1+1}*2", 4.0),
        ("x^2", 4.0),
        ("2cm", 2.0),
        ("2e3", 2000.0),
        ("2.5E-1", 0.25),
        ("1e+2", 100.0),
        ("2^3^2", 512.0),
    ] {
        close(t, e);
    }
}

#[test]
fn expression_errors() {
    let v = Variables::new();
    for t in ["", "1 2", "(1", "{1", "max(1, 2", "foo(1)", "@", "\\nothing", "doesnotexist", "1.2.3", "2e", "2cmx", "3 +"] {
        assert!(eval(t, &v).is_err(), "{t}");
    }
    // a unit suffix with an invalid rest: falls back to evaluating without a unit and fails there
    assert!(eval_length("xcm", &v).is_err());
    assert_eq!(eval_length("2", &v).unwrap(), (2.0, false));
    assert_eq!(unit_cm("km"), None);
    for u in ["bp", "in", "em", "ex", "pc", "mm"] {
        assert!(eval_length(&format!("1{u}"), &v).unwrap().1, "{u}");
    }
}

#[test]
fn errors_inside_subexpressions() {
    let v = Variables::new();
    for t in ["(1+)", "2*", "2/", "2^", "-", "max(1,)", "sin(1+)", "{1*}"] {
        assert!(eval(t, &v).is_err(), "{t}");
    }
}

#[test]
fn functions_without_parentheses() {
    // as pgfmath reads them: the operand after the name, its sign included, and nothing more
    for (t, e) in [
        ("sin 30", 0.5),
        ("cos 60 + 1", 1.5),
        ("sin 30^2", 0.25),
        ("sin -30", -0.5),
        ("-sin 30", -0.5),
        ("sqrt 4+5", 7.0),
        ("abs -3", 3.0),
        ("sin(30)*cos 60", 0.25),
        ("2*sin 30", 1.0),
        ("sin cos 60", 0.00873),
    ] {
        let r = eval(t, &Variables::new()).unwrap_or_else(|e| panic!("{t}: {e}"));
        assert!((r - e).abs() < 1e-4, "{t} = {r}, expected {e}");
    }
    // sin 2*30 is sin(2) times 30
    assert!((eval("sin 2*30", &Variables::new()).unwrap() - 1.0469).abs() < 1e-3);
    assert!(eval("foo 30", &Variables::new()).is_err());
}

#[test]
fn lengths_count_in_pt() {
    // pgfmath's 1/10pt is 0.1 (pt), and 3pt*2cm is 3 times 56.9 pt
    let v = Variables::new();
    let pt = tikra::PT;
    for (t, e) in
        [("1/10pt", 0.1 * pt), ("1/10.0pt", 0.1 * pt), ("3pt*2cm", 170.71654 * pt), ("1cm/2", 0.5), ("10pt/2", 5.0 * pt), ("2*3pt", 6.0 * pt), ("1cm+2pt", 1.0 + 2.0 * pt)]
    {
        let (r, unit) = eval_length(t, &v).unwrap();
        assert!(unit && (r - e).abs() < 1e-6, "{t} = {r}, expected {e}");
    }
    // exactly, without the rounding of the way through pt
    assert_eq!(eval_length("14cm", &v).unwrap(), (14.0, true));
    // and nothing else moves: lengths in pt, small ones included, are what they were
    for x in [120.0, 0.00003, 1.0 / 3.0] {
        assert_eq!(eval_length(&format!("{x}pt"), &v).unwrap().0, x * pt, "{x}pt");
    }
    // an element of an array and a declared function keep the count in pt
    assert!((eval_length("1/{10pt, 1}[0]", &v).unwrap().0 - 0.1 * pt).abs() < 1e-9);
    let mut f = Variables::new();
    f.functions.insert("w".into(), (Vec::new(), "10pt".into()));
    assert!((eval_length("1/w", &f).unwrap().0 - 0.1 * pt).abs() < 1e-9);
}

#[test]
fn arrays_of_strings() {
    let v = Variables::new();
    assert_eq!(eval_string(r#"{"orange","blue","red"}[1]"#, &v).unwrap().as_deref(), Some("blue"));
    assert_eq!(eval_string(r#"{"a",{"b","c"}}[1][1]"#, &v).unwrap().as_deref(), Some("c"));
    // a number in it is evaluated as one
    assert_eq!(eval_string(r#"{"a",2}[1]"#, &v).unwrap(), None);
    assert_eq!(eval(r#"{"a",2}[1]"#, &v).unwrap(), 2.0);
    // an array followed by more is not one string
    assert!(eval_string(r#"{"a","b"}[1] + 1"#, &v).is_err());
}

#[test]
fn random_integers_from_the_seed() {
    // what TeX gives after \pgfmathsetseed{1}
    let v = Variables::new();
    set_seed(1);
    assert_eq!([eval("random(10)", &v).unwrap(), eval("random(10)", &v).unwrap(), eval("random(10)", &v).unwrap()], [2.0, 8.0, 5.0]);
    set_seed(1);
    assert_eq!([eval("random(3,7)", &v).unwrap(), eval("random(7,3)", &v).unwrap()], [4.0, 5.0]);
    set_seed(1);
    assert_eq!(eval("random()", &v).unwrap(), 0.69621);
    assert!(eval("random(", &v).is_err());
}

#[test]
fn an_array_of_strings_is_indexed_from_zero() {
    let mut v = Variables::new();
    v.insert("x".into(), 2.0);
    let pick = |t: &str| eval_string(t, &v);
    assert_eq!(pick(r#"{"red","green","blue"}[1]"#).unwrap().as_deref(), Some("green"));
    // the index is an expression
    assert_eq!(pick(r#"{"red","green","blue"}[x-1]"#).unwrap().as_deref(), Some("green"));
    assert!(pick(r#"{"red","green"}[2]"#).is_err());
}
