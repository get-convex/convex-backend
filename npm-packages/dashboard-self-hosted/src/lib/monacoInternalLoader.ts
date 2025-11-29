import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";

loader.config({ monaco });

loader.init().then((_monacoInstance) => {
  /* ... */
});
