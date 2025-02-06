import dashboardCommonConfig from "dashboard-common/tailwind.config";

// eslint-disable-next-line import/no-default-export
export default {
  ...dashboardCommonConfig,
  content: [
    "./src/**/*.{js,ts,jsx,tsx}",
    "../dashboard-common/src/**/*.{js,ts,jsx,tsx}",
  ],
};
