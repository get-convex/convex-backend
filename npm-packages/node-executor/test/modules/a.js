export function fibonacci(n) {
  if (n < 2) {
    return 1;
  }
  return fibonacci(n - 2) + fibonacci(n - 1);
}
