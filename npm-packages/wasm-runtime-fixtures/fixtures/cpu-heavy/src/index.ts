export function burn(work: number) {
  let acc = 0;
  let x = 0x12345678;

  for (let i = 0; i < work; i += 1) {
    x = (x * 1664525 + 1013904223) >>> 0;
    acc = (acc + ((x ^ i) & 0xffff)) >>> 0;
  }

  return {
    work,
    checksum: acc,
    finalState: x,
  };
}
