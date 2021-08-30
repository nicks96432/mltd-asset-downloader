import os from "os";
import logUpdate from "log-update";
import { Command } from "commander";
import packageInfo from "../package.json";
import getAssetList from "./getAssetList.js";
import downloadAssets from "./downloadAssets.js";

const supportLocales = ["en-US", "zh-TW", "ko-KR"];

const main = async () => {
    let locale = Intl.DateTimeFormat().resolvedOptions().locale;
    if (!supportLocales.includes(locale)) locale = supportLocales[0];
    const i18n = (
        await import(/* webpackMode: "eager" */ `./i18n/${locale}.json`)
    ).default;

    process.on("SIGINT", () => {
        logUpdate(i18n.sigintText);
        logUpdate.done();
        process.exit(1);
    });

    const args = new Command()
        .usage(i18n.cliUsage)
        .description(i18n.cliDescription)
        .version(packageInfo.version, "-V, --version", i18n.cliVersion)
        .option("--latest", i18n.cliLatest)
        .option("--dry-run", i18n.cliDryRun)
        .option("--checksum", i18n.cliChecksum)
        .option(
            "-b, --batch-size <size>",
            i18n.cliBatchSize,
            os.cpus().length.toString()
        )
        .option("-o, --output-path <path>", i18n.cliOutputPath, "assets")
        .helpOption("-h, --help", i18n.cliHelp)
        .parse()
        .opts();

    const assetList = await getAssetList(args, i18n);
    await downloadAssets(assetList, args, i18n);
    if (!args.checksum) console.log(i18n.downloadComplete);
    else console.log(i18n.checksumComplete);
};

main();
