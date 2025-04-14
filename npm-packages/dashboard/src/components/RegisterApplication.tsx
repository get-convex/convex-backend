import { Button } from "@ui/Button";
import { TextInput } from "@ui/TextInput";
import { useFormik } from "formik";
import { Spinner } from "@ui/Spinner";
import { Sheet } from "@ui/Sheet";
import { useAuthHeader } from "hooks/fetching";
import { toast } from "@common/lib/utils";
import * as Yup from "yup";
import { useState } from "react";
import { useProfile } from "api/profile";
import { CheckIcon } from "@radix-ui/react-icons";

export function RegisterApplication() {
  const [done, setDone] = useState(false);
  const authHeader = useAuthHeader();
  const profile = useProfile();

  const formState = useFormik({
    initialValues: {
      applicationName: "",
      domain: "",
      redirectUris: "",
      description: "",
      contactEmail: profile?.email ?? "",
    },
    validationSchema: Yup.object({
      applicationName: Yup.string()
        .max(128)
        .required("Application name is required"),
      domain: Yup.string()
        .matches(
          /^https?:\/\/.+/,
          "Domain must be a valid URL starting with http:// or https://",
        )
        .required("Domain is required"),
      redirectUris: Yup.string()
        .required("Redirect URIs are required")
        .test(
          "valid-uris",
          "Invalid URIs. Each line should be a valid URL",
          (value) => {
            if (!value) return false;
            return value.split("\n").every((uri) => {
              try {
                const _ = new URL(uri.trim());
                return true;
              } catch {
                return false;
              }
            });
          },
        ),
      description: Yup.string().max(2500).required("Description is required"),
      contactEmail: Yup.string()
        .email("Invalid email address")
        .required("Contact email is required"),
    }),
    onSubmit: async (values) => {
      try {
        const resp = await fetch("/api/contact-form", {
          method: "POST",
          body: JSON.stringify({
            teamId: 0,
            subject: `OAuth Registration Request: ${values.applicationName}`,
            message: `Application Name: ${values.applicationName}
Domain: ${values.domain}
Contact Email: ${values.contactEmail}
Redirect URIs:
${values.redirectUris}

Description:
${values.description}

This is an OAuth Application registration request.`,
          }),
          headers: {
            "Content-Type": "application/json",
            Authorization: authHeader,
          },
        });

        if (!resp.ok) {
          toast(
            "error",
            "Failed to send registration request. Please try again or email us at support@convex.dev",
            undefined,
            false,
          );
          return;
        }

        setDone(true);
        formState.resetForm();
      } catch (error) {
        toast(
          "error",
          "Failed to send registration request. Please try again or email us at support@convex.dev",
          undefined,
          false,
        );
      }
    },
  });

  if (done) {
    return (
      <Sheet className="flex max-w-prose animate-fadeInFromLoading flex-col gap-4">
        <h3 className="flex items-center gap-1">
          <CheckIcon className="size-6" /> Registration request sent!
        </h3>
        <p>
          We'll review your request and get back to you soon at{" "}
          {formState.values.contactEmail}.
        </p>
        <p>
          Please contact us at{" "}
          <a href="mailto:support@convex.dev">support@convex.dev</a> if you have
          any questions.
        </p>
      </Sheet>
    );
  }

  return (
    <Sheet className="flex max-w-prose animate-fadeInFromLoading flex-col gap-4">
      <h3>Register a Convex OAuth Application</h3>
      <p>
        Once approved, your application will be able to request access to Convex
        projects through OAuth.
      </p>
      <form className="flex flex-col gap-4" onSubmit={formState.handleSubmit}>
        <TextInput
          label="Application Name"
          id="applicationName"
          required
          onChange={formState.handleChange}
          onBlur={formState.handleBlur}
          value={formState.values.applicationName}
          error={
            formState.touched.applicationName
              ? formState.errors.applicationName
              : undefined
          }
        />
        <TextInput
          label="Domain"
          id="domain"
          required
          placeholder="https://your-domain.com"
          onChange={formState.handleChange}
          onBlur={formState.handleBlur}
          value={formState.values.domain}
          error={formState.touched.domain ? formState.errors.domain : undefined}
        />
        <TextInput
          label="Contact Email"
          id="contactEmail"
          type="email"
          required
          onChange={formState.handleChange}
          onBlur={formState.handleBlur}
          value={formState.values.contactEmail}
          error={
            formState.touched.contactEmail
              ? formState.errors.contactEmail
              : undefined
          }
        />
        <label
          htmlFor="redirectUris"
          className="flex flex-col gap-1 text-sm text-content-primary"
        >
          Redirect URIs
          <textarea
            id="redirectUris"
            name="redirectUris"
            className="h-24 resize-y rounded border bg-background-secondary px-4 py-2 text-content-primary placeholder:text-content-tertiary focus:border-border-selected focus:outline-none"
            required
            onChange={formState.handleChange}
            onBlur={formState.handleBlur}
            value={formState.values.redirectUris}
            placeholder="https://your-domain.com/oauth/callback"
          />
          {formState.touched.redirectUris && formState.errors.redirectUris && (
            <p
              className="flex max-w-prose gap-1 text-xs text-content-errorSecondary"
              role="alert"
            >
              {formState.errors.redirectUris}
            </p>
          )}
          <p className="text-xs text-content-secondary">
            Enter one redirect URI per line. These are the allowed callback URLs
            for your OAuth flow. You'll be able to modify these or add more
            later.
          </p>
        </label>
        <label
          htmlFor="description"
          className="flex flex-col gap-1 text-sm text-content-primary"
        >
          Description
          <textarea
            id="description"
            name="description"
            className="h-32 resize-y rounded border bg-background-secondary px-4 py-2 text-content-primary placeholder:text-content-tertiary focus:border-border-selected focus:outline-none"
            required
            onChange={formState.handleChange}
            onBlur={formState.handleBlur}
            value={formState.values.description}
            placeholder="Describe your application and how it will use Convex..."
          />
          {formState.touched.description && formState.errors.description && (
            <p
              className="flex max-w-prose gap-1 text-xs text-content-errorSecondary"
              role="alert"
            >
              {formState.errors.description}
            </p>
          )}
        </label>
        <Button
          type="submit"
          className="ml-auto mt-4"
          disabled={formState.isSubmitting || !formState.isValid}
          icon={formState.isSubmitting && <Spinner />}
        >
          {formState.isSubmitting ? "Sending..." : "Submit Registration"}
        </Button>
      </form>
    </Sheet>
  );
}
