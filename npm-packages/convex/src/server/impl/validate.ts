export function validateArg(
  arg: any,
  idx: number,
  method: string,
  argName: string,
) {
  if (arg === undefined) {
    throw new Error(`Must provide arg ${idx} \`${argName}\` to \`${method}\``);
  }
}
