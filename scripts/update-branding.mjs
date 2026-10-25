import { cp, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

// Generate every desktop/site size from the approved press master to avoid branding drift.
const root = path.resolve(import.meta.dirname, ".."),
  master = path.join(root, "press/logos/flaccompagnon-icon-1024.png"),
  output = await mkdtemp(path.join(tmpdir(), "flaccompagnon-icons-")),
  cli = path.join(root, "node_modules/@tauri-apps/cli/tauri.js");
function generate(options) {
  const run = spawnSync(process.execPath, [cli, "icon", master, "--output", output, ...options], {
    cwd: root,
    stdio: "inherit",
  });
  if (run.error) throw run.error;
  if (run.status !== 0) throw new Error(`Icon generation failed (${run.status}).`);
}
const copy = (source, target) => cp(path.join(output, source), path.join(root, target));
try {
  generate([]);
  generate(["16", "32", "48", "180", "512"].flatMap((size) => ["--png", size]));
  for (const name of ["32x32.png", "128x128.png", "128x128@2x.png", "icon.icns", "icon.ico"])
    await copy(name, `src-tauri/icons/${name}`);
  for (const [size, file] of [
    [32, "32x32.png"],
    [128, "128x128.png"],
    [256, "128x128@2x.png"],
    [512, "512x512.png"],
  ])
    await copy(file, `src-tauri/icons/icon-${size}.png`);
  await cp(master, path.join(root, "src-tauri/icons/app-icon.png"));
  for (const size of [16, 32, 48, 180])
    await copy(`${size}x${size}.png`, `site/assets/favicon-${size}.png`);
  await copy("icon.ico", "site/assets/favicon.ico");
  await copy("128x128.png", "site/assets/logo.png");
  await cp(
    path.join(root, "press/logos/flaccompagnon-icon.svg"),
    path.join(root, "site/assets/logo.svg"),
  );
  console.log("Updated desktop and website icon exports from the approved press master.");
} finally {
  await rm(output, { recursive: true, force: true });
}
