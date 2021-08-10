import inquirer from "inquirer";
import { formatBytes } from "./utils.js";

const getDownloadList = async (assetList, args, i18n) => {
    const assetVersions = Object.keys(assetList);
    if (assetVersions.length === 1 || args.checksum) return assetVersions;

    let confirm, answers;
    while (!confirm) {
        answers = await inquirer.prompt([
            {
                type: "checkbox",
                name: "downloadList",
                pageSize: 15,
                message: i18n.downloadMessage,
                choices: assetVersions.reverse().map(a => {
                    const name = `${a} (${assetList[a].length} ${
                        i18n.file
                    }, ${formatBytes(
                        assetList[a].reduce(
                            (size, asset) => size + asset.size,
                            0
                        )
                    )})`;
                    return { name, value: a };
                }),
                loop: false
            }
        ]);
        answers = answers.downloadList;
        confirm = await inquirer.prompt([
            {
                type: "confirm",
                name: "confirm",
                message: i18n.confirmDownload
            }
        ]);
        confirm = confirm.confirm;
    }
    return answers;
};

export default getDownloadList;
