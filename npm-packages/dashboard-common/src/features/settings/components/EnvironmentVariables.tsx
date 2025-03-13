import { Form, Formik, getIn, useFormikContext } from "formik";

import {
  ClipboardCopyIcon,
  Cross2Icon,
  EyeNoneIcon,
  EyeOpenIcon,
  Pencil1Icon,
  PlusIcon,
  TrashIcon,
} from "@radix-ui/react-icons";

import classNames from "classnames";
import {
  ClipboardEventHandler,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import { z } from "zod";
import { Spinner } from "@common/elements/Spinner";
import { Callout } from "@common/elements/Callout";
import { Button } from "@common/elements/Button";
import { copyTextToClipboard, toast } from "@common/lib/utils";
import { TextInput } from "@common/elements/TextInput";

const MAX_NUMBER_OF_ENV_VARS = 100;

export const ENVIRONMENT_VARIABLES_ROW_CLASSES =
  "grid grid-cols-[minmax(0,1fr)_7.5rem] gap-4 py-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_7.5rem] items-start";
export const ENVIRONMENT_VARIABLE_NAME_COLUMN = "col-span-2 md:col-span-1";

const ERROR_ENV_VAR_NOT_UNIQUE = "Environment variable name is not unique";

export type BaseEnvironmentVariable = { name: string; value: string };

type FormState<T extends BaseEnvironmentVariable> = {
  editedVars: {
    oldEnvVar: T;
    newEnvVar: BaseEnvironmentVariable;
  }[];
  newVars: BaseEnvironmentVariable[];
  deletedVars: T[];
  tooManyEnvVars: boolean;
};

// This is used for showing both deployment environment variables and project level environment variables
export function EnvironmentVariables<T extends BaseEnvironmentVariable>({
  environmentVariables,
  updateEnvironmentVariables,
  initialFormValues,
  hasAdminPermissions,
}: {
  environmentVariables: Array<T> | undefined;
  initialFormValues?: Array<BaseEnvironmentVariable>;
  updateEnvironmentVariables: (
    creations: BaseEnvironmentVariable[],
    modifications: { oldEnvVar: T; newEnvVar: BaseEnvironmentVariable }[],
    deletions: T[],
  ) => Promise<void>;
  hasAdminPermissions: boolean;
}) {
  return (
    <Formik
      enableReinitialize
      initialValues={
        {
          editedVars: [],
          newVars: initialFormValues ?? [],
          deletedVars: [],
          tooManyEnvVars: false,
        } as FormState<T>
      }
      onSubmit={async (values, helpers) => {
        await updateEnvironmentVariables(
          values.newVars,
          values.editedVars,
          values.deletedVars,
        );

        helpers.resetForm({});
      }}
      validate={(values) => {
        const errors: Record<string, string> = {};

        // Names / values validation
        const uneditedVarNames =
          environmentVariables
            ?.filter(
              (v) =>
                !values.editedVars.some((edited) => edited.oldEnvVar === v) &&
                !values.deletedVars.some((deleted) => deleted === v),
            )
            .map((sourceVar) => sourceVar.name) ?? [];
        const editedVarNames = values.editedVars.map(
          (editedVar) => editedVar.newEnvVar.name,
        );
        const newVarNames = values.newVars.map((newVar) => newVar.name);
        const nameOccurrences = [
          ...uneditedVarNames,
          ...editedVarNames,
          ...newVarNames,
        ].reduce(
          (acc, name) => acc.set(name, (acc.get(name) ?? 0) + 1),
          new Map(),
        );
        const variablesToValidate: {
          value: BaseEnvironmentVariable;
          key: string;
        }[] = [
          ...values.editedVars.map((editedVar, index) => ({
            value: editedVar.newEnvVar,
            key: `editedVars[${index}].newEnvVar`,
          })),
          ...values.newVars.map((value, index) => ({
            value,
            key: `newVars[${index}]`,
          })),
        ];
        variablesToValidate.forEach(({ value, key }) => {
          try {
            EnvVarName.parse(value.name);
          } catch (err) {
            if (err instanceof z.ZodError) {
              errors[`${key}.name`] = err.issues[0].message;
            }
          }

          try {
            EnvVarValue.parse(value.value);
          } catch (err) {
            if (err instanceof z.ZodError) {
              errors[`${key}.value`] = err.issues[0].message;
            }
          }

          if (nameOccurrences.get(value.name) > 1) {
            errors[`${key}.name`] = ERROR_ENV_VAR_NOT_UNIQUE;
          }
        });

        return errors;
      }}
    >
      <EnvironmentVariablesForm
        environmentVariables={environmentVariables}
        hasAdminPermissions={hasAdminPermissions}
      />
    </Formik>
  );
}

function EnvironmentVariablesForm<T extends BaseEnvironmentVariable>({
  environmentVariables,
  hasAdminPermissions,
}: {
  environmentVariables: Array<T> | undefined;
  hasAdminPermissions: boolean;
}) {
  const formState = useFormikContext<FormState<T>>();

  // Remove elements from editedVars/deletedVars that refer to variables that
  // don’t exist anymore. This can be caused by a realtime update
  // of `environmentVariables` (see CX-5439).
  const prevEnvironmentVariables = useRef<Array<T>>();
  useEffect(() => {
    if (
      !environmentVariables ||
      prevEnvironmentVariables.current === environmentVariables
    )
      return;
    prevEnvironmentVariables.current = environmentVariables;

    const oldEditedVars = formState.values.editedVars;
    const oldDeletedVars = formState.values.deletedVars;
    if (oldEditedVars.length === 0 && oldDeletedVars.length === 0) return;

    const existingEnvironmentVariables = new Set(
      environmentVariables.map(({ name }) => name),
    );

    const newEditedVars = oldEditedVars.filter((v) =>
      existingEnvironmentVariables.has(v.oldEnvVar.name),
    );
    if (newEditedVars.length !== oldEditedVars.length) {
      void formState.setFieldValue("editedVars", newEditedVars);
    }

    const newDeletedVars = oldDeletedVars.filter((v) =>
      existingEnvironmentVariables.has(v.name),
    );
    if (newDeletedVars.length !== oldDeletedVars.length) {
      void formState.setFieldValue("deletedVars", newDeletedVars);
    }
  }, [environmentVariables, formState]);

  return (
    <Form className="flex flex-col">
      {environmentVariables === undefined ? (
        <Spinner />
      ) : (
        <>
          <div className="divide-y divide-border-transparent">
            {(environmentVariables.length > 0 ||
              formState.values.newVars.length > 0) && (
              <div
                className={classNames(
                  ENVIRONMENT_VARIABLES_ROW_CLASSES,
                  "hidden md:grid",
                )}
              >
                <div
                  className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
                >
                  <span className="text-xs text-content-secondary">Name</span>
                </div>
                <div className="flex flex-col gap-1">
                  <span className="text-xs text-content-secondary">Value</span>
                </div>
              </div>
            )}
            {environmentVariables?.map((value) => (
              <EnvironmentVariableListItem
                key={value.name}
                environmentVariable={value}
                hasAdminPermissions={hasAdminPermissions}
              />
            ))}
          </div>

          <NewEnvVars
            existingEnvVars={environmentVariables}
            hasAdminPermissions={hasAdminPermissions}
          />

          {environmentVariables.length >= MAX_NUMBER_OF_ENV_VARS && (
            <div>
              <Callout variant="error">
                You've reached the environment variable limit (
                {MAX_NUMBER_OF_ENV_VARS}). Contact support@convex.dev if you
                need more.
              </Callout>
            </div>
          )}
        </>
      )}
    </Form>
  );
}

// Adapted from https://github.com/motdotla/dotenv/blob/cf4c56957974efb7238ecaba6f16e0afa895c194/lib/main.js#L12
const LINE =
  /(?:^|^)\s*(?:export\s+)?([\w.-]+)(?:\s*=\s*?|:\s+?)(\s*'(?:\\'|[^'])*'|\s*"(?:\\"|[^"])*"|\s*`(?:\\`|[^`])*`|[^#\r\n]+)?\s*(?:#.*)?(?:$|$)/gm;
function _parseEnvVars(input: string): Record<string, string> {
  const obj: Record<string, string> = {};

  // Convert line breaks to same format
  const lines = input.replace(/\r\n?/gm, "\n");

  let match = LINE.exec(lines);
  while (match !== null) {
    const key = match[1];

    // Default undefined or null to empty string
    let value = match[2] || "";

    // Remove whitespace
    value = value.trim();

    // Check if double quoted
    const maybeQuote = value[0];

    // Remove surrounding quotes
    value = value.replace(/^(['"`])([\s\S]*)\1$/gm, "$2");

    // Expand newlines if double quoted
    if (maybeQuote === '"') {
      value = value.replace(/\\n/g, "\n");
      value = value.replace(/\\r/g, "\r");
    }

    // Add to object
    obj[key] = value;
    match = LINE.exec(lines);
  }

  return obj;
}

export function parseEnvVars(
  input: string,
): Array<BaseEnvironmentVariable> | null {
  const parsedEnvVars = _parseEnvVars(input);
  // Ignore if the pasted string does not match an environment variable string
  if (Object.keys(parsedEnvVars).length === 0) {
    return null;
  }
  return Object.entries(parsedEnvVars).map(([name, value]) => ({
    name,
    value,
  }));
}

function DisplayEnvVar<T extends BaseEnvironmentVariable>({
  environmentVariable,
  onEdit,
  onDelete,
  hasAdminPermissions,
}: {
  environmentVariable: T;
  onEdit: () => void;
  onDelete: () => void;
  hasAdminPermissions: boolean;
}) {
  const formState = useFormikContext<FormState<T>>();
  const [showValue, setShowValue] = useState(false);

  return (
    <div className={ENVIRONMENT_VARIABLES_ROW_CLASSES}>
      <div
        className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
      >
        <div className="flex h-[2.375rem] items-center truncate text-content-primary md:col-span-1">
          {environmentVariable.name}
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <div className="flex h-[2.375rem] items-center gap-1 font-mono">
          <Button
            tip={showValue ? "Hide" : "Show"}
            type="button"
            onClick={() => setShowValue(!showValue)}
            variant="neutral"
            inline
            size="sm"
            icon={showValue ? <EyeNoneIcon /> : <EyeOpenIcon />}
          />
          {showValue ? (
            <span className=" truncate text-content-primary">
              {environmentVariable.value}
            </span>
          ) : (
            <span
              title="Hidden environment variable"
              className="text-content-primary"
            >
              ••••••
            </span>
          )}
        </div>
      </div>
      <div className="flex gap-2">
        <Button
          tip={
            !hasAdminPermissions
              ? "You do not have permission to edit environment variables."
              : "Edit"
          }
          type="button"
          onClick={() => onEdit()}
          variant="neutral"
          size="sm"
          inline
          icon={<Pencil1Icon />}
          disabled={formState.isSubmitting || !hasAdminPermissions}
        />
        <Button
          tip="Copy Value"
          type="button"
          onClick={async () => {
            await copyTextToClipboard(environmentVariable.value);
            toast(
              "success",
              "Environment variable value copied to the clipboard.",
            );
          }}
          variant="neutral"
          size="sm"
          inline
          icon={<ClipboardCopyIcon />}
          disabled={formState.isSubmitting}
        />
        <Button
          tip={
            !hasAdminPermissions
              ? "You do not have permission to delete environment variables."
              : "Delete"
          }
          type="button"
          onClick={() => onDelete()}
          variant="danger"
          size="sm"
          inline
          icon={<TrashIcon />}
          disabled={formState.isSubmitting || !hasAdminPermissions}
        />
      </div>
    </div>
  );
}

function DeletedEnvVar<T extends BaseEnvironmentVariable>({
  environmentVariable,
  onCancelDelete,
}: {
  environmentVariable: T;
  onCancelDelete: () => void;
}) {
  const formState = useFormikContext<FormState<T>>();

  return (
    <div className={ENVIRONMENT_VARIABLES_ROW_CLASSES}>
      <div
        className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
      >
        <div className="flex h-[2.375rem] items-center truncate text-content-primary md:col-span-1">
          {environmentVariable.name}
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <div className="flex h-[2.375rem] items-center justify-center gap-1 bg-background-error">
          <TrashIcon className="text-content-error" /> Will be deleted
        </div>
      </div>
      <div className="flex items-center justify-center">
        <Button
          inline
          size="sm"
          onClick={() => onCancelDelete()}
          disabled={formState.isSubmitting}
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}

const EnvVarName = z
  .string()
  .min(1, "Environment variable name is required.")
  .max(40, "Environment variable name cannot exceed 40 characters.")
  .refine(
    (n) => /^[a-zA-Z_]+[a-zA-Z0-9_]*$/.test(n),
    "Name must start with a letter and only contain letters, numbers, and underscores.",
  );

const EnvVarValue = z
  .string()
  .min(1, "Environment variable value is required.")
  .max(8192, "Environment variable value cannot be larger than 8KB");

function EditEnvVarForm<T extends BaseEnvironmentVariable>({
  editIndex,
  onCancelEdit,
}: {
  editIndex: number;
  onCancelEdit: () => void;
}) {
  const nameId = useId();
  const valueId = useId();

  const formState = useFormikContext<FormState<T>>();
  const { value } = (formState.values as any).editedVars[editIndex].newEnvVar;

  return (
    <div>
      <div className={ENVIRONMENT_VARIABLES_ROW_CLASSES}>
        <label
          htmlFor={nameId}
          className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
        >
          <ValidatedTextInput
            formKey={`editedVars[${editIndex}].newEnvVar.name`}
            id={nameId}
          />
        </label>
        <label htmlFor={valueId} className="flex grow flex-col flex-wrap gap-1">
          <ValidatedTextInput
            formKey={`editedVars[${editIndex}].newEnvVar.value`}
            id={valueId}
            noAutocomplete
          />
        </label>
        <div className="flex items-center justify-center">
          <Button
            type="button"
            inline
            size="sm"
            onClick={() => onCancelEdit()}
            disabled={formState.isSubmitting}
          >
            Cancel
          </Button>
        </div>
      </div>
      {value.length > 1 && value.startsWith('"') && value.endsWith('"') && (
        <Callout className="mb-2 w-full">
          Environment variables usually shouldn't be surrounded by quotes.
          Quotes are useful in shell syntax and .env files but shouldn't be
          included in the environment variable value.
        </Callout>
      )}
    </div>
  );
}

function EnvironmentVariableListItem<
  T extends { name: string; value: string },
>({
  environmentVariable,
  hasAdminPermissions,
}: {
  environmentVariable: T;
  hasAdminPermissions: boolean;
}) {
  const formState = useFormikContext<FormState<T>>();

  const editIndex = formState.values.editedVars.findIndex(
    (edit) => edit.oldEnvVar === environmentVariable,
  );
  const isEditing = editIndex !== -1;
  if (isEditing) {
    return (
      <EditEnvVarForm
        editIndex={editIndex}
        onCancelEdit={() => {
          const newEditedVars = [
            ...formState.values.editedVars.slice(0, editIndex),
            ...formState.values.editedVars.slice(editIndex + 1),
          ];
          void formState.setFieldValue("editedVars", newEditedVars);

          // https://github.com/jaredpalmer/formik/issues/2059#issuecomment-612733378
          setTimeout(() =>
            formState.setTouched({
              editedVars: newEditedVars.map(() => ({
                newEnvVar: { name: true, value: true },
              })),
            }),
          );
        }}
      />
    );
  }

  const deleteIndex = formState.values.deletedVars.findIndex(
    (deletedVar) => deletedVar === environmentVariable,
  );
  const isDeleting = deleteIndex !== -1;
  if (isDeleting) {
    return (
      <DeletedEnvVar
        environmentVariable={environmentVariable}
        onCancelDelete={() => {
          const newDeletedVars = [
            ...formState.values.deletedVars.slice(0, deleteIndex),
            ...formState.values.deletedVars.slice(deleteIndex + 1),
          ];
          void formState.setFieldValue("deletedVars", newDeletedVars);
        }}
      />
    );
  }

  return (
    <DisplayEnvVar
      hasAdminPermissions={hasAdminPermissions}
      environmentVariable={environmentVariable}
      onEdit={() => {
        void formState.setFieldValue("editedVars", [
          ...formState.values.editedVars,
          {
            oldEnvVar: environmentVariable,
            newEnvVar: {
              name: environmentVariable.name,
              value: environmentVariable.value,
            },
          },
        ]);
      }}
      onDelete={() => {
        void formState.setFieldValue("deletedVars", [
          ...formState.values.deletedVars,
          environmentVariable,
        ]);
      }}
    />
  );
}

function NewEnvVars<T extends BaseEnvironmentVariable>({
  existingEnvVars,
  hasAdminPermissions,
}: {
  existingEnvVars: Array<T>;
  hasAdminPermissions: boolean;
}) {
  const formState = useFormikContext<FormState<T>>();

  const handlePaste = (envVars: Array<BaseEnvironmentVariable>) => {
    const newVars = formState.values.newVars.filter(
      ({ name, value }) => name !== "" || value !== "",
    );

    let totalEnvVars = existingEnvVars.length;
    let tooManyEnvVars = false;
    envVars.forEach((envVar) => {
      if (totalEnvVars < MAX_NUMBER_OF_ENV_VARS) {
        newVars.push(envVar);
        totalEnvVars += 1;
      } else {
        tooManyEnvVars = true;
      }
    });

    void formState.setFieldValue("newVars", newVars, true);
    void formState.setFieldValue("tooManyEnvVars", tooManyEnvVars);

    // https://github.com/jaredpalmer/formik/issues/2059#issuecomment-612733378
    setTimeout(() =>
      formState.setTouched({
        newVars: newVars.map(() => ({ name: true, value: true })),
      }),
    );
  };

  return (
    <div>
      {formState.values.newVars.length > 0 && (
        <>
          <div className="divide-y divide-border-transparent border-t">
            {formState.values.newVars.map((_, index) => (
              <NewEnvVar
                key={index}
                newVarIndex={index}
                onDelete={() => {
                  const newVars = [
                    ...formState.values.newVars.slice(0, index),
                    ...formState.values.newVars.slice(index + 1),
                  ];
                  void formState.setFieldValue("newVars", newVars);
                  void formState.setFieldValue("tooManyEnvVars", false);

                  // https://github.com/jaredpalmer/formik/issues/2059#issuecomment-612733378
                  setTimeout(() => {
                    void formState.setTouched({
                      newVars: newVars.map(() => ({ name: true, value: true })),
                    });
                  });
                }}
                onPasteVariables={(input) => handlePaste(input)}
                isLastVariable={index === formState.values.newVars.length - 1}
              />
            ))}
          </div>

          <p className="pb-4 pt-2 text-content-secondary">
            Tip: Paste your .env file directly into here!
          </p>
        </>
      )}

      <div className="my-2 flex place-content-between">
        <div className="flex gap-2">
          {existingEnvVars.length + formState.values.newVars.length <
            MAX_NUMBER_OF_ENV_VARS && (
            <Button
              type="button"
              variant="neutral"
              onClick={() => {
                void formState.setFieldValue("newVars", [
                  ...formState.values.newVars,
                  {
                    name: "",
                    value: "",
                  },
                ]);
              }}
              icon={<PlusIcon />}
              disabled={!hasAdminPermissions}
              tip={
                !hasAdminPermissions
                  ? "You do not have permission to add new environment variables."
                  : undefined
              }
            >
              Add
            </Button>
          )}

          {existingEnvVars.length > 0 && (
            <Button
              type="button"
              variant="neutral"
              inline
              onClick={async () => {
                await copyTextToClipboard(
                  existingEnvVars
                    .map(({ name, value }) => `${name}=${value}`)
                    .join("\n"),
                );

                toast(
                  "success",
                  "Environment variables copied to the clipboard.",
                );
              }}
              icon={<ClipboardCopyIcon />}
            >
              Copy All
            </Button>
          )}
        </div>

        {(formState.values.editedVars.length > 0 ||
          formState.values.newVars.length > 0 ||
          formState.values.deletedVars.length > 0) && (
          <Button
            type="submit"
            disabled={formState.isSubmitting || !formState.isValid}
          >
            {formState.values.editedVars.length +
              formState.values.newVars.length +
              formState.values.deletedVars.length >=
            2
              ? "Save All"
              : "Save"}
          </Button>
        )}
      </div>

      {formState.values.tooManyEnvVars && (
        <Callout variant="error">
          You've reached the environment variable limit (
          {MAX_NUMBER_OF_ENV_VARS}). Some pasted environment variables have been
          omitted.
        </Callout>
      )}
    </div>
  );
}

function NewEnvVar({
  newVarIndex,
  onDelete,
  onPasteVariables,
  isLastVariable,
}: {
  newVarIndex: number;
  onDelete: () => void;
  onPasteVariables: (variables: Array<BaseEnvironmentVariable>) => void;
  isLastVariable: boolean;
}) {
  const nameId = useId();
  const valueId = useId();
  const formState = useFormikContext();
  const { value } = (formState.values as any).newVars[newVarIndex];

  return (
    <div className={ENVIRONMENT_VARIABLES_ROW_CLASSES}>
      <label
        htmlFor={nameId}
        className={`flex flex-col gap-1 ${ENVIRONMENT_VARIABLE_NAME_COLUMN}`}
      >
        <div>
          <ValidatedTextInput
            formKey={`newVars[${newVarIndex}].name`}
            id={nameId}
            onPaste={(e) => {
              const variables = parseEnvVars(e.clipboardData.getData("text"));
              if (variables) {
                e.preventDefault();
                onPasteVariables(variables);
              }
            }}
            autoFocus={isLastVariable}
          />
        </div>
      </label>

      <label htmlFor={valueId} className="flex grow flex-col gap-1">
        <div>
          <ValidatedTextInput
            formKey={`newVars[${newVarIndex}].value`}
            id={valueId}
            noAutocomplete
          />
          {value.length > 1 && value.startsWith('"') && value.endsWith('"') && (
            <Callout>
              Environment variables usually shouldn't be surrounded by quotes.
              Quotes are useful in shell syntax and .env files but shouldn't be
              included in the environment variable value.
            </Callout>
          )}
        </div>
      </label>

      <div className="pt-1">
        <Button
          tip="Remove"
          type="button"
          onClick={() => {
            onDelete();
          }}
          variant="neutral"
          inline
          size="sm"
          icon={<Cross2Icon />}
        />
      </div>
    </div>
  );
}

function ValidatedTextInput({
  formKey,
  id,
  noAutocomplete = false,
  onPaste = undefined,
  autoFocus = false,
}: {
  formKey: string;
  id: string;
  noAutocomplete?: boolean;
  onPaste?: ClipboardEventHandler;
  autoFocus?: boolean;
}) {
  const formState = useFormikContext();
  const error = (formState.errors as Record<string, string>)[formKey];

  return (
    <TextInput
      id={id}
      labelHidden
      disabled={formState.isSubmitting}
      {...formState.getFieldProps(formKey)}
      autoComplete={noAutocomplete ? "off" : undefined}
      onPaste={onPaste}
      error={
        (getIn(formState.touched, formKey) ||
          error === ERROR_ENV_VAR_NOT_UNIQUE) &&
        error
      }
      autoFocus={autoFocus}
    />
  );
}
