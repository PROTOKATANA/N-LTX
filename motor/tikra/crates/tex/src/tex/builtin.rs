//! The primitive names, the macros and the registers TeX starts with.

use super::*;
use std::collections::HashMap;
use std::rc::Rc;

/// Registers reserved before `\newcount` hands out new ones (pgf uses low numbers as scratch).
const FIRST_FREE_REGISTER: u32 = 256;

/// The register of the first of [`LATEX_LENGTHS`] (the others follow it).
const FIRST_LATEX_LENGTH: u32 = 180;

/// LaTeX's lengths and their values in `article` at 10pt on letter paper, as TeX shows them;
/// `\paperwidth` and `\paperheight` first.
const LATEX_LENGTHS: &[(&str, &str)] = &[
    ("paperwidth", "614.295pt"),
    ("paperheight", "794.96999pt"),
    ("textheight", "550pt"),
    ("vsize", "550pt"),
    ("oddsidemargin", "62pt"),
    ("evensidemargin", "62pt"),
    ("topmargin", "16pt"),
    ("headheight", "12pt"),
    ("headsep", "25pt"),
    ("footskip", "30pt"),
    ("marginparwidth", "65pt"),
    ("marginparsep", "11pt"),
    ("marginparpush", "5pt"),
    ("hoffset", "0pt"),
    ("voffset", "0pt"),
    ("maxdepth", "5pt"),
    ("topskip", "10pt"),
    ("columnsep", "10pt"),
    ("columnseprule", "0pt"),
    ("parskip", "0pt"),
    ("tabcolsep", "6pt"),
    ("arraycolsep", "5pt"),
    ("arrayrulewidth", "0.4pt"),
    ("doublerulesep", "2pt"),
    ("footnotesep", "6.65pt"),
    ("jot", "3pt"),
    ("lineskip", "1pt"),
    ("lineskiplimit", "0pt"),
    ("emergencystretch", "0pt"),
    ("labelsep", "5pt"),
    ("labelwidth", "20.00003pt"),
    ("leftmargin", "25.00003pt"),
    ("rightmargin", "0pt"),
    ("itemindent", "0pt"),
    ("listparindent", "0pt"),
    ("itemsep", "4pt"),
    ("parsep", "4pt"),
    ("topsep", "8pt"),
    ("partopsep", "2pt"),
    ("smallskipamount", "3pt"),
    ("medskipamount", "6pt"),
    ("bigskipamount", "12pt"),
    ("floatsep", "12pt"),
    ("textfloatsep", "20pt"),
    ("intextsep", "12pt"),
    ("abovedisplayskip", "10pt"),
    ("belowdisplayskip", "10pt"),
    ("captionmargin", "0pt"),
];

/// Those of [`LATEX_LENGTHS`] that are glue (`\skip` registers).
const SKIPS: &[&str] = &[
    "topskip",
    "parskip",
    "itemsep",
    "parsep",
    "topsep",
    "partopsep",
    "smallskipamount",
    "medskipamount",
    "bigskipamount",
    "floatsep",
    "textfloatsep",
    "intextsep",
    "abovedisplayskip",
    "belowdisplayskip",
];

/// LaTeX's paper sizes: name, width, height.
const PAPERS: &[(&str, &str, &str)] = &[
    ("letterpaper", "614.295pt", "794.96999pt"),
    ("a4paper", "597.50787pt", "845.04684pt"),
    ("a5paper", "421.10078pt", "597.50787pt"),
    ("b5paper", "500.7685pt", "711.3189pt"),
    ("legalpaper", "614.295pt", "1011.78pt"),
    ("executivepaper", "523.95749pt", "758.83499pt"),
];

impl Tex {
    pub fn new() -> Tex {
        let mut t = Tex {
            input: Vec::new(),
            meanings: Map::default(),
            active: Map::default(),
            counts: Map::default(),
            dimens: Map::default(),
            toks: Map::default(),
            catcodes: Map::default(),
            save: Vec::new(),
            global: false,
            conds: 0,
            next_register: FIRST_FREE_REGISTER,
            expansions: 0,
            statement_expansions: 0,
            after_group: Vec::new(),
            after_assignment: None,
            warnings: Vec::new(),
            environments: HashMap::new(),
            env_stack: Vec::new(),
            named_boxes: Vec::new(),
            box_dims: Map::default(),
        };
        t.define_primitives();
        t
    }

    fn define_primitives(&mut self) {
        self.bind_primitive_names();
        self.define_builtin_macros();
        self.init_builtin_registers();
    }

    fn bind_primitive_names(&mut self) {
        use Prim::*;
        let list: &[(&str, Prim)] = &[
            ("expandafter", Expandafter),
            ("noexpand", Noexpand),
            ("csname", Csname),
            ("endcsname", Endcsname),
            ("string", Stringify),
            ("number", Number),
            ("romannumeral", Romannumeral),
            ("the", The),
            ("meaning", ShowMeaning),
            ("unexpanded", Unexpanded),
            ("detokenize", Detokenize),
            ("if", If),
            ("ifcat", Ifcat),
            ("ifx", Ifx),
            ("ifnum", Ifnum),
            ("ifdim", Ifdim),
            ("ifodd", Ifodd),
            ("ifcase", Ifcase),
            ("iftrue", Iftrue),
            ("iffalse", Iffalse),
            ("ifdefined", Ifdefined),
            ("ifcsname", Ifcsname),
            ("ifvmode", Ifvmode),
            ("ifhmode", Ifhmode),
            ("ifmmode", Ifmmode),
            ("ifinner", Ifinner),
            ("else", Else),
            ("or", Or),
            ("fi", Fi),
            ("unless", Unless),
            ("jobname", Jobname),
            ("pdfstrcmp", Pdfstrcmp),
            ("strcmp", Pdfstrcmp),
            ("relax", Relax),
            ("def", Def),
            ("gdef", Gdef),
            ("edef", Edef),
            ("xdef", Xdef),
            ("let", Let),
            ("futurelet", Futurelet),
            ("global", Global),
            ("long", Long),
            ("outer", Outer),
            ("protected", Protected),
            ("begingroup", Begingroup),
            ("endgroup", Endgroup),
            ("count", Count),
            ("dimen", Dimen),
            ("skip", Skip),
            ("toks", Toks),
            ("countdef", Countdef),
            ("dimendef", Dimendef),
            ("skipdef", Skipdef),
            ("toksdef", Toksdef),
            ("chardef", Chardef),
            ("mathchardef", Mathchardef),
            ("advance", Advance),
            ("multiply", Multiply),
            ("divide", Divide),
            ("catcode", Catcode),
            ("numexpr", Numexpr),
            ("dimexpr", Dimexpr),
            ("glueexpr", Glueexpr),
            ("afterassignment", Afterassignment),
            ("aftergroup", Aftergroup),
            ("ignorespaces", Ignorespaces),
            ("par", Par),
            ("message", Message),
            ("immediate", Immediate),
            ("write", Write),
            ("typeout", Typeout),
            ("errmessage", Errmessage),
            ("PackageWarning", Typeout),
            ("showtokens", Showtokens),
            ("newcount", Newcount),
            // a box register: a number, as `\setbox` reads it
            ("newbox", Newcount),
            ("newdimen", Newdimen),
            ("newskip", Newskip),
            ("newlength", Newlength),
            ("newtoks", Newtoks),
            ("newif", Newif),
            ("setlength", Setlength),
            ("addtolength", Addtolength),
            ("setcounter", Setcounter),
            ("addtocounter", Addtocounter),
            ("newcounter", Newcounter),
            ("stepcounter", Stepcounter),
            ("refstepcounter", Stepcounter),
            ("value", Value),
            ("arabic", Arabic),
            ("newcommand", Newcommand),
            ("renewcommand", Renewcommand),
            ("providecommand", Providecommand),
            ("DeclareRobustCommand", Declarerobustcommand),
            ("NewDocumentCommand", Newdocumentcommand),
            ("RenewDocumentCommand", Newdocumentcommand),
            ("ProvideDocumentCommand", Newdocumentcommand),
            ("DeclareDocumentCommand", Newdocumentcommand),
            ("NewExpandableDocumentCommand", Newdocumentcommand),
            ("RenewExpandableDocumentCommand", Newdocumentcommand),
            ("ProvideExpandableDocumentCommand", Newdocumentcommand),
            ("DeclareExpandableDocumentCommand", Newdocumentcommand),
            ("tikra@documentarguments", Documentarguments),
            ("IfNoValueTF", Documentif),
            ("IfNoValueT", Documentif),
            ("IfNoValueF", Documentif),
            ("IfValueTF", Documentif),
            ("IfValueT", Documentif),
            ("IfValueF", Documentif),
            ("IfBooleanTF", Documentif),
            ("IfBooleanT", Documentif),
            ("IfBooleanF", Documentif),
            ("newenvironment", Newenvironment),
            ("renewenvironment", Renewenvironment),
            ("makeatletter", Makeatletter),
            ("makeatother", Makeatother),
            ("@ifnextchar", Ifnextchar),
            ("pgfutil@ifnextchar", Ifnextchar),
            ("@firstoftwo", Firstoftwo),
            ("pgfutil@firstoftwo", Firstoftwo),
            ("@secondoftwo", Secondoftwo),
            ("pgfutil@secondoftwo", Secondoftwo),
            ("@gobble", Gobble),
            ("pgfutil@gobble", Gobble),
            ("@gobbletwo", Gobbletwo),
            ("@firstofone", Firstofone),
            ("pgfutil@firstofone", Firstofone),
            ("ifthenelse", Ifthenelse),
            ("whiledo", Whiledo),
            ("ifstrequal", Ifstrequal),
            ("ifstrempty", Ifstrempty),
            ("ifblank", Ifstrempty),
            ("ifdefempty", Ifstrempty),
            ("g@addto@macro", Addtomacro),
            ("addto", Addtomacro),
            ("appto", Addtomacro),
            ("gappto", Addtomacro),
            ("eappto", Addtomacro),
            ("xappto", Addtomacro),
            ("preto", Addtomacro),
            ("gpreto", Addtomacro),
            ("epreto", Addtomacro),
            ("xpreto", Addtomacro),
            ("boolean", Ifboolean),
            ("newboolean", Newboolean),
            ("setboolean", Setboolean),
            ("wd", Wd),
            ("ht", Ht),
            ("dp", Dp),
            ("setbox", Setbox),
            ("hbox", Hbox),
            ("uppercase", Uppercase),
            ("lowercase", Lowercase),
            ("show", Show),
            ("showthe", Showthe),
        ];
        for (name, p) in list {
            self.meanings.insert(Rc::from(*name), Meaning::Prim(*p));
        }
        for name in xstring::COMMANDS {
            self.meanings.insert(Rc::from(*name), Meaning::Prim(Xstring));
        }
        // the mark \noexpand and \unexpanded put before a token; on its own it does nothing
        self.meanings.insert(Rc::from("tikra@noexpand"), Meaning::Prim(Relax));
    }

    fn define_builtin_macros(&mut self) {
        // a few plain and LaTeX macros, defined the way they are there
        // the value of a missing optional argument of a document command, and of an `s` one
        self.define_text(r"\def\tikra@novalue{-NoValue-}\def\BooleanTrue{\BooleanTrue}\def\BooleanFalse{\BooleanFalse}");
        self.define_text(r"\def\space{ }\def\empty{}\def\@empty{}\def\pgfutil@empty{}\def\@nil{\@nil}\def\pgfutil@nil{\pgfutil@nil}\def\pgfeov{\pgfeov}");
        self.define_text(r"\def\@ifundefined#1{\ifcsname#1\endcsname\expandafter\ifx\csname#1\endcsname\relax\expandafter\expandafter\expandafter\@firstoftwo\else\expandafter\expandafter\expandafter\@secondoftwo\fi\else\expandafter\@firstoftwo\fi}");
        self.define_text(r"\let\pgfutil@ifundefined\@ifundefined\def\@namedef#1{\expandafter\def\csname#1\endcsname}\def\@nameuse#1{\csname#1\endcsname}");
        self.define_text(r"\def\loop#1\repeat{\def\body{#1}\iterate}\def\iterate{\body\let\next\iterate\else\let\next\relax\fi\next}\let\repeat\fi");
        // pgfmath's loop: as \loop, with \pgfmathcounter counting the rounds from 1
        self.define_text(
            r"\def\pgfmathloop#1\repeatpgfmathloop{\def\pgfmathcounter{1}\def\tikra@mathloop@body{#1}\tikra@mathloop}\let\repeatpgfmathloop\fi
              \def\tikra@mathloop{\tikra@mathloop@body\edef\pgfmathcounter{\the\numexpr\pgfmathcounter+1\relax}\let\next\tikra@mathloop\else\let\next\relax\fi\next}",
        );
        self.define_text(
            r"\def\obeyspaces{}\def\protect{}\def\nobreak{}\def\noindent{}\def\indent{}\def\leavevmode{}\def\unskip{}\def\strut{}\def\quad{\ }\def\qquad{\ \ }\def\enspace{\ }",
        );
        // the preview package's setup, as in `\PreviewEnvironment{tikzpicture}\setlength\PreviewBorder{5pt}`:
        // the figure is already all that is drawn
        self.define_text(r"\def\PreviewEnvironment#1{}\def\PreviewMacro#1{}");
        // amsmath's `\DeclareMathOperator{\p}{p}`: `\p` is the operator's name in upright letters
        // (starred, with limits above and below)
        self.define_text(
            r"\def\DeclareMathOperator{\@ifnextchar*{\tikra@operator@star}{\tikra@operator}}\def\tikra@operator#1#2{\def#1{\operatorname{#2}}}\def\tikra@operator@star*#1#2{\def#1{\operatorname*{#2}}}",
        );
        // etoolbox's toggles: each one a macro that takes the first or the second of two branches
        self.define_text(
            r"\def\newtoggle#1{\expandafter\let\csname tikra@toggle@#1\endcsname\@secondoftwo}\def\providetoggle#1{\ifcsname tikra@toggle@#1\endcsname\else\newtoggle{#1}\fi}\def\toggletrue#1{\expandafter\let\csname tikra@toggle@#1\endcsname\@firstoftwo}\let\togglefalse\newtoggle\def\settoggle#1#2{\csname toggle#2\endcsname{#1}}\def\iftoggle#1{\csname tikra@toggle@#1\endcsname}\def\nottoggle#1#2#3{\iftoggle{#1}{#3}{#2}}",
        );
    }

    fn init_builtin_registers(&mut self) {
        for i in 0..10u32 {
            self.dimens.insert(i, 0);
            self.counts.insert(i, 0);
        }
        self.meanings.insert(Rc::from("z@"), Meaning::Dimen(250));
        self.meanings.insert(Rc::from("p@"), Meaning::Dimen(251));
        self.dimens.insert(251, 65536);
        self.meanings.insert(Rc::from("pgfutil@tempdima"), Meaning::Dimen(252));
        self.meanings.insert(Rc::from("pgfutil@tempdimb"), Meaning::Dimen(253));
        self.meanings.insert(Rc::from("pgfutil@tempcnta"), Meaning::Count(252));
        self.meanings.insert(Rc::from("pgfutil@tempcntb"), Meaning::Count(253));
        // pgf's scratch counters, which code of a figure's own uses too
        for (i, c) in ["a", "b", "c", "d"].iter().enumerate() {
            self.meanings.insert(Rc::from(format!("c@pgf@count{c}")), Meaning::Count(244 + i as u32));
        }
        self.meanings.insert(Rc::from("m@ne"), Meaning::CharDef(-1));
        self.meanings.insert(Rc::from("z@"), Meaning::Dimen(250));
        self.meanings.insert(Rc::from("@ne"), Meaning::CharDef(1));
        self.meanings.insert(Rc::from("tw@"), Meaning::CharDef(2));
        self.meanings.insert(Rc::from("thr@@"), Meaning::CharDef(3));
        self.meanings.insert(Rc::from("@m"), Meaning::CharDef(1000));
        self.meanings.insert(Rc::from("@M"), Meaning::CharDef(10000));
        self.meanings.insert(Rc::from("active"), Meaning::CharDef(13));
        self.meanings.insert(Rc::from("textwidth"), Meaning::Dimen(254));
        self.dimens.insert(254, 345 * 65536);
        self.meanings.insert(Rc::from("linewidth"), Meaning::Dimen(255));
        self.dimens.insert(255, 345 * 65536);
        self.meanings.insert(Rc::from("columnwidth"), Meaning::Dimen(255));
        self.meanings.insert(Rc::from("hsize"), Meaning::Dimen(255));
        self.meanings.insert(Rc::from("baselineskip"), Meaning::Dimen(249));
        self.dimens.insert(249, 12 * 65536);
        self.meanings.insert(Rc::from("parindent"), Meaning::Dimen(248));
        self.meanings.insert(Rc::from("fboxsep"), Meaning::Dimen(247));
        self.dimens.insert(247, 3 * 65536);
        self.meanings.insert(Rc::from("fboxrule"), Meaning::Dimen(246));
        self.dimens.insert(246, 26214);
        self.meanings.insert(Rc::from("unitlength"), Meaning::Dimen(245));
        self.dimens.insert(245, 65536);
        self.meanings.insert(Rc::from("PreviewBorder"), Meaning::Dimen(244));
        // LaTeX's page layout and list lengths, as `article` sets them at 10pt on letter paper (glue
        // at its natural size), and the caption package's \captionmargin
        for (i, (name, value)) in LATEX_LENGTHS.iter().enumerate() {
            let r = FIRST_LATEX_LENGTH + i as u32;
            let skip = SKIPS.contains(name);
            self.meanings.insert(Rc::from(*name), if skip { Meaning::Skip(r) } else { Meaning::Dimen(r) });
            let v = self.dimen_of(string_tokens(value)).expect("a length");
            self.dimens.insert(r, v);
        }
    }

    /// `\paperwidth` and `\paperheight` from the paper options of `\documentclass` or the geometry
    /// package (`a4paper`, `paper=a5paper`, `landscape`); none of them leaves them as they are.
    pub fn paper_options(&mut self, options: &str) {
        let mut landscape = false;
        let mut size = None;
        for o in options.split(',').map(str::trim) {
            let o = o.strip_prefix("paper").and_then(|p| p.trim_start().strip_prefix('=')).map_or(o, str::trim);
            match PAPERS.iter().find(|(n, ..)| *n == o) {
                Some(&(_, w, h)) => size = Some((w, h)),
                None => landscape |= o == "landscape",
            }
        }
        if size.is_none() && !landscape {
            return;
        }
        let (w, h) = match size {
            Some((w, h)) => (self.dimen_of(string_tokens(w)).expect("a length"), self.dimen_of(string_tokens(h)).expect("a length")),
            None => (self.dimens[&FIRST_LATEX_LENGTH], self.dimens[&(FIRST_LATEX_LENGTH + 1)]),
        };
        let (w, h) = if landscape { (w.max(h), w.min(h)) } else { (w, h) };
        self.set_dimen(FIRST_LATEX_LENGTH, w, true);
        self.set_dimen(FIRST_LATEX_LENGTH + 1, h, true);
    }

    /// A Tex that only reads tokens, with `@` a letter (for text written by tikra itself).
    pub fn new_lexer() -> Tex {
        let mut t = Tex {
            input: Vec::new(),
            meanings: Map::default(),
            active: Map::default(),
            counts: Map::default(),
            dimens: Map::default(),
            toks: Map::default(),
            catcodes: Map::default(),
            save: Vec::new(),
            global: false,
            conds: 0,
            next_register: FIRST_FREE_REGISTER,
            expansions: 0,
            statement_expansions: 0,
            after_group: Vec::new(),
            after_assignment: None,
            warnings: Vec::new(),
            environments: HashMap::new(),
            env_stack: Vec::new(),
            named_boxes: Vec::new(),
            box_dims: Map::default(),
        };
        t.catcodes.insert('@', Cat::Letter);
        t
    }

    /// Defines macros from TeX source (at load time; errors are bugs).
    pub fn define_text(&mut self, text: &str) {
        let saved_catcode = self.catcodes.get(&'@').copied();
        self.catcodes.insert('@', Cat::Letter);
        let depth = self.input.len();
        self.push_text(text);
        while self.input.len() > depth {
            match self.get_token() {
                Some(t) => {
                    let _ = self.execute_basic(t);
                }
                None => break,
            }
        }
        match saved_catcode {
            Some(c) => self.catcodes.insert('@', c),
            None => self.catcodes.remove(&'@'),
        };
    }
}
