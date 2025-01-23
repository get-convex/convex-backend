// This is where React components go.
if (typeof window === "undefined") {
  throw new Error("this is frontend code, but it's running somewhere else!");
}

export function subtract(a: number, b: number): number {
  return a - b;
}
