import { spawnSync } from "node:child_process";
import { error } from "node:console";
import { homedir } from "node:os";
import { resolve } from "node:path";
import process from "node:process";
import { fileURLToPath, URL } from "node:url";

const args = process.argv.slice(2);
const env = { ...process.env };

// The release executable is itself the portable STDIO helper. Rust embeds
// dependency source filenames (including the build user's home) even in a
// release without debug symbols. Prevent those paths from reaching the ZIP.
if (args[0] === "build") {
  const workspace = fileURLToPath(new URL("..", import.meta.url)).replace(/[\\/]$/, "");
  const cargo = resolve(env.CARGO_HOME || resolve(homedir(), ".cargo"));
  const former = env.CARGO_ENCODED_RUSTFLAGS
    ? env.CARGO_ENCODED_RUSTFLAGS.split("\x1f")
    : (env.RUSTFLAGS || "").split(/\s+/).filter(Boolean);
  delete env.RUSTFLAGS;
  const mapped = [
    [workspace, "/agent-desk-src"],
    [cargo, "/agent-desk-cargo"],
    [homedir(), "/agent-desk-build-home"],
  ];
  const flags = mapped
    .sort(([a], [b]) => a.length - b.length)
    .flatMap(([path, redacted]) => {
      const variants = new Set([path, path.replaceAll("\\", "/")]);
      return [...variants].map((source) => `--remap-path-prefix=${source}=${redacted}`);
    });
  if (process.platform === "win32") flags.push("-C", "link-arg=/PDBALTPATH:%_PDB%");
  env.CARGO_ENCODED_RUSTFLAGS = [...former, ...flags].join("\x1f");
}

const cli = fileURLToPath(new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url));
const result = spawnSync(process.execPath, [cli, ...args], { env, stdio: "inherit" });
if (result.error) error(result.error.message);
process.exitCode = result.status ?? 1;
