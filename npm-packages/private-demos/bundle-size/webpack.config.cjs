const path = require("path");

module.exports = {
  mode: "production",
  devtool: "source-map",
  entry: "./src/main.tsx",
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "webpack.bundle.js",
  },
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
    ],
  },
};
