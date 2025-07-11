export function isStale(document: { _creationTime: number }) {
  const now = Date.now();
  const oneHour = 60 * 60 * 1000;
  return now - document._creationTime > oneHour;
}
