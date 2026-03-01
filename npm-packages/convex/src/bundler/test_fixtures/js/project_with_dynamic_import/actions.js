export async function loadModule() {
  const mod = await import("./helper.js");
  return mod.default();
}
