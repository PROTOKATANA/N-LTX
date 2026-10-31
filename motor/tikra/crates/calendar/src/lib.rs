//! The calendar library and pgf's date macros (`pgfcalendar.code.tex`, `tikzlibrarycalendar`).
//!
//! `\pgfcalendar{prefix}{start}{end}{code}` runs `code` once for every date from `start` to `end`
//! (`2000-01-01`, `2000-02-last`, `2000-01-01+5`), in one group, with `\pgfcalendarcurrentyear`,
//! `…month`, `…day` (two digits), `…weekday` (0 is Monday) and `…julian` set, and `\ifdate{tests}{yes}{no}`
//! choosing by the date: `Sunday`, `weekend`, `equals=…`, `day of month=1`, `between=… and …`…
//! A test can be a key of the document's, `/pgf/calendar/name/.code`: it sees the date in
//! `\pgfcalendarifdateyear`, `…month`, `…day`, `…weekday` and the count `\pgfcalendarifdatejulian`,
//! and says `\pgfcalendarmatchestrue` when the date passes. pgf's own tests stay as they are here
//! even when the document redefines one (its code is often expl3's, which tikra does not read).
//!
//! `\calendar (name) at (point) [options] if (tests) [options] {code} else …;` is a `\pgfcalendar`
//! whose code puts each day in a scope: `execute before day scope`, then the scope with `execute at
//! begin day scope`, the `if`s, the day code (a node with the day's number, named
//! `name-2000-01-20`, the name given by `(name)` or `name=`) and `execute at end day scope`, then `execute after day scope`. The
//! arrangements (`week list`, `day list downward`…) and month labels are styles on these hooks;
//! the shifts they make stay from one day to the next, as the transformation of the group does.

use tikra_engine::engine::piece::{CommandFn, Piece};
use tikra_engine::engine::{macro_from, macro_tokens, tikra_tokens, Engine};
use tikra_engine::scene::Transform;
use tikra_engine::tex::{self, Cat, Meaning, Token};
use tikra_engine::tex_dates::{date, easter, parse_date, weekday};
use tikra_engine::Error;

/// The calendar in an engine.
pub static PIECE: Piece = Piece { name: "calendar", commands: COMMANDS, define, definitions: || DEFINITIONS.to_string() };

const COMMANDS: &[(&str, CommandFn)] = &[
    ("calendar", |e, _| calendar(e)),
    ("pgfcalendar", pgfcalendar_command),
    ("ifdate", ifdate),
    ("pgfcalendarifdate", pgfcalendarifdate),
    ("pgfcalendardatetojulian", date_to_julian),
    ("pgfcalendarjuliantodate", julian_to_date),
    ("pgfcalendarjuliantoweekday", julian_to_weekday),
    ("tikra@calendar", calendar_step_command),
    ("tikra@calendarif", calendar_if_key),
];

fn define(e: &mut Engine) {
    // TeX's \year, \month, \day and \time: the date of the run (UTC)
    let (y, m, d, minutes) = today();
    for (name, v) in [("year", y), ("month", m), ("day", d), ("time", minutes)] {
        let r = e.tex.new_register();
        e.tex.define(name, Meaning::Count(r));
        e.tex.set_count(r, v, true);
    }
    // the date a test of the document's sees, as a count
    let r = e.tex.new_register();
    e.tex.define("pgfcalendarifdatejulian", Meaning::Count(r));
}

/// Today's date and the minutes since midnight; on the web, where there is no clock to read
/// without a host, the first of January 2000.
fn today() -> (i64, i64, i64, i64) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let secs = t.as_secs() as i64;
        // the Julian day of 1970-01-01 is 2440588
        let (y, m, d) = date(2440588 + secs / 86400);
        return (y, m, d, secs % 86400 / 60);
    }
    (2000, 1, 1, 0)
}

/// The calendar's keys and templates, and the names of days and months.
pub const DEFINITIONS: &str = r"
\newif\ifpgfcalendarmatches
\def\tikra@cal@append#1#2{\expandafter\def\expandafter#1\expandafter{#1#2}}
\def\tikra@cal@prepend#1#2{\def\tikra@cal@temp{#2}\expandafter\expandafter\expandafter\def\expandafter\expandafter\expandafter#1\expandafter\expandafter\expandafter{\expandafter\tikra@cal@temp#1}}
\def\tikra@cal@before{}\def\tikra@cal@after{}\def\tikra@cal@atbegin{}\def\tikra@cal@atend{}
\def\tikra@cal@dates{}\def\tikz@lib@cal@width{1}
\def\tikz@lib@cal@xshift{3.5ex}\def\tikz@lib@cal@yshift{3ex}
\def\tikz@lib@cal@month@xshift{9ex}\def\tikz@lib@cal@month@yshift{9ex}
% each day, month and year is a node with its style and text; a day is named after its date
\def\tikra@cal@node#1#2{\node[#2every #1]{\csname tikz#1text\endcsname};}
\def\tikzdaycode{\tikra@cal@node{day}{name=\pgfcalendarsuggestedname,}}
\def\tikzmonthcode{\tikra@cal@node{month}{}}\def\tikzyearcode{\tikra@cal@node{year}{}}
\def\tikzdaytext{\%d-}\def\tikzmonthtext{\%mt}\def\tikzyeartext{\%y0}
\tikzset{
  dates/.code={\def\tikra@cal@dates{#1}},
  execute before day scope/.code={\tikra@cal@append\tikra@cal@before{#1}},
  execute after day scope/.code={\tikra@cal@append\tikra@cal@after{#1}},
  execute at begin day scope/.code={\tikra@cal@append\tikra@cal@atbegin{#1}},
  execute at end day scope/.code={\tikra@cal@prepend\tikra@cal@atend{#1}},
  day xshift/.code={\def\tikz@lib@cal@xshift{#1}}, day yshift/.code={\def\tikz@lib@cal@yshift{#1}},
  month xshift/.code={\def\tikz@lib@cal@month@xshift{#1}}, month yshift/.code={\def\tikz@lib@cal@month@yshift{#1}},
  day code/.code={\def\tikzdaycode{#1}}, day text/.code={\def\tikzdaytext{#1}},
  month code/.code={\def\tikzmonthcode{#1}}, month text/.code={\def\tikzmonthtext{#1}},
  year code/.code={\def\tikzyearcode{#1}}, year text/.code={\def\tikzyeartext{#1}},
  if/.code={\tikra@calendarif{#1}}, days/.style={every day/.append style={#1}}, every day/.style={anchor=base east},
  every month/.style={}, every year/.style={},
  tikz@lib@cal@width/.code={\def\tikz@lib@cal@width{#1}},
  day list downward/.style={execute before day scope={\tikra@calendar{month down}},
    execute after day scope={\tikra@calendar{day down}}, tikz@lib@cal@width=1},
  day list upward/.style={execute before day scope={\tikra@calendar{month up}},
    execute after day scope={\tikra@calendar{day up}}, tikz@lib@cal@width=1},
  day list right/.style={execute before day scope={\tikra@calendar{month right}},
    execute after day scope={\tikra@calendar{day right}}, tikz@lib@cal@width=30},
  day list left/.style={execute before day scope={\tikra@calendar{month left}},
    execute after day scope={\tikra@calendar{day left}}, tikz@lib@cal@width=30},
  week list/.style={execute before day scope={\tikra@calendar{month down}},
    execute at begin day scope={\tikra@calendar{weekday}},
    execute after day scope={\tikra@calendar{week}}, tikz@lib@cal@width=7},
  month list/.style={execute before day scope={\tikra@calendar{month down}\tikra@calendar{month start}},
    execute at begin day scope={\tikra@calendar{day of month}}, tikz@lib@cal@width=37},
  month label left/.style={execute before day scope={\tikra@calendar{label left}},
    every month/.append style={anchor=base east,xshift=-3.5ex}},
  month label left vertical/.style={execute before day scope={\tikra@calendar{label left}},
    every month/.append style={anchor=base east,xshift=-4.5ex,yshift=2.25ex,rotate=90}},
  month label right/.style={execute before day scope={\tikra@calendar{label right}},
    every month/.append style={anchor=base west,xshift=1ex}},
  month label right vertical/.style={execute before day scope={\tikra@calendar{label right}},
    every month/.append style={anchor=base west,xshift=2ex,yshift=2.25ex,rotate=-90}},
  month label above centered/.style={execute before day scope={\tikra@calendar{label above centered}},
    every month/.append style={anchor=base}},
  month label above left/.style={execute before day scope={\tikra@calendar{label above left}},
    every month/.append style={anchor=base west}},
  month label above right/.style={execute before day scope={\tikra@calendar{label above right}},
    every month/.append style={anchor=base east}},
  month label below centered/.style={execute before day scope={\tikra@calendar{label below centered}},
    every month/.append style={anchor=base}},
  month label below left/.style={execute before day scope={\tikra@calendar{label below left}},
    every month/.append style={anchor=base west}},
}
\def\pgfcalendarweekdayname#1{\ifcase#1 Monday\or Tuesday\or Wednesday\or Thursday\or Friday\or Saturday\or Sunday\fi}
\def\pgfcalendarweekdayshortname#1{\ifcase#1 Mon\or Tue\or Wed\or Thu\or Fri\or Sat\or Sun\fi}
\def\pgfcalendarmonthname#1{\ifcase#1\or January\or February\or March\or April\or May\or June\or July\or August\or September\or October\or November\or December\fi}
\def\pgfcalendarmonthshortname#1{\ifcase#1\or Jan\or Feb\or Mar\or Apr\or May\or Jun\or Jul\or Aug\or Sep\or Oct\or Nov\or Dec\fi}
\def\pgfcalendarshorthand#1#2{\csname tikra@cal@short@#1#2\endcsname}
\expandafter\def\csname tikra@cal@short@d-\endcsname{\tikra@cal@day}
\expandafter\def\csname tikra@cal@short@d=\endcsname{\tikra@cal@paddedday}
\expandafter\def\csname tikra@cal@short@d0\endcsname{\pgfcalendarcurrentday}
\expandafter\def\csname tikra@cal@short@m-\endcsname{\tikra@cal@month}
\expandafter\def\csname tikra@cal@short@m=\endcsname{\tikra@cal@paddedmonth}
\expandafter\def\csname tikra@cal@short@m0\endcsname{\pgfcalendarcurrentmonth}
\expandafter\def\csname tikra@cal@short@y-\endcsname{\pgfcalendarcurrentyear}
\expandafter\def\csname tikra@cal@short@y=\endcsname{\pgfcalendarcurrentyear}
\expandafter\def\csname tikra@cal@short@y0\endcsname{\pgfcalendarcurrentyear}
\expandafter\def\csname tikra@cal@short@w.\endcsname{\pgfcalendarweekdayshortname{\pgfcalendarcurrentweekday}}
\expandafter\def\csname tikra@cal@short@wt\endcsname{\pgfcalendarweekdayname{\pgfcalendarcurrentweekday}}
\expandafter\def\csname tikra@cal@short@m.\endcsname{\pgfcalendarmonthshortname{\pgfcalendarcurrentmonth}}
\expandafter\def\csname tikra@cal@short@mt\endcsname{\pgfcalendarmonthname{\pgfcalendarcurrentmonth}}
";

/// The tests `\ifdate` knows itself.
const TESTS: &[&str] = &[
    "all",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "workday",
    "weekend",
    "equals",
    "day of month",
    "end of month",
    "at least",
    "at most",
    "between",
    "Easter",
];

fn pgfcalendar_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let prefix = e.argument_string()?;
    let start = date_argument(e)?;
    let end = date_argument(e)?;
    let code = e.argument()?;
    pgfcalendar(e, Dates { prefix: &prefix, start: &start, end: &end }, code)
}

fn ifdate(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let tests = e.argument()?;
    let yes = e.argument()?;
    let no = e.argument()?;
    let j = macro_int(e, "pgfcalendarcurrentjulian").ok_or_else(|| Error::Figure("\\ifdate outside \\pgfcalendar".into()))?;
    let m = date_matches(e, j, tests)?;
    e.tex.push_tokens(if m { yes } else { no });
    Ok(())
}

fn pgfcalendarifdate(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let d = date_argument(e)?;
    let tests = e.argument()?;
    let yes = e.argument()?;
    let no = e.argument()?;
    let j = parse_date(&d, None).ok_or_else(|| Error::Figure(format!("not a date: '{d}'")))?;
    let m = date_matches(e, j, tests)?;
    e.tex.push_tokens(if m { yes } else { no });
    Ok(())
}

fn date_to_julian(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let d = date_argument(e)?;
    let target = e.argument()?;
    let j = parse_date(&d, None).ok_or_else(|| Error::Figure(format!("not a date: '{d}'")))?;
    set_int(e, target, j);
    Ok(())
}

fn julian_to_date(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let j = e.argument()?;
    let j = e.tex.int_of(j)?;
    let (y, m, d) = date(j);
    for s in [y.to_string(), format!("{m:02}"), format!("{d:02}")] {
        let target = e.argument()?;
        if let Some(t) = target.into_iter().find(|t| !t.is_space()) {
            e.tex.set_meaning(&t, macro_from(&s), false);
        }
    }
    Ok(())
}

fn julian_to_weekday(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let j = e.argument()?;
    let j = e.tex.int_of(j)?;
    let target = e.argument()?;
    set_int(e, target, weekday(j));
    Ok(())
}

/// The `if` key: its value read as the `if` of a \\calendar.
fn calendar_if_key(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let mut v = e.argument()?;
    v.push(Token::cs("relax"));
    let depth = e.tex.depth();
    e.tex.push_frame(v);
    let r = calendar_if(e);
    e.tex.drain_to(depth);
    r
}

fn calendar_step_command(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let what = e.argument_string()?;
    calendar_step(e, &what)
}

/// A count register or a macro set to a number (`\pgfcalendardatetojulian{…}{\c}`).
fn set_int(e: &mut Engine, target: Vec<Token>, v: i64) {
    let Some(t) = target.into_iter().find(|t| !t.is_space()) else { return };
    match e.tex.meaning(&t) {
        Meaning::Count(r) => e.tex.set_count(r, v, false),
        _ => e.tex.set_meaning(&t, macro_from(&v.to_string()), false),
    }
}

/// A macro holding a number, as a number.
fn macro_int(e: &mut Engine, name: &str) -> Option<i64> {
    if !e.tex.is_defined(name) {
        return None;
    }
    e.tex.expand_to_string(vec![Token::cs(name)]).ok()?.trim().parse().ok()
}

/// A date argument: its macros expanded and its registers read (`\the\year-01-01`, `\year-\month-\day`).
fn date_argument(e: &mut Engine) -> Result<String, Error> {
    let a = e.argument()?;
    Ok(e.math_string(a)?.trim().to_string())
}

/// The text of a macro, expanded.
fn macro_text(e: &mut Engine, name: &str) -> String {
    e.tex.expand_to_string(vec![Token::cs(name)]).map(|s| s.trim().to_string()).unwrap_or_default()
}

/// `\pgfcalendar`: `code` for every date from `start` to `end`, in one group.
/// The dates of a `\\pgfcalendar`.
struct Dates<'a> {
    prefix: &'a str,
    start: &'a str,
    end: &'a str,
}

fn pgfcalendar(e: &mut Engine, Dates { prefix, start, end }: Dates<'_>, code: Vec<Token>) -> Result<(), Error> {
    let first = parse_date(start, None).ok_or_else(|| Error::Figure(format!("not a date: '{start}'")))?;
    let last = parse_date(end, None).ok_or_else(|| Error::Figure(format!("not a date: '{end}'")))?;
    if last - first > 20_000 {
        return Err(Error::Figure(format!("calendar from {start} to {end}: too many days")));
    }
    e.begin_group();
    e.tex.define_string("pgfcalendarprefix", prefix, false);
    e.tex.define_string("pgfcalendarbeginiso", start.trim(), false);
    e.tex.define_string("pgfcalendarendiso", end.trim(), false);
    e.tex.define_string("pgfcalendarbeginjulian", &first.to_string(), false);
    e.tex.define_string("pgfcalendarendjulian", &last.to_string(), false);
    let mut r = Ok(());
    for j in first..=last {
        let (y, m, d) = date(j);
        for (name, v) in [
            ("pgfcalendarcurrentjulian", j.to_string()),
            ("pgfcalendarcurrentyear", y.to_string()),
            ("pgfcalendarcurrentmonth", format!("{m:02}")),
            ("pgfcalendarcurrentday", format!("{d:02}")),
            ("pgfcalendarcurrentweekday", weekday(j).to_string()),
            ("tikra@cal@day", d.to_string()),
            ("tikra@cal@month", m.to_string()),
            ("pgfcalendarsuggestedname", if prefix.is_empty() { String::new() } else { format!("{prefix}-{y}-{m:02}-{d:02}") }),
        ] {
            e.tex.define_string(name, &v, false);
        }
        // `\%d=`: a figure's width before a single digit
        let pad = |v: i64| if v < 10 { format!("\\phantom{{1}}{v}") } else { v.to_string() };
        e.tex.define_macro("tikra@cal@paddedday", tikra_tokens(&pad(d)), false);
        e.tex.define_macro("tikra@cal@paddedmonth", tikra_tokens(&pad(m)), false);
        e.count_step()?;
        r = e.run_tokens(code.clone());
        if r.is_err() {
            break;
        }
    }
    e.end_group();
    r
}

/// Whether the date `j` passes one of `tests` (a key list: `Sunday, equals=2000-01-20`).
fn date_matches(e: &mut Engine, j: i64, tests: Vec<Token>) -> Result<bool, Error> {
    let (year, month, day) = date(j);
    let wd = weekday(j);
    for (name, v) in [
        ("pgfcalendarifdateyear", year.to_string()),
        ("pgfcalendarifdatemonth", format!("{month:02}")),
        ("pgfcalendarifdateday", format!("{day:02}")),
        ("pgfcalendarifdateweekday", wd.to_string()),
    ] {
        e.tex.define_string(name, &v, false);
    }
    if let Meaning::Count(r) = e.tex.meaning_of("pgfcalendarifdatejulian") {
        e.tex.set_count(r, j, false);
    }
    let mut matches = false;
    for item in tikra_engine::keys::split_list(&tests) {
        // a test of the document's: its code says `\pgfcalendarmatchestrue`
        let path = format!("/pgf/calendar/{}", item.key);
        if !TESTS.contains(&item.key.as_str()) && e.key_exists(&path) {
            e.tex.set_meaning(&Token::cs("ifpgfcalendarmatches"), Meaning::Prim(tex::Prim::Iffalse), false);
            e.run_key(&path, item.value)?;
            matches |= matches!(e.tex.meaning_of("ifpgfcalendarmatches"), Meaning::Prim(tex::Prim::Iftrue));
            continue;
        }
        let value = match item.value {
            Some(v) => Some(e.math_string(v)?.trim().to_string()),
            None => None,
        };
        let at = |s: &str| parse_date(s, Some(year)).ok_or_else(|| Error::Figure(format!("not a date: '{s}'")));
        let v = value.as_deref().unwrap_or("");
        matches |= match item.key.as_str() {
            "all" => true,
            "Monday" => wd == 0,
            "Tuesday" => wd == 1,
            "Wednesday" => wd == 2,
            "Thursday" => wd == 3,
            "Friday" => wd == 4,
            "Saturday" => wd == 5,
            "Sunday" => wd == 6,
            "workday" => wd < 5,
            "weekend" => wd > 4,
            "equals" => j == at(v)?,
            "day of month" => v.parse::<i64>().ok() == Some(day),
            "end of month" => date(j + value.as_deref().and_then(|v| v.parse().ok()).unwrap_or(1)).2 == 1,
            "at least" => j >= at(v)?,
            "at most" => j <= at(v)?,
            "between" => {
                let (a, b) = v.split_once(" and ").or_else(|| v.split_once("and")).ok_or_else(|| Error::Figure(format!("between={v}: no 'and'")))?;
                j >= at(a)? && j <= at(b)?
            }
            "Easter" => j == easter(year) + value.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0),
            other => {
                e.warn(format!("\\ifdate: unknown test '{other}'"));
                false
            }
        };
    }
    Ok(matches)
}

/// `\calendar (name) at (point) [options] if (tests) … ;`
fn calendar(e: &mut Engine) -> Result<(), Error> {
    e.begin_group();
    let r = calendar_body(e);
    e.end_group();
    r
}

fn calendar_body(e: &mut Engine) -> Result<(), Error> {
    let shorthand = e.tex.meaning_of("pgfcalendarshorthand");
    e.tex.set_meaning(&Token::cs("%"), shorthand, false);
    for name in ["tikra@cal@before", "tikra@cal@after", "tikra@cal@atbegin", "tikra@cal@atend", "tikra@cal@ifs"] {
        e.tex.define_string(name, "", false);
    }
    e.st.pending = Transform::default();
    e.st.node.at = None;
    e.st.node.name = None;
    e.process_keys(tikra_tokens("every calendar/.try"), Some("/tikz"))?;
    let mut guard = 0;
    loop {
        guard += 1;
        let Some(t) = e.tex.next_nonspace() else { break };
        if guard > 1000 {
            return Err(Error::Figure("\\calendar: giving up on this calendar".into()));
        }
        match &t {
            t if t.is_char(';') => break,
            t if t.is_char('(') => {
                // `(name)` is `name=`: the last one names the days
                let name = e.tex.read_until_char(')', false)?;
                e.st.node.name = Some(e.tex.expand_to_string(name)?.trim().to_string());
            }
            t if t.is_char('[') => {
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
            }
            t if t.is_char('a') => {
                // at (point)
                let _ = e.tex.get_token();
                let p = match e.tex.next_nonspace() {
                    Some(x) if x.is_char('(') => e.tex.read_until_char(')', false)?,
                    _ => return Err(Error::Figure("\\calendar at: a point is missing".into())),
                };
                let s = e.tex.expand_to_string(p)?;
                e.st.node.at = Some(e.coordinate(&s)?.p);
            }
            t if t.is_char('i') => {
                let _ = e.tex.get_token();
                calendar_if(e)?;
            }
            _ => {
                let m = e.tex.meaning(&t);
                if m.is_expandable() {
                    e.tex.expand(&t, &m)?;
                } else {
                    return Err(Error::Figure(format!("\\calendar: unexpected {}", tex::show_token(&t))));
                }
            }
        }
    }
    e.st.apply_pending();
    let prefix = e.st.node.name.take().unwrap_or_default();
    // `at (point)` or `at=`: where the calendar's origin goes
    if let Some(p) = e.st.node.at.take() {
        let local = e.st.transform.inverse().apply(p);
        e.st.transform = e.st.transform.compose(&Transform::translation(local.x, local.y));
    }
    let dates = e.math_string(vec![Token::cs("tikra@cal@dates")])?;
    let dates = dates.trim();
    let Some((start, end)) = dates.split_once("to") else {
        return Err(Error::Figure(format!("\\calendar: dates={dates} is not 'start to end'")));
    };
    let code = tikra_tokens(r"\tikra@cal@before\begin{scope}\tikra@cal@atbegin\tikra@cal@ifs\tikzdaycode\tikra@cal@atend\end{scope}\tikra@cal@after");
    pgfcalendar(e, Dates { prefix: &prefix, start, end }, code)
}

/// `if (tests) [options] else {code}` (either code an `[options]` or a `{code}`, the `else` part
/// optional): appended to what each day's scope runs.
fn calendar_if(e: &mut Engine) -> Result<(), Error> {
    let tests = match e.tex.next_nonspace() {
        Some(x) if x.is_char('(') => e.tex.read_until_char(')', false)?,
        _ => return Err(Error::Figure("\\calendar if: the tests are missing".into())),
    };
    let yes = calendar_if_branch(e)?;
    let no = match e.tex.next_nonspace() {
        Some(x) if x.is_char('e') => {
            for _ in 0..3 {
                let _ = e.tex.get_token();
            }
            calendar_if_branch(e)?
        }
        Some(x) => {
            e.tex.back(x);
            Vec::new()
        }
        None => Vec::new(),
    };
    let mut ifs = match e.tex.meaning_of("tikra@cal@ifs") {
        Meaning::Macro(m) => tex::substitute(&m.body, &[]),
        _ => Vec::new(),
    };
    ifs.push(Token::cs("ifdate"));
    for v in [tests, yes, no] {
        ifs.push(Token::Char('{', Cat::Begin));
        ifs.extend(v);
        ifs.push(Token::Char('}', Cat::End));
    }
    e.tex.set_meaning(&Token::cs("tikra@cal@ifs"), macro_tokens(ifs), false);
    Ok(())
}

/// The code of an `if`: `[options]` (set in the day's scope) or `{code}`.
fn calendar_if_branch(e: &mut Engine) -> Result<Vec<Token>, Error> {
    match e.tex.next_nonspace() {
        Some(x) if x.is_char('[') => {
            let o = e.tex.read_until_char(']', false)?;
            let mut v = vec![Token::cs("tikzset"), Token::Char('{', Cat::Begin)];
            v.extend(o);
            v.push(Token::Char('}', Cat::End));
            Ok(v)
        }
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body(),
        Some(x) => {
            e.tex.back(x);
            Ok(Vec::new())
        }
        None => Ok(Vec::new()),
    }
}

/// A length from a calendar macro (`\tikz@lib@cal@xshift`), in cm; `em` and `ex` are the current
/// font's.
fn calendar_length(e: &mut Engine, name: &str) -> Result<f64, Error> {
    let s = macro_text(e, name);
    font_length(e, &s)
}

fn font_length(e: &mut Engine, s: &str) -> Result<f64, Error> {
    // `em` and `ex` in the ambient font; the calendar's own `font=` counts too
    let v = e.length(s)?;
    let sans = e.st.font.contains("\\sffamily");
    let quad = |pt: f64| tikra_engine::label::quad(pt, sans);
    Ok(if s.contains("em") {
        v * e.st.font_size * quad(e.st.font_size) / (e.st.ambient_size * quad(e.st.ambient_size))
    } else if s.contains("ex") {
        v * e.st.font_size / e.st.ambient_size
    } else {
        v
    })
}

fn calendar_shift(e: &mut Engine, x: f64, y: f64) {
    e.st.transform = e.st.transform.compose(&Transform::translation(x, y));
}

/// `\tikra@calendar{what}`: one step of an arrangement or a month label, for the current day.
fn calendar_step(e: &mut Engine, what: &str) -> Result<(), Error> {
    let j = macro_int(e, "pgfcalendarcurrentjulian").unwrap_or(0);
    let begin = macro_int(e, "pgfcalendarbeginjulian").unwrap_or(0);
    let (_, _, day) = date(j);
    let wd = weekday(j);
    // a new month moves the days on, except at the calendar's first day
    let new_month = day == 1 && j != begin;
    match what {
        "month down" | "month up" if new_month => {
            let d = calendar_length(e, "tikz@lib@cal@month@yshift")?;
            calendar_shift(e, 0.0, if what == "month up" { d } else { -d });
        }
        "month right" | "month left" if new_month => {
            let d = calendar_length(e, "tikz@lib@cal@month@xshift")?;
            calendar_shift(e, if what == "month right" { d } else { -d }, 0.0);
        }
        "day down" | "day up" => {
            let d = calendar_length(e, "tikz@lib@cal@yshift")?;
            calendar_shift(e, 0.0, if what == "day up" { d } else { -d });
        }
        "day right" | "day left" => {
            let d = calendar_length(e, "tikz@lib@cal@xshift")?;
            calendar_shift(e, if what == "day right" { d } else { -d }, 0.0);
        }
        "weekday" => {
            let d = calendar_length(e, "tikz@lib@cal@xshift")?;
            calendar_shift(e, wd as f64 * d, 0.0);
        }
        "week" if wd == 6 => {
            let d = calendar_length(e, "tikz@lib@cal@yshift")?;
            calendar_shift(e, 0.0, -d);
        }
        "month start" if day == 1 || j == begin => {
            // the weekday of the month's first day, for `month list`
            let first = weekday(j - day + 1);
            e.tex.define_string("tikra@cal@monthstart", &first.to_string(), false);
        }
        "day of month" => {
            let d = calendar_length(e, "tikz@lib@cal@xshift")?;
            let first = macro_int(e, "tikra@cal@monthstart").unwrap_or(0);
            calendar_shift(e, (day + first - 1) as f64 * d, 0.0);
        }
        label if label.starts_with("label ") && day == 1 => calendar_month_label(e, &label[6..])?,
        _ => {}
    }
    Ok(())
}

/// The month's label on its first day, placed as `month label …` says, in a group.
fn calendar_month_label(e: &mut Engine, place: &str) -> Result<(), Error> {
    let x = calendar_length(e, "tikz@lib@cal@xshift")?;
    let y = calendar_length(e, "tikz@lib@cal@yshift")?;
    let width = macro_text(e, "tikz@lib@cal@width").parse::<f64>().unwrap_or(1.0);
    let ex = font_length(e, "1ex")?;
    let row = (width - 1.0) * x;
    let (dx, dy) = match place {
        "right" => (row, 0.0),
        "above centered" => (row / 2.0 - 1.5 * ex, 1.25 * y),
        "above left" => (-3.25 * ex, 1.25 * y),
        "above right" => (row, 1.25 * y),
        "below centered" => (row / 2.0 - 1.5 * ex, -1.25 * y),
        "below left" => (-3.25 * ex, -1.25 * y),
        _ => (0.0, 0.0),
    };
    e.begin_group();
    calendar_shift(e, dx, dy);
    let r = e.run_tokens(vec![Token::cs("tikzmonthcode")]);
    e.end_group();
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use tikra_engine::canvas::Canvas;
    use tikra_engine::engine::piece::Parts;

    fn run(code: &str) -> Result<(Canvas, Vec<String>), Error> {
        let parts = Parts { pieces: &[&PIECE], libraries: &[] };
        tikra_engine::engine::run_document_in(code, parts).map(|(canvas, warnings, _)| (canvas, warnings))
    }

    #[test]
    fn a_week_list_names_its_days() {
        let (canvas, warnings) =
            run(r"\begin{tikzpicture}\calendar (c) [dates=2000-01-01 to 2000-01-31,week list] if (Sunday) [red];\draw (c-2000-01-20) circle (4pt);\end{tikzpicture}").unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(canvas.items.len() > 31);
    }

    #[test]
    fn name_names_the_days_as_a_name_in_parentheses_does() {
        // the last of `(name)` and `name=` wins
        for opts in ["[name=cal,dates=2000-01-01 to 2000-01-31]", "(c) [name=cal,dates=2000-01-01 to 2000-01-31]", "[name=c,dates=2000-01-01 to 2000-01-31] (cal)"] {
            let doc = format!(r"\begin{{tikzpicture}}\calendar {opts};\draw (cal-2000-01-20.north west) circle (4pt);\end{{tikzpicture}}");
            let (_, warnings) = run(&doc).unwrap();
            assert!(warnings.is_empty(), "{opts}: {warnings:?}");
        }
    }

    #[test]
    fn a_test_of_the_document_sees_the_date() {
        // odd Julian days, and a day of the month given as the value or by default
        let (canvas, warnings) = run(r"\pgfkeys{/pgf/calendar/odd/.code={\ifodd\pgfcalendarifdatejulian\pgfcalendarmatchestrue\fi}}
\pgfkeys{/pgf/calendar/on/.default=2,/pgf/calendar/on/.code={\ifnum\pgfcalendarifdateday=#1\relax\ifnum\pgfcalendarifdatemonth=1 \pgfcalendarmatchestrue\fi\fi}}
\begin{tikzpicture}
\pgfcalendar{c}{2000-01-01}{2000-01-05}{\ifdate{odd}{\fill (\pgfcalendarcurrentday,0) circle (1pt);}{}}
\pgfcalendar{c}{2000-01-01}{2000-01-05}{\ifdate{on,on=4}{\fill (\pgfcalendarcurrentday,1) circle (1pt);}{}}
\end{tikzpicture}")
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        // 2000-01-01 is the Julian day 2451545: the 1st, 3rd and 5th, then the 2nd and 4th
        assert_eq!(canvas.items.len(), 5);
    }

    #[test]
    fn a_layout_of_the_document_reads_the_calendar_registers() {
        // pgf's scratch count and the library's shifts, in calendars of whole months side by side
        let (_, warnings) = run(r"\makeatletter
\tikzset{my week/.style={execute at begin day scope={\c@pgf@counta=\pgfcalendarcurrentweekday
  \pgfmathsetlength\pgf@x{\tikz@lib@cal@xshift}\pgf@x=\c@pgf@counta\pgf@x\pgftransformxshift{\pgf@x}},
  execute after day scope={\ifdate{Sunday}{\pgfmathsetlength\pgf@y{\tikz@lib@cal@yshift}\pgftransformyshift{-\pgf@y}}{}},
  tikz@lib@cal@width=7}}
\makeatother
\begin{tikzpicture}
\matrix{\calendar[dates=2000-01-01 to 2000-01-last,my week]; & \calendar[dates=2000-02-01 to 2000-02-last,my week]; \\};
\end{tikzpicture}")
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
