import { build } from "esbuild";
import { mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const outdir = join(tmpdir(), "flaccompagnon-table-performance");
await mkdir(outdir, { recursive: true });
await build({ entryPoints: ["tests/table-performance.tsx"], bundle: true, minify: true,
  jsx: "automatic", define: { "process.env.NODE_ENV": '"production"' }, outdir });
await writeFile(join(outdir, "index.html"), `<!doctype html><html lang="en"><head>
<meta charset="utf-8"><title>Table performance regression</title>
<link rel="stylesheet" href="table-performance.css"></head><body><div id="root"></div>
<script type="module" src="table-performance.js"></script></body></html>`);
console.log(outdir);
