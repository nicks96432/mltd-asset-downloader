import chalk from "chalk";
import Promise from "bluebird";
import logUpdate from "log-update";
import { sprintf } from "sprintf-js";
import { readdir } from "fs/promises";
import { decode } from "@msgpack/msgpack";
import { SingleBar, Presets } from "cli-progress";

import {
  fetchWithRetry,
  formatBytes,
  getBufferChecksum,
  getResponseAssetHash,
} from "./utils.js";

const getAssetList = async (args, i18n) => {
  let manifestList = [];

  logUpdate(args.latest ? i18n.getLatestManifest : i18n.getManifestList);
  const res = await fetchWithRetry(
    `https://api.matsurihi.me/mltd/v1/version/${
      args.latest ? "latest" : "assets"
    }`
  );
  const result = await res.json();

  if (!args.latest) {
    result.forEach(manifest => {
      let dataURL = args.dataURLBase;
      dataURL += `${manifest.version}/production/201${
        manifest.version < 70000 ? "7" : "8"
      }`;
      dataURL += `/Android/${manifest.indexName}`;
      manifestList.push({
        ...manifest,
        dataURL,
      });
    });
  } else {
    const manifest = result.res;
    let dataURL = args.dataURLBase;
    dataURL += `${manifest.version}/production/201${
      manifest.version < 70000 ? "7" : "8"
    }`;
    dataURL += `/Android/${manifest.indexName}`;
    manifestList.push({ ...manifest, dataURL });
  }
  logUpdate(
    (args.latest ? i18n.getLatestManifest : i18n.getManifestList) + i18n.done
  );
  logUpdate.done();

  let downloaded;
  if (args.checksum)
    try {
      downloaded = await readdir(args.outputPath);
      manifestList = manifestList.filter(manifest =>
        downloaded.includes(manifest.version.toString())
      );
    } catch (e) {
      if (e.code === "EACCES") {
        console.error(sprintf(i18n.eaccesText, args.outputPath));
        process.exit(1);
      }
      manifestList = [];
    }

  logUpdate(i18n.downloadingManifest);
  const bar = new SingleBar(
    {
      clearOnComplete: true,
      format:
        chalk.red("{bar}") + " {file} (asset {version}) | {value}/{total}",
    },
    Presets.shades_classic
  );
  if (!args.latest) bar.start(manifestList.length, 0);
  const assetList = {};

  await Promise.map(
    manifestList,
    async manifest => {
      const res = await fetchWithRetry(manifest.dataURL);
      if (res.status !== 200) return;

      const buf = Buffer.from(await res.arrayBuffer());
      if (getResponseAssetHash(res) !== getBufferChecksum(buf))
        throw new Error(sprintf(i18n.checksumFailed, manifest.indexName));

      const [result] = decode(buf);
      assetList[manifest.version] = [];
      for (const key of Object.keys(result))
        assetList[manifest.version].push({
          name: key, // asset name
          hash: result[key][0], // file hash
          file: result[key][1], // download file name
          size: result[key][2], // file size
        });
      if (!args.latest)
        bar.increment(1, {
          file: manifest.indexName,
          version: manifest.version,
        });
      else {
        logUpdate(
          `downloading ${manifest.version}, ${formatBytes(
            assetList[manifest.version].reduce(
              (size, asset) => size + asset.size,
              0
            )
          )}`
        ); // TODO: i18n
        logUpdate.done();
      }
    },
    {
      concurrency: parseInt(args.batchSize, 10),
    }
  );

  if (!args.latest) bar.stop();
  logUpdate(`${i18n.downloadingManifest} ${i18n.done}`);
  logUpdate.done();
  return assetList;
};

export default getAssetList;
