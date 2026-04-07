import { shapeSchema, stringifyShape } from "shapes";
import { query } from "./_generated/server";

export default query((_, { shapeJson }: { shapeJson: string }) => {
  const shape = shapeSchema.parse(JSON.parse(shapeJson));
  return stringifyShape(shape);
});
