import { Button } from "dashboard-common/elements/Button";
import { TextInput } from "dashboard-common/elements/TextInput";
import { useFormik } from "formik";
import { useCreateProfileEmail } from "api/profile";
import * as Yup from "yup";
import { MemberEmailResponse } from "generatedApi";

export function EmailCreateForm({
  emails,
  onCreate,
}: {
  emails: MemberEmailResponse[];
  onCreate: () => void;
}) {
  const schema = Yup.object().shape({
    email: Yup.string()
      .email()
      .required()
      .max(254, "Email must be at most 254 characters long.")
      .notOneOf(
        emails.map((email) => email.email),
        "This email is already associated with your account.",
      ),
  });

  const createEmail = useCreateProfileEmail();
  const formState = useFormik({
    initialValues: {
      email: "",
    },
    validateOnChange: false,
    validateOnBlur: true,
    validationSchema: schema,
    onSubmit: async (values, form) => {
      try {
        await createEmail(values);
        onCreate();
      } catch (error: any) {
        form.setErrors({ email: error.message });
      }
    },
  });

  return (
    <form
      onSubmit={formState.handleSubmit}
      className="flex items-start gap-2"
      data-testid="email-create-form"
    >
      <TextInput
        id="email"
        placeholder="Email"
        labelHidden
        onChange={(e) => {
          // Reset the errors so the user can blur the form
          formState.setErrors({});
          formState.handleChange(e);
        }}
        onBlur={formState.handleBlur}
        value={formState.values.email}
        error={formState.touched ? formState.errors.email : undefined}
      />
      <Button
        type="submit"
        disabled={
          !formState.dirty || formState.isSubmitting || !formState.isValid
        }
      >
        Save
      </Button>
    </form>
  );
}
