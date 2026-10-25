import { createReportImport } from "./report-import.js";
import { reportCopy, reportTranslate } from "./report-copy.js";

const element = (tag, className, text) => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text) node.textContent = text;
  return node;
};

// This host owns the local file picker; the Phémios component owns all report rendering.
class ReportViewer extends HTMLElement {
  connectedCallback() {
    if (this.controller) return;
    this.lifecycle = new AbortController();
    const events = { signal: this.lifecycle.signal };
    const locale = document.documentElement.lang === "fr" ? "fr" : "en";
    const copy = reportCopy(locale);
    const translate = reportTranslate(locale);
    const input = element("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.hidden = true;
    input.setAttribute("aria-label", copy.input);
    const drop = element("button", "report-drop");
    drop.type = "button";
    drop.append(
      element("span", "report-drop-icon", "↓"),
      element("strong", "", copy.drop),
      element("span", "", copy.choose),
    );
    const privacy = element("p", "report-privacy", copy.privacy);
    const tools = element("div", "report-tools");
    const name = element("span", "report-name");
    const reset = element("button", "btn btn-outline small", copy.clear);
    reset.type = "button";
    reset.hidden = true;
    tools.append(name, reset);
    const status = element("p", "report-status");
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
    const error = element("p", "report-error");
    error.setAttribute("role", "alert");
    const host = element("div", "report-output");
    let view;
    this.controller = createReportImport({
      show: async (text, current) => {
        const { mountAnalysisReport } =
          await import("./vendor/phemios-analysis/flac-analysis-report.js");
        if (!current()) return;
        const options = { document: text, locale, translate };
        if (view) view.update(options);
        else view = mountAnalysisReport(host, options);
      },
      clear: () => {
        view?.dispose();
        view = undefined;
      },
      state: (next) => {
        status.textContent = next.busy ? copy.reading : "";
        drop.setAttribute("aria-busy", String(next.busy));
        error.textContent = next.error ? translate(next.error) : "";
        if (next.name !== undefined) name.textContent = next.name;
        // Clear can cancel an in-flight read even before the first report is displayed.
        reset.hidden = !name.textContent && !next.busy;
      },
    });
    input.addEventListener("change", () => {
      const files = [...(input.files ?? [])];
      input.value = "";
      void this.controller.open(files);
    });
    drop.addEventListener("click", () => input.click());
    reset.addEventListener("click", () => this.controller.reset());
    let depth = 0;
    // Cover the complete reader so replacing a report by dropping onto its results stays local.
    this.addEventListener(
      "dragenter",
      (event) => {
        event.preventDefault();
        depth++;
        drop.dataset.dragging = "true";
      },
      events,
    );
    this.addEventListener(
      "dragover",
      (event) => {
        event.preventDefault();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
      },
      events,
    );
    this.addEventListener(
      "dragleave",
      () => {
        depth = Math.max(0, depth - 1);
        if (!depth) delete drop.dataset.dragging;
      },
      events,
    );
    this.addEventListener(
      "drop",
      (event) => {
        event.preventDefault();
        depth = 0;
        delete drop.dataset.dragging;
        void this.controller.open([...(event.dataTransfer?.files ?? [])]);
      },
      events,
    );
    this.append(input, drop, privacy, tools, status, error, host);
  }

  disconnectedCallback() {
    this.lifecycle?.abort();
    this.controller?.reset();
    this.controller = undefined;
    this.replaceChildren();
  }
}
customElements.define("flac-report-viewer", ReportViewer);
