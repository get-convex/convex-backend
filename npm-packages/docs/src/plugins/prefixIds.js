// https://github.com/facebook/docusaurus/issues/8297#issuecomment-2156076317
const path = require("path");

module.exports = function svgFixPlugin() {
  return {
    name: "svg-fix",
    configureWebpack(config) {
      const svgRule = config.module?.rules?.find((r) =>
        r.test.test("file.svg"),
      );
      if (!svgRule) {
        console.warn(
          "Failed to apply SVG fix, could not find SVG rule in webpack config!",
        );
        return {};
      }
      const svgrLoader = svgRule.oneOf?.find(
        (r) =>
          r.use?.length === 1 && r.use?.[0].loader.includes("@svgr/webpack"),
      );
      if (!svgrLoader) {
        console.warn(
          "Failed to apply SVG fix, could not find svgr loader in webpack config!",
        );
        return {};
      }

      const svgoConfig = svgrLoader.use[0].options.svgoConfig;
      if (!svgoConfig?.plugins) {
        console.warn(
          "Failed to apply SVG fix, could not find svgo config in webpack config!",
        );
        return {};
      }

      svgoConfig.plugins.push({
        name: "prefixIds",
        params: {
          delim: "_",
          prefix: (element, file) => {
            return path.basename(file?.path ?? "").split(".")[0];
          },
          prefixIds: true,
          prefixClassNames: true,
        },
      });

      return {};
    },
  };
};
