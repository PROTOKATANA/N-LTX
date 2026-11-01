use std::collections::HashMap;
use std::sync::OnceLock;

use mathra::locale::Locale;

/// The phrases of each language, in the order of [`Locale::ALL`].
const SOURCES: [&str; 16] = [
    include_str!("../locales/en.json"),
    include_str!("../locales/pt-BR.json"),
    include_str!("../locales/es.json"),
    include_str!("../locales/fr.json"),
    include_str!("../locales/de.json"),
    include_str!("../locales/it.json"),
    include_str!("../locales/ru.json"),
    include_str!("../locales/zh.json"),
    include_str!("../locales/ja.json"),
    include_str!("../locales/ko.json"),
    include_str!("../locales/hi.json"),
    include_str!("../locales/ar.json"),
    include_str!("../locales/id.json"),
    include_str!("../locales/tr.json"),
    include_str!("../locales/nl.json"),
    include_str!("../locales/pl.json"),
];

pub(in crate::describe) fn phrases(locale: Locale) -> &'static HashMap<String, String> {
    static TABLE: OnceLock<Vec<HashMap<String, String>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        Locale::ALL
            .iter()
            .zip(SOURCES)
            .map(|(l, json)| {
                let map: HashMap<String, String> = serde_json::from_str(json).unwrap_or_else(|e| panic!("src/locales/{}.json: {e}", l.code()));
                assert_eq!(map.get("code").map(String::as_str), Some(l.code()), "the file of {l:?} declares another code");
                map
            })
            .collect()
    });
    let i = Locale::ALL.iter().position(|l| *l == locale).unwrap_or(0);
    &table[i]
}

/// The plural form of `n` in the language (CLDR's categories, as far as these phrases need them).
pub(in crate::describe) fn plural_form(locale: Locale, n: usize) -> &'static str {
    let (m10, m100) = (n % 10, n % 100);
    match locale {
        Locale::Zh | Locale::Ja | Locale::Ko | Locale::Id | Locale::Tr => "other",
        Locale::PtBr | Locale::Fr | Locale::Hi if n <= 1 => "one",
        Locale::Ru if m10 == 1 && m100 != 11 => "one",
        Locale::Ru | Locale::Pl if (2..=4).contains(&m10) && !(12..=14).contains(&m100) => "few",
        Locale::Ru => "many",
        Locale::Pl if n == 1 => "one",
        Locale::Pl => "many",
        Locale::Ar => match (n, m100) {
            (1, _) => "one",
            (2, _) => "two",
            (_, 3..=10) => "few",
            _ => "other",
        },
        _ if n == 1 => "one",
        _ => "other",
    }
}

/// The words of one language.
pub(in crate::describe) struct Words {
    pub(in crate::describe) locale: Locale,
}

impl Words {
    pub(in crate::describe) fn phrase(&self, key: &str) -> &'static str {
        let own = phrases(self.locale);
        own.get(key).or_else(|| phrases(Locale::En).get(key)).map_or_else(|| panic!("phrase '{key}' missing from src/locales/en.json"), String::as_str)
    }

    /// A phrase with `{0}`, `{1}`… replaced by `args`.
    pub(in crate::describe) fn t(&self, key: &str, args: &[&str]) -> String {
        let mut s = self.phrase(key).to_string();
        for (i, a) in args.iter().enumerate() {
            s = s.replace(&format!("{{{i}}}"), a);
        }
        s
    }

    /// `n` things: the language's plural form of `key`, then its `other`, then English's.
    pub(in crate::describe) fn count(&self, key: &str, n: usize) -> String {
        let form = plural_form(self.locale, n);
        let own = phrases(self.locale);
        let en = phrases(Locale::En);
        let k = format!("{key}.{form}");
        let other = format!("{key}.other");
        let en_form = format!("{key}.{}", plural_form(Locale::En, n));
        let p = own.get(&k).or_else(|| own.get(&other)).or_else(|| en.get(&en_form)).or_else(|| en.get(&other));
        p.map_or_else(|| panic!("phrase '{other}' missing from src/locales/en.json"), |p| p.replace("{0}", &n.to_string()))
    }

    pub(in crate::describe) fn quote(&self, s: &str) -> String {
        self.t("quote", &[s])
    }

    pub(in crate::describe) fn quoted_list(&self, items: &[String]) -> String {
        items.iter().map(|t| self.quote(t)).collect::<Vec<_>>().join(self.phrase("sep.items"))
    }

    /// Items joined by the list separator, the first `cap` of them and how many more.
    pub(in crate::describe) fn capped(&self, items: &[String], cap: usize) -> String {
        let sep = self.phrase("sep.list");
        let mut s = items.iter().take(cap).cloned().collect::<Vec<_>>().join(sep);
        if items.len() > cap {
            s.push_str(sep);
            s.push_str(&self.t("and_more", &[&(items.len() - cap).to_string()]));
        }
        s
    }

    pub(in crate::describe) fn sentence(&self, s: &str) -> String {
        format!("{s}{}", self.phrase("end"))
    }

    /// Between the numbers of a pair or a list: a comma, or a semicolon where the comma is decimal.
    pub(in crate::describe) fn between(&self) -> &'static str {
        if self.locale.decimal_separator() == ',' {
            "; "
        } else {
            self.phrase("sep.items")
        }
    }

    pub(in crate::describe) fn num(&self, v: f64, span: f64) -> String {
        let s = super::speech::num_in(v, span);
        match self.locale.decimal_separator() {
            '.' => s,
            c => s.replace('.', &c.to_string()),
        }
    }

    pub(in crate::describe) fn spoken(&self, tex: &str) -> String {
        super::speech::spoken(tex, self.locale)
    }
}
