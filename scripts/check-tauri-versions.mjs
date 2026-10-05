// Tauri checks Rust/JavaScript major and minor versions before bundling. Check
// the resolved lockfiles early so CI catches an incompatible dependency update.
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

function rustVersion(cargoLock, name) {
  const packages = cargoLock.split(/^\[\[package\]\]\s*$/m).slice(1);
  const matches = packages.filter((block) => new RegExp(`^name = "${name}"\\s*$`, "m").test(block));
  if (matches.length !== 1) {
    throw new Error(`Cargo.lock must contain exactly one resolved ${name} package.`);
  }
  const versions = [...matches[0].matchAll(/^version = "([^"\r\n]+)"\s*$/gm)];
  if (versions.length !== 1) {
    throw new Error(`Cargo.lock has a missing or malformed version for ${name}.`);
  }
  return versions[0][1];
}

function majorMinor(version, name, file) {
  const match =
    typeof version === "string" &&
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.exec(
      version,
    );
  if (!match || match[4]?.split(".").some((part) => /^0\d+$/.test(part))) {
    throw new Error(`${file} has an invalid resolved version for ${name}.`);
  }
  return `${match[1]}.${match[2]}`;
}

/** Check the resolved core bridge; plugins and the CLI follow separate release cycles. */
export function checkTauriVersions(cargoLock, packageLock) {
  if (typeof cargoLock !== "string") throw new Error("Cargo.lock must be readable text.");
  let npmLock;
  try {
    npmLock = JSON.parse(packageLock);
  } catch {
    throw new Error("package-lock.json must contain valid JSON.");
  }
  if (
    ![2, 3].includes(npmLock?.lockfileVersion) ||
    !npmLock?.packages ||
    typeof npmLock.packages !== "object" ||
    Array.isArray(npmLock.packages)
  ) {
    throw new Error("package-lock.json must contain resolved packages (lockfile version 2 or 3).");
  }
  const rustPackage = "tauri";
  const npmPackage = "@tauri-apps/api";
  const rust = rustVersion(cargoLock, rustPackage);
  const javascript = npmLock.packages[`node_modules/${npmPackage}`]?.version;
  if (
    majorMinor(rust, rustPackage, "Cargo.lock") !==
    majorMinor(javascript, npmPackage, "package-lock.json")
  ) {
    throw new Error(
      `${rustPackage} ${rust} is incompatible with ${npmPackage} ${javascript}: major/minor versions must match.` +
        "\nAlign the dependency versions and regenerate the corresponding lockfile before building.",
    );
  }
  return { rust, javascript };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const root = resolve(import.meta.dirname, "..");
    const [cargoLock, packageLock] = await Promise.all([
      readFile(resolve(root, "Cargo.lock"), "utf8"),
      readFile(resolve(root, "package-lock.json"), "utf8"),
    ]);
    const { rust, javascript } = checkTauriVersions(cargoLock, packageLock);
    console.log(`tauri ${rust} matches @tauri-apps/api ${javascript} (major/minor).`);
  } catch (error) {
    console.error(`Tauri compatibility check failed: ${error.message}`);
    process.exitCode = 1;
  }
}
