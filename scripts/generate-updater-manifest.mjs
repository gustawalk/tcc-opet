import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

const assetsDirectory = process.env.RELEASE_ASSETS_DIR;
const releaseTag = process.env.RELEASE_TAG;
const repository = process.env.GITHUB_REPOSITORY;
const releaseNotesFile = process.env.RELEASE_NOTES_FILE;
const outputPath = process.env.UPDATER_MANIFEST_PATH ?? "updater.json";

if (!assetsDirectory || !releaseTag || !repository || !releaseNotesFile) {
  throw new Error("RELEASE_ASSETS_DIR, RELEASE_TAG, GITHUB_REPOSITORY, and RELEASE_NOTES_FILE are required.");
}

const version = releaseTag.replace(/^v/, "");
const files = await readdir(assetsDirectory, { recursive: true });
const notes = (await readFile(releaseNotesFile, "utf8")).trim();
if (!notes) throw new Error("Release notes file must not be empty.");

const findAsset = (predicate, label) => {
  const fileName = files.find(predicate);
  if (!fileName) throw new Error(`Missing ${label} updater asset.`);
  return fileName;
};

const windowsAsset = findAsset(
  (fileName) => fileName.endsWith(".exe") && !fileName.endsWith(".exe.sig"),
  "Windows",
);
const linuxAsset = findAsset(
  (fileName) => fileName.endsWith(".AppImage"),
  "Linux AppImage",
);

const signatureFor = async (fileName) => {
  const signature = await readFile(path.join(assetsDirectory, `${fileName}.sig`), "utf8");
  if (!signature.trim()) throw new Error(`Empty updater signature for ${fileName}.`);
  return signature.trim();
};

const downloadUrl = (fileName) =>
  `https://github.com/${repository}/releases/download/${releaseTag}/${encodeURIComponent(path.basename(fileName))}`;

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: await signatureFor(windowsAsset),
      url: downloadUrl(windowsAsset),
    },
    "linux-x86_64": {
      signature: await signatureFor(linuxAsset),
      url: downloadUrl(linuxAsset),
    },
  },
};

await writeFile(
  path.resolve(outputPath),
  `${JSON.stringify(manifest, null, 2)}\n`,
  "utf8",
);
