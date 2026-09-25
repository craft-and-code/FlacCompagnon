const french = document.documentElement.lang === "fr";
const technical = document.querySelector("#technical");

// Fragment links also work when the requested section starts collapsed.
function revealFragment() {
  let id;
  try {
    id = decodeURIComponent(location.hash.slice(1));
  } catch {
    return;
  }
  const target = document.getElementById(id);
  if (!target) return;
  if (technical?.contains(target)) technical.open = true;
  requestAnimationFrame(() => target.scrollIntoView());
}
window.addEventListener("hashchange", revealFragment);
if (location.hash) revealFragment();
document.querySelectorAll('a[href^="#"]').forEach((link) =>
  link.addEventListener("click", () => {
    const target = document.getElementById(link.hash.slice(1));
    if (technical?.contains(target)) technical.open = true;
  }),
);

const sidebar = document.querySelector(".guide-sidebar");
if (sidebar) {
  const scrollKey = `fc-guide-sidebar-scroll:${document.documentElement.lang}`;
  const sectionsKey = `fc-guide-sidebar-sections:${document.documentElement.lang}`;
  const sections = [...sidebar.querySelectorAll("[data-guide-nav-section]")];
  const currentSection = sections.find((section) => section.querySelector("[aria-current]"));
  try {
    const savedSections = JSON.parse(sessionStorage.getItem(sectionsKey) || "{}");
    for (const section of sections) {
      if (typeof savedSections[section.dataset.guideNavSection] === "boolean") {
        section.open = savedSections[section.dataset.guideNavSection];
      }
    }
  } catch {}
  // The current item must stay visible after navigation, even if its group
  // was closed on the page the visitor came from.
  if (currentSection) currentSection.open = true;
  const saveSectionState = () => {
    try {
      sessionStorage.setItem(
        sectionsKey,
        JSON.stringify(
          Object.fromEntries(
            sections.map((section) => [section.dataset.guideNavSection, section.open]),
          ),
        ),
      );
    } catch {}
  };
  sections.forEach((section) => section.addEventListener("toggle", saveSectionState));
  const saveSidebarScroll = () => {
    try {
      sessionStorage.setItem(scrollKey, String(sidebar.scrollTop));
    } catch {}
  };
  try {
    const saved = sessionStorage.getItem(scrollKey);
    if (saved !== null) {
      const scrollTop = Number(saved);
      if (Number.isFinite(scrollTop)) requestAnimationFrame(() => (sidebar.scrollTop = scrollTop));
    }
  } catch {}
  sidebar.addEventListener("scroll", saveSidebarScroll, { passive: true });
  window.addEventListener("pagehide", saveSidebarScroll);
}
document.querySelectorAll("[data-language]").forEach((link) =>
  link.addEventListener("click", () => {
    try {
      localStorage.setItem("fc-lang", link.dataset.language);
    } catch {}
  }),
);

document.querySelectorAll(".guide-prose pre").forEach((pre) => {
  const code = pre.querySelector("code");
  const language = [...code.classList]
    .find((name) => name.startsWith("language-"))
    ?.slice("language-".length);
  const terminalLabel =
    language === "powershell"
      ? "PowerShell"
      : ["sh", "bash", "zsh", "shell"].includes(language)
        ? "Terminal"
        : null;
  if (terminalLabel) {
    pre.classList.add("terminal-code-block");
    const header = document.createElement("div");
    header.className = "terminal-code-header";
    const controls = document.createElement("span");
    controls.className = "terminal-code-controls";
    for (const color of ["red", "yellow", "green"]) {
      const control = document.createElement("i");
      control.className = `terminal-code-control ${color}`;
      controls.append(control);
    }
    const title = document.createElement("span");
    title.className = "terminal-code-title";
    title.textContent = terminalLabel;
    header.append(controls, title);
    pre.prepend(header);
  }
  if (!navigator.clipboard?.writeText) return;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "copy-code";
  button.textContent = french ? "Copier" : "Copy";
  button.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(code.textContent.replace(/\r?\n$/, ""));
      button.textContent = french ? "Copié ✓" : "Copied ✓";
    } catch {
      button.textContent = french ? "Sélectionnez le texte" : "Select the text";
    }
    setTimeout(() => {
      button.textContent = french ? "Copier" : "Copy";
    }, 2500);
  });
  pre.prepend(button);
});

const search = document.querySelector(".guide-search");
const normalize = (value) =>
  value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "");
if (search) {
  search.hidden = false;
  const items = [...document.querySelectorAll(".analysis-group li")];
  search.querySelector("input").addEventListener("input", (event) => {
    const terms = normalize(event.target.value).trim().split(/\s+/);
    let count = 0;
    for (const item of items) {
      item.hidden = !terms.every((term) => normalize(item.textContent).includes(term));
      if (!item.hidden) count++;
    }
    document.querySelectorAll(".analysis-group").forEach((group) => {
      group.hidden = !group.querySelector("li:not([hidden])");
    });
    document.querySelector(".guide-search-status").textContent = french
      ? `${count} analyse${count > 1 ? "s" : ""} trouvée${count > 1 ? "s" : ""}`
      : `${count} ${count === 1 ? "analysis" : "analyses"} found`;
  });
}
