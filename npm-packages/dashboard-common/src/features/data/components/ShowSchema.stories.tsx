import { Meta, StoryObj } from "@storybook/nextjs";
import { Shape } from "shapes";
import { ShowSchema } from "@common/features/data/components/ShowSchema";
import { SchemaJson } from "@common/lib/format";

export default {
  component: ShowSchema,
  args: {
    activeSchema: undefined,
    inProgressSchema: undefined,
    shapes: new Map<string, Shape>([
      [
        "tasks",
        {
          type: "Object",
          fields: [
            {
              fieldName: "status",
              optional: false,
              shape: { type: "String" },
            },
          ],
        },
      ],
    ]),
  },
} as Meta<typeof ShowSchema>;

const sampleSchema: SchemaJson = {
  tables: [
    {
      tableName: "tasks",
      indexes: [],
      searchIndexes: [],
      vectorIndexes: [],
      documentType: {
        type: "object",
        value: {
          status: {
            fieldType: {
              type: "union",
              value: [
                { type: "literal", value: "todo" },
                { type: "literal", value: "in-progress" },
                { type: "literal", value: "done" },
              ],
            },
            optional: false,
          },
        },
      },
    },
  ],
  schemaValidation: true,
};

export const NoSchema: StoryObj<typeof ShowSchema> = { args: {} };

export const GenerationError: StoryObj<typeof ShowSchema> = {
  args: {
    hasShapeError: true,
  },
};

export const LoadingSchema: StoryObj<typeof ShowSchema> = {
  args: {
    inProgressSchema: sampleSchema,
  },
};

export const LoadingSchemaWithExistingSchema: StoryObj<typeof ShowSchema> = {
  args: {
    activeSchema: sampleSchema,
    inProgressSchema: sampleSchema,
  },
};

export const SavedSchema: StoryObj<typeof ShowSchema> = {
  args: {
    activeSchema: {
      ...sampleSchema,
      schemaValidation: false,
    },
  },
};

export const EnforcedSchema: StoryObj<typeof ShowSchema> = {
  args: {
    activeSchema: sampleSchema,
  },
};
