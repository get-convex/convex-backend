// @ts-expect-error this library doesn't assume Node.js but process will get replaced during build
export const version = process.env.npm_package_version ?? "0.0.0";
