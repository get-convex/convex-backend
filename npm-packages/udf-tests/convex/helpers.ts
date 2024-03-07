export function fibonacci(n: number): number {
  let a = 0;
  let b = 1;
  for (let i = 0; i < n; i++) {
    const c = a + b;
    a = b;
    b = c;
  }
  return a;
}

export function doesntWork() {
  throw new Error("Doesn't has an apostrophe.");
}
