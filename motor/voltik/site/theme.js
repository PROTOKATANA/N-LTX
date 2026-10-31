// light unless the visitor picked dark
(function () {
  const root = document.documentElement;
  try { const t = localStorage.getItem("voltik-theme"); if (t) root.dataset.theme = t; } catch (e) {}
  document.addEventListener("DOMContentLoaded", () => {
    const button = document.getElementById("theme");
    if (!button) return;
    button.onclick = () => {
      root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
      try { localStorage.setItem("voltik-theme", root.dataset.theme); } catch (e) {}
    };
  });
})();
