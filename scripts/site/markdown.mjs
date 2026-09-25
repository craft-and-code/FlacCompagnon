import { Marked } from "marked";
import richtypo from "richtypo";
import frenchRules from "richtypo/rules/fr";
import { escapeHtml as esc, externalDocumentationPrefixes, pages, repository } from "./catalog.mjs";

const aliases = {
  README: "index.html",
  "audio-measurements": "index.html#loudness",
  discontinuities: "index.html#integrity",
  "dsd-authenticity": "index.html#dsd",
};

function opensInNewTab(href) {
  return externalDocumentationPrefixes.some((prefix) => href.startsWith(prefix));
}

export function renderMarkdown(source, language = "en") {
  const headings = [];
  const used = new Map();
  const parser = new Marked({
    gfm: true,
    walkTokens(token) {
      if (token.type !== "link" || /^(https?:|mailto:|#)/.test(token.href)) return;
      const [file, hash] = token.href.split("#");
      const slug = file.replace(/\.md$/, "");
      if (aliases[slug]) token.href = aliases[slug];
      else if (pages.some((p) => p.slug === slug))
        token.href = `${slug}.html${hash ? `#${hash}` : ""}`;
      else
        token.href = `${repository}/blob/main/${file.replace(/^\.\.\//, "")}${hash ? `#${hash}` : ""}`;
    },
    renderer: {
      heading({ tokens, depth }) {
        const title = this.parser.parseInline(tokens);
        const plain = title.replace(/<[^>]*>/g, "");
        const base = plain
          .toLowerCase()
          .normalize("NFD")
          .replace(/[\u0300-\u036f]/g, "")
          .replace(/[^a-z0-9]+/g, "-")
          .replace(/^-|-$/g, "");
        const count = used.get(base) || 0;
        used.set(base, count + 1);
        const id = count ? `${base}-${count}` : base;
        if (depth === 2) headings.push({ title: plain, id });
        return `<h${depth} id="${id}">${title}<a class="heading-link" href="#${id}" aria-label="${esc(plain)}">#</a></h${depth}>`;
      },
      table(token) {
        const cell = (c, tag) => `<${tag}>${this.parser.parseInline(c.tokens)}</${tag}>`;
        return `<div class="guide-table" tabindex="0"><table><thead><tr>${token.header.map((c) => cell(c, "th")).join("")}</tr></thead><tbody>${token.rows.map((row) => `<tr>${row.map((c) => cell(c, "td")).join("")}</tr>`).join("")}</tbody></table></div>`;
      },
      link({ href, title, tokens }) {
        const label = this.parser.parseInline(tokens);
        const titleAttribute = title ? ` title="${esc(title)}"` : "";
        const newTabAttributes = opensInNewTab(href) ? ' target="_blank" rel="noopener"' : "";
        return `<a href="${esc(href)}"${titleAttribute}${newTabAttributes}>${label}</a>`;
      },
    },
  });
  // Only repository-authored Markdown is rendered; no runtime/user Markdown.
  const content = source.replace(/^# .+\n+/, "");
  const typography = language === "fr" ? richtypo(frenchRules, content) : content;
  const html = parser.parse(typography);
  return { html, headings };
}
