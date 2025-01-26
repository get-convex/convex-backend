// @ts-nocheck
// TODO: CX-4916 fix the types in this file.
import { StoryObj } from "@storybook/react";
import { Id } from "system-udfs/convex/_generated/dataModel";
import { DeploymentEventContent } from "./DeploymentEventContent";

export default { component: DeploymentEventContent };

export const CreateEnvironmentVariable: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      // @ts-expect-error
      action: "create_environment_variable",
      metadata: {
        // @ts-expect-error
        variable_name: "envVar",
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const DeleteEnvironmentVariable: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      // @ts-expect-error
      action: "delete_environment_variable",
      metadata: {
        // @ts-expect-error
        variable_name: "envVar",
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const UpdateEnvironmentVariable: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      // @ts-expect-error
      action: "update_environment_variable",
      metadata: {
        // @ts-expect-error
        variable_name: "envVar",
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const ReplaceEnvironmentVariable: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      // @ts-expect-error
      action: "replace_environment_variable",
      metadata: {
        // @ts-expect-error
        previous_variable_name: "envVar",
        variable_name: "envVar2",
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const UpdateIndexes: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      // @ts-expect-error
      action: "build_indexes",
      metadata: {
        // @ts-expect-error
        added_indexes: [
          { name: "my_index", type: "database", fields: ["field1", "field2"] },
          {
            name: "my_search_index",
            type: "search",
            searchField: "field",
            filterFields: ["field1", "field2"],
          },
        ],
        removed_indexes: [
          { name: "my_index", type: "database", fields: ["field1", "field2"] },
          {
            name: "my_search_index",
            type: "search",
            searchField: "field",
            filterFields: ["field1", "field2"],
          },
        ],
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PauseDeployment: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "change_deployment_state",
      metadata: {
        old_state: "running",
        new_state: "paused",
      },
      memberName: "member@convex.dev",
      member_id: 1,
    },
  },
};

export const ResumeDeployment: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "change_deployment_state",
      metadata: {
        old_state: "paused",
        new_state: "running",
      },
      memberName: "member@convex.dev",
      member_id: 1,
    },
  },
};

export const PushConfig: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: null,
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: null,
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithVersionChange: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: { previous_version: "0.5.0", next_version: "0.60" },
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: null,
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithAuthChange: StoryObj<typeof DeploymentEventContent> =
  {
    args: {
      event: {
        _id: "" as Id<"_deployment_audit_log">,
        _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
        action: "push_config",
        metadata: {
          auth: { added: ["auth1", "auth2"], removed: ["auth3", "auth4"] },
          server_version: null,
          modules: { added: [], removed: [] },
          crons: { added: [], updated: [], deleted: [] },
          schema: null,
        },
        memberName: "member@convex.dev",
        // @ts-expect-error
        member_id: 1,
      },
    },
  };

export const PushConfigWithChange: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: null,
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: {
          previous_schema_id: "" as Id<"_schemas">,
          previous_schema: {
            _creationTime: 1684788174774.141,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                      isRemoved: {
                        fieldType: {
                          type: "object",
                          value: {
                            booleanOrNumber: {
                              fieldType: {
                                type: "union",
                                value: [
                                  { type: "boolean" },
                                  { type: "number" },
                                ],
                              },
                              optional: false,
                            },
                            nestedObject: {
                              fieldType: {
                                type: "object",
                                value: {
                                  property: {
                                    fieldType: { type: "string" },
                                    optional: false,
                                  },
                                },
                              },
                              optional: false,
                            },
                            numbers: {
                              fieldType: {
                                type: "set",
                                value: { type: "number" },
                              },
                              optional: false,
                            },
                            string: {
                              fieldType: { type: "string" },
                              optional: true,
                            },
                          },
                        },
                        optional: false,
                      },
                    },
                  },
                },
              ],
              schemaValidation: true,
            }),
            state: {
              state: "overwritten",
            },
          },
          next_schema_id: "" as Id<"_schemas">,
          next_schema: {
            _creationTime: 1684788230173.204,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                      isRemoved: {
                        fieldType: {
                          type: "object",
                          value: {
                            booleanOrNumber: {
                              fieldType: {
                                type: "union",
                                value: [
                                  { type: "boolean" },
                                  { type: "number" },
                                ],
                              },
                              optional: false,
                            },
                            nestedObject: {
                              fieldType: {
                                type: "object",
                                value: {
                                  property: {
                                    fieldType: {
                                      type: "array",
                                      value: {
                                        type: "union",
                                        value: [
                                          { type: "string" },
                                          { type: "bigint" },
                                          { type: "null" },
                                        ],
                                      },
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              optional: false,
                            },
                            numbers: {
                              fieldType: {
                                type: "set",
                                value: { type: "number" },
                              },
                              optional: false,
                            },
                            string: {
                              fieldType: { type: "string" },
                              optional: true,
                            },
                          },
                        },
                        optional: false,
                      },
                    },
                  },
                },
              ],
              schemaValidation: true,
            }),
            state: {
              state: "active",
            },
          },
        },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithAddition: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: null,
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: {
          previous_schema_id: null,
          previous_schema: null,
          next_schema_id: "" as Id<"_schemas">,
          next_schema: {
            _creationTime: 1684788230173.204,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                      isRemoved: {
                        fieldType: {
                          type: "object",
                          value: {
                            booleanOrNumber: {
                              fieldType: {
                                type: "union",
                                value: [
                                  { type: "boolean" },
                                  { type: "number" },
                                ],
                              },
                              optional: false,
                            },
                            nestedObject: {
                              fieldType: {
                                type: "object",
                                value: {
                                  property: {
                                    fieldType: {
                                      type: "array",
                                      value: {
                                        type: "union",
                                        value: [
                                          { type: "string" },
                                          { type: "bigint" },
                                          { type: "null" },
                                        ],
                                      },
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              optional: false,
                            },
                            numbers: {
                              fieldType: {
                                type: "set",
                                value: { type: "number" },
                              },
                              optional: false,
                            },
                            string: {
                              fieldType: { type: "string" },
                              optional: true,
                            },
                          },
                        },
                        optional: false,
                      },
                    },
                  },
                },
              ],
              schemaValidation: true,
            }),
            state: {
              state: "active",
            },
          },
        },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithDeletion: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: null,
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: {
          previous_schema_id: "" as Id<"_schemas">,
          previous_schema: {
            _creationTime: 1684788174774.141,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                      isRemoved: {
                        fieldType: {
                          type: "object",
                          value: {
                            booleanOrNumber: {
                              fieldType: {
                                type: "union",
                                value: [
                                  { type: "boolean" },
                                  { type: "number" },
                                ],
                              },
                              optional: false,
                            },
                            nestedObject: {
                              fieldType: {
                                type: "object",
                                value: {
                                  property: {
                                    fieldType: { type: "string" },
                                    optional: false,
                                  },
                                },
                              },
                              optional: false,
                            },
                            numbers: {
                              fieldType: {
                                type: "set",
                                value: { type: "number" },
                              },
                              optional: false,
                            },
                            string: {
                              fieldType: { type: "string" },
                              optional: true,
                            },
                          },
                        },
                        optional: false,
                      },
                    },
                  },
                },
              ],
              schemaValidation: true,
            }),
            state: {
              state: "overwritten",
            },
          },
          next_schema_id: null,
          next_schema: null,
        },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithEnforcementChange: StoryObj<
  typeof DeploymentEventContent
> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "push_config",
      metadata: {
        auth: { added: [], removed: [] },
        server_version: null,
        modules: { added: [], removed: [] },
        crons: { added: [], updated: [], deleted: [] },
        schema: {
          previous_schema_id: "" as Id<"_schemas">,
          previous_schema: {
            _creationTime: 1684788174774.141,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                    },
                  },
                },
              ],
              schemaValidation: false,
            }),
            state: {
              state: "overwritten",
            },
          },
          next_schema_id: "" as Id<"_schemas">,
          next_schema: {
            _creationTime: 1684788230173.204,
            _id: "" as Id<"_schemas">,
            schema: JSON.stringify({
              tables: [
                {
                  tableName: "messages",
                  indexes: [],
                  searchIndexes: [],
                  documentType: {
                    type: "object",
                    value: {
                      author: {
                        fieldType: { type: "string" },
                        optional: false,
                      },
                      body: { fieldType: { type: "string" }, optional: false },
                    },
                  },
                },
              ],
              schemaValidation: true,
            }),
            state: {
              state: "active",
            },
          },
        },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const PushConfigWithLargeDiff: StoryObj<typeof DeploymentEventContent> =
  {
    args: {
      event: {
        _id: "" as Id<"_deployment_audit_log">,
        _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
        action: "push_config",
        metadata: {
          auth: {
            added: [],
            removed: [],
          },
          crons: {
            added: [],
            deleted: [],
            updated: [],
          },
          modules: {
            added: [],
            removed: [],
          },
          schema: {
            next_schema: {
              _id: "" as Id<"_schemas">,
              _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
              schema: JSON.stringify({
                tables: [
                  {
                    tableName: "games",
                    indexes: [
                      {
                        indexDescriptor: "s",
                        fields: ["slug", "_creationTime"],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        hostId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        nextGameId: {
                          fieldType: { type: "id", tableName: "games" },
                          optional: true,
                        },
                        playerIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "users" },
                          },
                          optional: false,
                        },
                        roundIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "rounds" },
                          },
                          optional: false,
                        },
                        slug: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        state: {
                          fieldType: {
                            type: "union",
                            value: [
                              {
                                type: "object",
                                value: {
                                  stage: {
                                    fieldType: {
                                      type: "union",
                                      value: [
                                        { type: "literal", value: "lobby" },
                                        { type: "literal", value: "generate" },
                                        { type: "literal", value: "recap" },
                                      ],
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              {
                                type: "object",
                                value: {
                                  roundId: {
                                    fieldType: {
                                      type: "id",
                                      tableName: "rounds",
                                    },
                                    optional: false,
                                  },
                                  stage: {
                                    fieldType: {
                                      type: "literal",
                                      value: "rounds",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                            ],
                          },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "publicGame",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        roundId: {
                          fieldType: { type: "id", tableName: "rounds" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "rounds",
                    indexes: [
                      {
                        indexDescriptor: "public_game",
                        fields: [
                          "publicRound",
                          "stage",
                          "lastUsed",
                          "_creationTime",
                        ],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        authorId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        imageStorageId: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        lastUsed: {
                          fieldType: { type: "number" },
                          optional: true,
                        },
                        publicRound: {
                          fieldType: { type: "boolean" },
                          optional: true,
                        },
                        stage: {
                          fieldType: {
                            type: "union",
                            value: [
                              { type: "literal", value: "label" },
                              { type: "literal", value: "guess" },
                              { type: "literal", value: "reveal" },
                            ],
                          },
                          optional: false,
                        },
                        stageEnd: {
                          fieldType: { type: "number" },
                          optional: false,
                        },
                        stageStart: {
                          fieldType: { type: "number" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "sessions",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        gameIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "games" },
                          },
                          optional: false,
                        },
                        submissionIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "submissions" },
                          },
                          optional: false,
                        },
                        userId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "submissions",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        authorId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        prompt: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        result: {
                          fieldType: {
                            type: "union",
                            value: [
                              {
                                type: "object",
                                value: {
                                  elapsedMs: {
                                    fieldType: { type: "number" },
                                    optional: false,
                                  },
                                  status: {
                                    fieldType: {
                                      type: "literal",
                                      value: "waiting",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              {
                                type: "object",
                                value: {
                                  details: {
                                    fieldType: { type: "string" },
                                    optional: false,
                                  },
                                  status: {
                                    fieldType: {
                                      type: "literal",
                                      value: "generating",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              {
                                type: "object",
                                value: {
                                  elapsedMs: {
                                    fieldType: { type: "number" },
                                    optional: false,
                                  },
                                  reason: {
                                    fieldType: { type: "string" },
                                    optional: false,
                                  },
                                  status: {
                                    fieldType: {
                                      type: "literal",
                                      value: "failed",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              {
                                type: "object",
                                value: {
                                  elapsedMs: {
                                    fieldType: { type: "number" },
                                    optional: false,
                                  },
                                  imageStorageId: {
                                    fieldType: { type: "string" },
                                    optional: false,
                                  },
                                  status: {
                                    fieldType: {
                                      type: "literal",
                                      value: "saved",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                            ],
                          },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "users",
                    indexes: [
                      {
                        indexDescriptor: "by_token",
                        fields: ["tokenIdentifier", "_creationTime"],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        claimedByUserId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: true,
                        },
                        name: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        password_hash: {
                          fieldType: { type: "string" },
                          optional: true,
                        },
                        pictureUrl: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        tokenIdentifier: {
                          fieldType: { type: "string" },
                          optional: true,
                        },
                      },
                    },
                  },
                ],
                schemaValidation: true,
              }),
              state: {
                state: "overwritten",
              },
            },
            next_schema_id: "" as Id<"_schemas">,
            previous_schema: {
              _id: "" as Id<"_schemas">,
              _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
              schema: JSON.stringify({
                tables: [
                  {
                    tableName: "games",
                    indexes: [
                      {
                        indexDescriptor: "s",
                        fields: ["slug", "_creationTime"],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        hostId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        nextGameId: {
                          fieldType: { type: "id", tableName: "games" },
                          optional: true,
                        },
                        playerIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "users" },
                          },
                          optional: false,
                        },
                        roundIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "rounds" },
                          },
                          optional: false,
                        },
                        slug: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        state: {
                          fieldType: {
                            type: "union",
                            value: [
                              {
                                type: "object",
                                value: {
                                  stage: {
                                    fieldType: {
                                      type: "union",
                                      value: [
                                        { type: "literal", value: "lobby" },
                                        { type: "literal", value: "generate" },
                                        { type: "literal", value: "recap" },
                                      ],
                                    },
                                    optional: false,
                                  },
                                },
                              },
                              {
                                type: "object",
                                value: {
                                  roundId: {
                                    fieldType: {
                                      type: "id",
                                      tableName: "rounds",
                                    },
                                    optional: false,
                                  },
                                  stage: {
                                    fieldType: {
                                      type: "literal",
                                      value: "rounds",
                                    },
                                    optional: false,
                                  },
                                },
                              },
                            ],
                          },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "publicGame",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        roundId: {
                          fieldType: { type: "id", tableName: "rounds" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "rounds",
                    indexes: [
                      {
                        indexDescriptor: "public_game",
                        fields: [
                          "publicRound",
                          "stage",
                          "lastUsed",
                          "_creationTime",
                        ],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        authorId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        imageStorageId: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        lastUsed: {
                          fieldType: { type: "number" },
                          optional: true,
                        },
                        options: {
                          fieldType: {
                            type: "array",
                            value: {
                              type: "object",
                              value: {
                                authorId: {
                                  fieldType: { type: "id", tableName: "users" },
                                  optional: false,
                                },
                                likes: {
                                  fieldType: {
                                    type: "array",
                                    value: { type: "id", tableName: "users" },
                                  },
                                  optional: false,
                                },
                                prompt: {
                                  fieldType: { type: "string" },
                                  optional: false,
                                },
                                votes: {
                                  fieldType: {
                                    type: "array",
                                    value: { type: "id", tableName: "users" },
                                  },
                                  optional: false,
                                },
                              },
                            },
                          },
                          optional: false,
                        },
                        publicRound: {
                          fieldType: { type: "boolean" },
                          optional: true,
                        },
                        stage: {
                          fieldType: {
                            type: "union",
                            value: [
                              { type: "literal", value: "label" },
                              { type: "literal", value: "guess" },
                              { type: "literal", value: "reveal" },
                            ],
                          },
                          optional: false,
                        },
                        stageEnd: {
                          fieldType: { type: "number" },
                          optional: false,
                        },
                        stageStart: {
                          fieldType: { type: "number" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "sessions",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        gameIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "games" },
                          },
                          optional: false,
                        },
                        submissionIds: {
                          fieldType: {
                            type: "array",
                            value: { type: "id", tableName: "submissions" },
                          },
                          optional: false,
                        },
                        userId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "submissions",
                    indexes: [],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        authorId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: false,
                        },
                        prompt: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                      },
                    },
                  },
                  {
                    tableName: "users",
                    indexes: [
                      {
                        indexDescriptor: "by_token",
                        fields: ["tokenIdentifier", "_creationTime"],
                      },
                    ],
                    searchIndexes: [],
                    documentType: {
                      type: "object",
                      value: {
                        claimedByUserId: {
                          fieldType: { type: "id", tableName: "users" },
                          optional: true,
                        },
                        name: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        password_hash: {
                          fieldType: { type: "string" },
                          optional: true,
                        },
                        pictureUrl: {
                          fieldType: { type: "string" },
                          optional: false,
                        },
                        tokenIdentifier: {
                          fieldType: { type: "string" },
                          optional: true,
                        },
                      },
                    },
                  },
                ],
                schemaValidation: true,
              }),
              state: {
                state: "overwritten",
              },
            },
            previous_schema_id: "" as Id<"_schemas">,
          },
          server_version: null,
        },
        memberName: "member@convex.dev",
        // @ts-expect-error
        member_id: 1,
      },
    },
  };

export const SnapshotImportZip: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "snapshot_import",
      metadata: {
        table_names: ["_storage", "users", "friendships"],
        table_count: 4,
        import_mode: "Replace",
        import_format: { format: "zip" },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};

export const SnapshotImportCsv: StoryObj<typeof DeploymentEventContent> = {
  args: {
    event: {
      _id: "" as Id<"_deployment_audit_log">,
      _creationTime: Date.parse("12/19/2022, 10:00:00 AM"),
      action: "snapshot_import",
      metadata: {
        table_names: ["users"],
        table_count: 1,
        import_mode: "RequireEmpty",
        import_format: { format: "csv" },
      },
      memberName: "member@convex.dev",
      // @ts-expect-error
      member_id: 1,
    },
  },
};
