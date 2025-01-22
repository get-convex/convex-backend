export function classNames(
  ...classes: (string | boolean | undefined | null)[]
) {
  return classes.filter((s) => typeof s === "string").join(" ");
}
