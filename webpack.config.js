const path = require("path");

module.exports = {
  mode: "production",
  target: "node",
  entry: { index: "./src/index.js" },
  output: {
    path: path.join(__dirname, "/build"),
    filename: "[name].bundle.js",
    clean: true,
  },

  resolve: { extensions: [".js"] },
};
