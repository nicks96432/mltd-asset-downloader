import path from "path";
import chalk from "chalk";
import fs from "fs/promises";
import Promise from "bluebird";
import logUpdate from "log-update";
import { sprintf } from "sprintf-js";
import { Presets, SingleBar } from "cli-progress";
import getDownloadList from "./getDownloadList.js";
import {
    fetchWithRetry,
    getBufferChecksum,
    getResponseAssetHash
} from "./utils.js";

const downloadAssets = async (assetList, args, i18n) => {
    const downloadList = await getDownloadList(assetList, args, i18n);
    const bar = new SingleBar(
        {
            clearOnComplete: true,
            format: `${chalk.blue("{bar}")} {file} | {value}/{total}`,
            stopOnComplete: true
        },
        Presets.shades_classic
    );
    if (!args.dryRun && !args.checksum)
        try {
            await fs.mkdir(args.outputPath, { recursive: true });
        } catch (e) {}
    for (const assetVersion of downloadList) {
        const outputPath = path.join(args.outputPath, assetVersion);
        logUpdate(sprintf(i18n.downloadAssets, outputPath));
        if (!args.dryRun && !args.checksum)
            try {
                await fs.mkdir(outputPath, { recursive: true });
            } catch (e) {
                if (e.code === "EACCES") {
                    console.error(sprintf(i18n.eaccesText, args.outputPath));
                    process.exit(1);
                }
            }
        bar.start(assetList[assetVersion].length, 0);
        await Promise.map(
            assetList[assetVersion],
            async assetListItem => {
                let dataURL = `https://d3k5923sb1sy5k.cloudfront.net/${assetVersion}/production/`;
                assetVersion < 70000
                    ? (dataURL += `2017v1`)
                    : (dataURL += `2018v1`);
                dataURL += `/Android/${assetListItem.file}`;
                if (!args.dryRun || args.checksum)
                    try {
                        const res = await fetchWithRetry(dataURL, "HEAD");
                        const buf = await fs.readFile(
                            `./${args.outputPath}/${assetVersion}/${assetListItem.name}`
                        );
                        if (
                            getResponseAssetHash(res) === getBufferChecksum(buf)
                        ) {
                            bar.increment(1, { file: assetListItem.name });
                            return;
                        } else if (args.checksum) {
                            console.error(
                                sprintf(i18n.checksumFailed, manifest.indexName)
                            );
                            process.exit(1);
                        }
                    } catch (e) {}
                const res = await fetchWithRetry(dataURL);
                const buf = await res.buffer();
                if (getResponseAssetHash(res) !== getBufferChecksum(buf)) {
                    console.error(
                        sprintf(i18n.checksumFailed, manifest.indexName)
                    );
                    process.exit(1);
                }
                if (!args.dryRun)
                    await fs.writeFile(
                        path.join(outputPath, assetListItem.name),
                        buf
                    );
                bar.increment(1, { file: assetListItem.name });
            },
            { concurrency: parseInt(args.batchSize, 10) }
        );
        bar.stop();
        logUpdate(sprintf(`${i18n.downloadAssets} ${i18n.done}`, outputPath));
        logUpdate.done();
    }
};

export default downloadAssets;
