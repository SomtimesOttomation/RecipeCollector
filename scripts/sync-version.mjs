// Copies the version in VERSION into tauri.conf.json, package.json and the app's Cargo.toml,
// so "prep release" only has to change VERSION. Run before every build (CI does this).
import { readFileSync, writeFileSync } from "node:fs";

const version = readFileSync("VERSION", "utf8").trim();
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`VERSION must look like 1.2.3, got "${version}"`);

const editJson = (file) => {
  const data = JSON.parse(readFileSync(file, "utf8"));
  data.version = version;
  writeFileSync(file, JSON.stringify(data, null, 2) + "\n");
};
editJson("src-tauri/tauri.conf.json");
editJson("package.json");

const cargo = "src-tauri/Cargo.toml";
writeFileSync(cargo, readFileSync(cargo, "utf8").replace(/^version = ".*"$/m, `version = "${version}"`));
console.log(`Version set to ${version}`);
