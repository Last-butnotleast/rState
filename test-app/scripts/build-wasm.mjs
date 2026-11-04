import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const crate = fileURLToPath(
  new URL("../../crates/rstate-wasm", import.meta.url),
);
const output = fileURLToPath(new URL("../src/generated", import.meta.url));
const result = spawnSync(
  "wasm-pack",
  ["build", crate, "--target", "web", "--dev", "--out-dir", output],
  {
    stdio: "inherit",
  },
);
if (result.error) throw result.error;
process.exit(result.status ?? 1);
