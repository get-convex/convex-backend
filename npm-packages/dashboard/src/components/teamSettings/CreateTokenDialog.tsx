import { Modal } from "@ui/Modal";
import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { Formik } from "formik";
import * as Yup from "yup";
import React from "react";

const CREATE_TOKEN_SCHEMA = Yup.object({
  tokenName: Yup.string()
    .min(1, "Token name is required")
    .max(50, "Token name must be at most 50 characters")
    .required("Token name is required"),
});

export function CreateTokenDialog({
  onClose,
  onSubmit,
}: {
  onClose: () => void;
  onSubmit: (tokenName: string) => Promise<void>;
}) {
  return (
    <Modal onClose={onClose} title="Create Team Access Token">
      <Formik
        initialValues={{ tokenName: "" }}
        validationSchema={CREATE_TOKEN_SCHEMA}
        onSubmit={async (values, { setSubmitting }) => {
          await onSubmit(values.tokenName);
          setSubmitting(false);
        }}
      >
        {({
          values,
          errors,
          touched,
          handleChange,
          handleSubmit,
          isSubmitting,
        }) => (
          <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
            <p className="text-sm text-content-primary">
              This token will allow programmatic access to your team's
              resources. Keep it secure and do not share it publicly.
            </p>

            <TextInput
              id="tokenName"
              label="Token Name"
              type="text"
              value={values.tokenName}
              onChange={handleChange}
              placeholder="Enter a memorable name, like 'asdfjkl;'"
              error={
                touched.tokenName && typeof errors.tokenName === "string"
                  ? errors.tokenName
                  : undefined
              }
              required
            />

            <div className="flex justify-end gap-2">
              <Button variant="neutral" onClick={onClose} type="button">
                Cancel
              </Button>
              <Button type="submit" loading={isSubmitting}>
                Save
              </Button>
            </div>
          </form>
        )}
      </Formik>
    </Modal>
  );
}
