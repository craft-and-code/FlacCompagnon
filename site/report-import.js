// Local file admission is separate from presentation so late reads cannot restore a cleared report.
export const MAX_REPORT_BYTES = 16 * 1024 * 1024;

export function createReportImport({ show, clear, state }) {
  let generation = 0;
  return {
    async open(files) {
      if (!files?.length) return;
      const ticket = ++generation;
      state({ busy: true, error: null });
      try {
        if (files.length !== 1) throw new Error("Choose one JSON report at a time.");
        const file = files[0];
        if (!/\.json$/i.test(file.name))
          throw new Error("Choose a JSON report exported by FlacCompagnon.");
        if (file.size > MAX_REPORT_BYTES) throw new Error("The report exceeds the 16 MiB limit.");
        const document = await file.text();
        if (ticket !== generation) return;
        // show validates before replacing an existing report, including when the renderer loads lazily.
        await show(document, () => ticket === generation);
        if (ticket === generation) state({ busy: false, error: null, name: file.name });
      } catch (error) {
        if (ticket === generation)
          state({
            busy: false,
            error: error instanceof Error ? error.message : "Unable to read this report.",
          });
      }
    },
    reset() {
      generation++;
      clear();
      state({ busy: false, error: null, name: "" });
    },
  };
}
