import path from "path";

export default () => ({
  name: "imageZoom",
  getClientModules() {
    return [path.resolve(__dirname, "./imageZoom")];
  },
});
