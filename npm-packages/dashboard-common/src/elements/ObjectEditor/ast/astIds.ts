import { isId } from "id-encoding";
import { Node, LiteralNode } from "./types";

export class IdWalker {
  private ids: LiteralNode[] = [];

  walk(n: Node): LiteralNode[] {
    switch (n.type) {
      case "ArrayExpression": {
        n.elements.forEach((element) => {
          if (element !== null) {
            this.walk(element);
          }
        });
        break;
      }

      case "ObjectExpression": {
        n.properties.forEach((property) => {
          this.walk(property.key);
          this.walk(property.value);
        });
        break;
      }

      case "UnaryExpression": {
        break;
      }

      case "NewExpression": {
        break;
      }

      case "TemplateLiteral": {
        n.expressions?.forEach((expr) => this.walk(expr));
        break;
      }

      case "Literal": {
        if (typeof n.value === "string" && isId(n.value)) {
          this.ids.push(n);
        }
        break;
      }

      case "Identifier": {
        // Identifiers are not string literals, so we skip them.
        break;
      }

      case "CallExpression": {
        break;
      }

      default: {
        // Unsupported syntax, do nothing.
        break;
      }
    }

    return this.ids;
  }
}
