export function extractVersionFromHeader(header: string | null): string | null {
  if (!header) {
    return null;
  }

  const match = header.match(/npm-cli-(.+)/);
  return match ? match[1] : null;
}
