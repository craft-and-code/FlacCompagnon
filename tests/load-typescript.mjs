import { readFileSync } from "node:fs";
import ts from "typescript";

// CI uses Node 20. Pure modules with only type imports can be transpiled in
// memory so the tests exercise the shipped functions without a second build.
export async function loadTypeScript(url) {
  const compiled = ts.transpileModule(readFileSync(url, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
}
