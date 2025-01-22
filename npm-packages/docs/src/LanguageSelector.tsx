import React from "react";
import ReactDropdown from "react-dropdown";
import { useSelectedDialect, useSetDialect } from "./theme/Root";

export function LanguageSelector({ verbose }: { verbose?: true }) {
  const dialect = useSelectedDialect();
  const setDialect = useSetDialect();
  const options = [
    dialect === "JS"
      ? { value: "TS", label: label("TS", verbose) }
      : { value: "JS", label: label("JS", verbose) },
  ];
  return (
    <ReactDropdown
      className={`language-selector ${
        verbose ? "language-selector-verbose" : ""
      }`}
      options={options}
      onChange={({ value }) => setDialect(value)}
      value={{ value: dialect, label: label(dialect, verbose) }}
      placeholder="Select language"
    />
  );
}

function label(dialect: "JS" | "TS", verbose: true | undefined): string {
  return verbose ? (dialect === "JS" ? "JavaScript" : "TypeScript") : dialect;
}

export function convertFilePath(
  filename: string,
  dialect: "JS" | "TS",
  overrideDialectExtension?: string,
) {
  const [_, name, extension] = filename.match(/^(.*)\.([^.]*)$/);
  return `${name}.${
    dialect === "JS"
      ? overrideDialectExtension !== undefined
        ? overrideDialectExtension
        : extension.replace("t", "j")
      : extension.replace("j", "t")
  }`;
}
