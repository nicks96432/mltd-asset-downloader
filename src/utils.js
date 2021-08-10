import crypto from "crypto";
import Promise from "bluebird";
import fetch from "node-fetch";

export const fetchWithRetry = async (url, method, retry) => {
    if (method === undefined) method = "GET";
    if (retry === undefined) retry = 3;
    try {
        return await fetch(url, { method });
    } catch (e) {
        if (retry > 0) {
            await new Promise(resolve => setTimeout(() => resolve(), 500));
            return await fetchWithRetry(url, method, retry - 1);
        }
        console.error(e.message);
    }
};

export const getResponseAssetHash = res =>
    Buffer.from(
        res.headers.get("x-goog-hash").replace(/^.*md5=/, ""),
        "base64"
    ).toString("hex");

export const getBufferChecksum = buf =>
    crypto.createHash("md5").update(buf).digest("hex");

export const formatBytes = bytes => {
    if (bytes === 0) return "0 Bytes";
    const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB"];
    const power = Math.floor(Math.log2(bytes) / 10);
    return `${(bytes / Math.pow(1024, power)).toFixed(3)} ${sizes[power]}`;
};
