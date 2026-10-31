// colors the static <pre data-lang="js|html|py|rust|sh"> blocks; without it they stay plain text
(function () {
  const escape = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const span = (cls, t) => `<span class="c-${cls}">${escape(t)}</span>`;

  // each language: one regex whose groups are, in order, the classes below
  const STR = String.raw`"(?:[^"\\\n]|\\.)*"|'(?:[^'\\\n]|\\.)*'`;
  const LANGS = {
    js: {
      re: new RegExp(String.raw`(\/\/[^\n]*)|(${STR}|\x60[^\x60]*\x60)|\b(import|from|const|let|await|async|function|return|new|export)\b|\b(\d+(?:\.\d+)?)\b|([A-Za-z_$][\w$]*(?=\x60|\())`, "g"),
      classes: ["comment", "str", "kw", "num", "fn"],
    },
    py: {
      re: new RegExp(String.raw`(#[^\n]*)|([rRbBfF]?(?:${STR}))|\b(import|from|as|def|return|None|True|False)\b|\b(\d+(?:\.\d+)?)\b|([A-Za-z_]\w*(?=\())`, "g"),
      classes: ["comment", "str", "kw", "num", "fn"],
    },
    rust: {
      re: new RegExp(String.raw`(\/\/[^\n]*)|(${STR})|\b(let|mut|fn|use|pub|struct|impl|match|return)\b|\b(\d+(?:\.\d+)?)\b|([A-Za-z_]\w*!?(?=\())`, "g"),
      classes: ["comment", "str", "kw", "num", "fn"],
    },
    // one command line: the program after the start or a pipe, quoted arguments, flags, a trailing comment
    sh: {
      re: new RegExp(String.raw`((?:^|\s)#[^\n]*)|(${STR})|((?:^|[|;&]\s*)[A-Za-z][\w-]*)|((?:^|\s)--?[A-Za-z][\w-]*)`, "g"),
      classes: ["comment", "str", "fn", "flag"],
    },
  };
  // the html example is a <script> around JavaScript
  LANGS.html = { re: new RegExp(String.raw`(<\/?[a-z][^>]*>)|` + LANGS.js.re.source, "g"), classes: ["tag", ...LANGS.js.classes] };

  function color(src, lang) {
    let html = "", at = 0;
    for (const m of src.matchAll(lang.re)) {
      const i = m.findIndex((g, k) => k > 0 && g !== undefined);
      // a match may start with the space or pipe before the token: keep that plain
      const lead = m[0].match(/^[\s|;&]*/)[0];
      html += escape(src.slice(at, m.index) + lead) + span(lang.classes[i - 1], m[0].slice(lead.length));
      at = m.index + m[0].length;
    }
    return html + escape(src.slice(at));
  }

  // a shell block with "$ " prompts is a session: only those lines are commands, the rest is output
  function shell(src) {
    const session = /^\$ /m.test(src);
    return src.split("\n").map((line) => {
      if (!session) return color(line, LANGS.sh);
      if (!line.startsWith("$ ")) return escape(line);
      return span("prompt", "$ ") + color(line.slice(2), LANGS.sh);
    }).join("\n");
  }

  document.addEventListener("DOMContentLoaded", () => {
    for (const pre of document.querySelectorAll("pre[data-lang]")) {
      const lang = pre.dataset.lang;
      if (lang === "sh") pre.innerHTML = shell(pre.textContent);
      else if (LANGS[lang]) pre.innerHTML = color(pre.textContent, LANGS[lang]);
    }
  });
})();
