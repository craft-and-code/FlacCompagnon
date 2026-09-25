import assert from "node:assert/strict";
import test from "node:test";
import { renderMarkdown } from "../scripts/site/markdown.mjs";

test("opens Aede and Rustdoc links in a new tab", () => {
  const { html } = renderMarkdown(
    [
      "[Aede](https://craft-and-code.github.io/aede/)",
      "[Rustdoc](https://craft-and-code.github.io/FlacCompagnon/doc/)",
      "[Local guide](integrated-loudness.md)",
      "[External source](https://example.com/reference)",
    ].join("\n\n"),
  );

  assert.match(
    html,
    /<a href="https:\/\/craft-and-code\.github\.io\/aede\/" target="_blank" rel="noopener">Aede<\/a>/,
  );
  assert.match(
    html,
    /<a href="https:\/\/craft-and-code\.github\.io\/FlacCompagnon\/doc\/" target="_blank" rel="noopener">Rustdoc<\/a>/,
  );
  assert.match(html, /<a href="integrated-loudness\.html">Local guide<\/a>/);
  assert.doesNotMatch(html, /example\.com\/reference" target="_blank"/);
});

test("applies French typography without changing inline or fenced code", () => {
  const { html } = renderMarkdown(
    [
      "Puis analysez-les :",
      "",
      '`echo "colon : preserve"`',
      "",
      "```sh",
      'echo "colon : preserve"',
      "```",
    ].join("\n"),
    "fr",
  );

  assert.match(html, /Puis analysez-les :<\/p>/);
  assert.match(html, /<code>echo &quot;colon : preserve&quot;<\/code>/);
  assert.match(html, /<code class="language-sh">echo &quot;colon : preserve&quot;\n<\/code>/);
});
