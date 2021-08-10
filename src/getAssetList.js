import chalk from "chalk";
import Promise from "bluebird";
import logUpdate from "log-update";
import { sprintf } from "sprintf-js";
import { decode } from "@msgpack/msgpack";
import { SingleBar, Presets } from "cli-progress";
import {
    fetchWithRetry,
    getBufferChecksum,
    getResponseAssetHash
} from "./utils.js";

const getAssetList = async (args, i18n) => {
    let manifestList = [];
    logUpdate(args.latest ? i18n.getLatestManifest : i18n.getManifestList);
    const res = await fetchWithRetry(
        "https://api.matsurihi.me/mltd/v1/zh/version/" +
            `${args.latest ? "latest" : "assets"}`
    );
    if (!args.latest) {
        (await res.json()).forEach(manifest => {
            let dataURL = "https://d3k5923sb1sy5k.cloudfront.net/";
            dataURL += `${manifest.version}/production/`;
            dataURL += manifest.version < 70000 ? "2017v1" : "2018v1";
            dataURL += `/Android/${manifest.indexName}`;
            manifestList.push({
                ...manifest,
                dataURL
            });
        });
    } else {
        const manifest = (await res.json()).res;
        let dataURL = "https://d3k5923sb1sy5k.cloudfront.net/";
        dataURL += `${manifest.version}/production/`;
        dataURL += manifest.version < 70000 ? "2017v1" : "2018v1";
        dataURL += `/Android/${manifest.indexName}`;
        manifestList.push({
            ...manifest,
            dataURL
        });
    }
    logUpdate(
        `${args.latest ? i18n.getLatestManifest : i18n.getManifestList} ${
            i18n.done
        }`
    );
    logUpdate.done();

    let downloaded;
    if (args.checksum)
        try {
            downloaded = await (
                await import("fs/promises")
            ).readdir(args.outputPath);
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
                chalk.red("{bar}") +
                " {file} (asset {version}) | {value}/{total}"
        },
        Presets.shades_classic
    );
    if (!args.latest) bar.start(manifestList.length, 0);
    const assetList = {};

    await Promise.map(
        manifestList,
        async manifest => {
            const res = await fetchWithRetry(manifest.dataURL);
            const buf = await res.buffer();
            if (getResponseAssetHash(res) !== getBufferChecksum(buf))
                throw new Error(
                    sprintf(i18n.checksumFailed, manifest.indexName)
                );

            const [result] = decode(buf);
            assetList[manifest.version] = [];
            for (const key of Object.keys(result))
                assetList[manifest.version].push({
                    name: key, // asset name
                    hash: result[key][0], // file hash
                    file: result[key][1], // download file name
                    size: result[key][2] // file size
                });
            if (!args.latest)
                bar.increment(1, {
                    file: manifest.indexName,
                    version: manifest.version
                });
        },
        {
            concurrency: parseInt(args.batchSize, 10)
        }
    );

    if (!args.latest) bar.stop();
    logUpdate(`${i18n.downloadingManifest} ${i18n.done}`);
    logUpdate.done();
    return assetList;
};

export default getAssetList;
