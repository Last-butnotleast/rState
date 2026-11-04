import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const lock = JSON.parse(
  await readFile(new URL("../reference.lock.json", import.meta.url), "utf8"),
);
await Promise.all(
  lock.files.map(async ({ source, target, sha256 }) => {
    const destination = new URL(`../.reference/${target}`, import.meta.url);
    let contents;
    try {
      contents = await readFile(destination);
    } catch {}
    if (contents && digest(contents) === sha256) return;
    const response = await fetch(
      `https://raw.githubusercontent.com/statelyai/xstate/${lock.commit}/${source}`,
      { signal: AbortSignal.timeout(20_000) },
    );
    if (!response.ok)
      throw new Error(
        `Reference download failed: ${source} (${response.status})`,
      );
    contents = Buffer.from(await response.arrayBuffer());
    if (digest(contents) !== sha256)
      throw new Error(`Reference checksum mismatch: ${source}`);
    await mkdir(dirname(fileURLToPath(destination)), { recursive: true });
    await writeFile(destination, contents);
  }),
);
console.log(
  `Verified @xstate/store ${lock.version} at ${lock.commit.slice(0, 8)}`,
);

function digest(value) {
  return createHash("sha256").update(value).digest("hex");
}
