const path = require("path");

module.exports = {
	target: "node",
	entry: {
		index: "./src",
	},
	output: {
		path: path.join(__dirname, "/dist"),
		filename: "mltd-asset-downloader.js",
	},
	module: {
		rules: [
			{
				test: /\.js$/,
				exclude: /node_modules/,
			},
		],
	},
	resolve: { extensions: [".js"] },
};
