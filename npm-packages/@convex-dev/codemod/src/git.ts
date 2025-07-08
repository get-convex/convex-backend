import { execa } from "execa";

export async function hasUncommittedChanges(folder: string) {
  try {
    const result = await execa("git", ["status", "--porcelain"], {
      cwd: folder,
    });
    return result.stdout.length > 0;
  } catch (error) {
    return false;
  }
}
