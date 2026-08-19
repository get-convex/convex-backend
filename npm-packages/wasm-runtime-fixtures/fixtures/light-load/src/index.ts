import { console } from "convex:runtime";

export function greet(name: string): string {
  console.log("greet", name);
  return `Hello, ${name}!`;
}

export function sum(values: number[]): number {
  return values.reduce((total, value) => total + value, 0);
}
