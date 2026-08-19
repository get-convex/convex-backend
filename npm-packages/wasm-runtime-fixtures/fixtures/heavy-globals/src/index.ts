import { chunk } from "lodash-es";
import MarkdownIt from "markdown-it";
import { z } from "zod";

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
});

const productSchema = z.object({
  id: z.string(),
  name: z.string(),
  priceCents: z.number().int().nonnegative(),
  tags: z.array(z.string()),
});

const productGrid = chunk(
  Array.from({ length: 240 }, (_, index) => ({
    id: `product-${index}`,
    name: `Product ${index}`,
    priceCents: index * 17,
    tags: index % 2 === 0 ? ["featured", "sale"] : ["standard"],
  })),
  12,
);

const renderedCatalog = markdown.render(
  productGrid
    .flat()
    .slice(0, 12)
    .map((product) => `- **${product.name}**: ${product.priceCents}`)
    .join("\n"),
);

export function summarizeCatalog() {
  const parsed = productSchema.parse(productGrid[0][0]);

  return {
    chunks: productGrid.length,
    renderedCatalogLength: renderedCatalog.length,
    firstProduct: parsed.name,
  };
}

export function renderProduct(name: string): string {
  return markdown.render(`# ${name}\n\nLoaded from the heavy globals fixture.`);
}
