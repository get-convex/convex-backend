import { Project } from "ts-morph";
import { diffStringsUnified } from "jest-diff";
import chalk from "chalk";
import { relative } from "path";

export function diffProjects(
  root: string,
  original: Project,
  modified: Project,
) {
  // This assumes that the files in both projects are the same.
  // This works as long as codemods don’t create/delete/move files.
  const files = original.getSourceFiles();
  for (const file of files) {
    const originalCode = file.getFullText();
    const modifiedCode = modified
      .getSourceFileOrThrow(file.getFilePath())
      .getFullText();
    if (originalCode === modifiedCode) {
      continue;
    }

    console.log(chalk.bold(relative(root, file.getFilePath())));

    const diff = diffStringsUnified(originalCode, modifiedCode, {
      aColor: chalk.red,
      bColor: chalk.green,
      omitAnnotationLines: true,
      contextLines: 2,
      expand: false,
    });
    console.log(diff);
    console.log();
  }
}
