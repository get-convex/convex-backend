import { describe, expect } from "vitest";
import path, { relative } from "node:path";
import fs from "node:fs";
import { CommentRange, Project, SourceFile } from "ts-morph";
import { diffStringsUnified } from "jest-diff";
import { explicitIds } from "@/codemods/explicit-ids";
import { Context } from "@/context";
import { findTsconfig } from "@/load";
import { generateCodeframe } from "@/util/codeframe";

const WARNING_COMMENT_PREFIX = "WARNING:";

describe("explicit-ids", () => {
  const testRoot = path.join(__dirname, "explicit-ids");

  // Get all test projects
  const testFolders = fs
    .readdirSync(testRoot, { withFileTypes: true })
    .filter((dirent) => dirent.isDirectory())
    .map((dirent) => dirent.name);

  for (const folderName of testFolders) {
    describe(folderName, async () => {
      const folderPath = path.join(testRoot, folderName);

      // Verify that the project’s dependencies are installed (node_modules exist)
      if (!fs.existsSync(path.join(folderPath, "node_modules"))) {
        throw new Error(
          "node_modules not found in the project, was `npm install` ran before the test?",
        );
      }

      const expectedWarnings: Set<CommentRange> = new Set();

      const context: Context = {
        crash: async ({ printedMessage }) => {
          throw new Error(printedMessage ?? "Unknown error");
        },
        spinner: undefined,
        addWarning: ({ title, message, node }) => {
          const expectedComment = `${WARNING_COMMENT_PREFIX} ${title} / ${message}`;

          const comments = node.getTrailingCommentRanges();
          if (comments.length !== 1) {
            console.log(node.getSourceFile().getFilePath());
            console.log(generateCodeframe(node, message));

            throw new Error(
              `Warning emitted: ${title} ${message} ${node.getText()}`,
            );
          }

          const actualComment = stripCommentDelimiters(comments[0].getText());
          expectedWarnings.add(comments[0]);

          if (actualComment !== expectedComment) {
            console.log(node.getSourceFile().getFilePath());
            console.log(generateCodeframe(node, message));

            // Display the diff using jest-diff
            const diffOutput = diffStringsUnified(
              expectedComment,
              actualComment,
            );

            throw new Error(`Incorrect error message:\n${diffOutput}`);
          }
        },
        incrementChanges: () => {},
        printResults: () => {
          throw new Error("Not implemented");
        },
      };

      const tsConfigFilePath = await findTsconfig(context, folderPath);
      if (!tsConfigFilePath) {
        throw new Error("Can’t find tsconfig.json");
      }

      const project = new Project({
        tsConfigFilePath,
      });
      const allFiles = project.getSourceFiles();

      // Delete .after.ts files from the project, rename .before.ts to .ts
      const modifiedFiles = [];

      for (const file of allFiles) {
        if (file.getFilePath().match(/\.after\.[a-z]+$/)) {
          file.delete();
        } else if (file.getFilePath().match(/\.before\.[a-z]+$/)) {
          const newName = file
            .getFilePath()
            .replace(/\.before\.([a-z]+)$/, ".$1");
          file.move(newName);
          modifiedFiles.push(newName);
        }
      }

      beforeAll(async () => {
        // Run the codemod
        await explicitIds(context, project, folderPath);
      });

      for (const fileName of modifiedFiles) {
        const beforeFile = fileName.replace(/\.([a-z]+)$/, ".before.$1");
        const afterFile = fileName.replace(/\.([a-z]+)$/, ".after.$1");

        // eslint-disable-next-line jest/valid-title
        test(relative(folderPath, fileName), async () => {
          const expectedContent = fs.readFileSync(afterFile, "utf-8");
          const sourceFile = project.getSourceFileOrThrow(fileName);
          const actualContent = sourceFile.getFullText();

          expect(
            actualContent,
            `The file ${relative(folderPath, beforeFile)} should match ${relative(folderPath, afterFile)} after the codemod is ran`,
          ).toBe(expectedContent);

          // Verify that there are no warnings we should throw but we didn't
          for (const comment of getWarningComments(sourceFile)) {
            expect(
              expectedWarnings,
              `expected the following warning message to be thrown: “${stripCommentDelimiters(comment.getText())}”`,
            ).toContain(comment);
          }
        });
      }
    });
  }
});

function* getWarningComments(sourceFile: SourceFile): Generator<CommentRange> {
  for (const node of sourceFile.getDescendants()) {
    const commentRanges = node.getTrailingCommentRanges();

    for (const commentRange of commentRanges) {
      const commentText = commentRange.getText();
      const strippedComment = stripCommentDelimiters(commentText);

      if (strippedComment.startsWith(WARNING_COMMENT_PREFIX)) {
        yield commentRange;
      }
    }
  }
}

function stripCommentDelimiters(comment: string): string {
  if (comment.startsWith("//")) {
    return comment.slice(2).trim();
  }
  if (comment.startsWith("/*") && comment.endsWith("*/")) {
    return comment.slice(2, -2).trim();
  }
  return comment.trim();
}
