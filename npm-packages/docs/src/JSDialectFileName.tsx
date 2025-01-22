import React from "react";
import { useSelectedDialect } from "./theme/Root";
import { convertFilePath } from "./LanguageSelector";

export function JSDialectFileName({
  name,
  ext,
}: {
  name: string;
  ext?: string;
}) {
  const selectedDialect = useSelectedDialect();

  return <code>{convertFilePath(name, selectedDialect, ext)}</code>;
}
