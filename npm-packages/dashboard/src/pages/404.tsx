import { Button } from "@ui/Button";
import { LoginLayout } from "layouts/LoginLayout";
import { useRouter } from "next/router";

export default function Custom404() {
  const { query } = useRouter();
  return (
    <div className="h-screen">
      <LoginLayout>
        <div className="flex gap-2 text-content-primary">
          <h2>404</h2>
          <div className="flex items-center gap-1 pl-2">
            {query.reason === "deployment_not_found" ? (
              <DeploymentNotFound />
            ) : (
              <p>This page could not be found.</p>
            )}
            <Button
              variant="unstyled"
              onClick={() => {
                window.location.href = "/";
              }}
              className="flex items-center underline"
            >
              Go back.
            </Button>
          </div>
        </div>
      </LoginLayout>
    </div>
  );
}

function DeploymentNotFound() {
  return (
    <p>
      The requested deployment could not be found or belongs to another user.
    </p>
  );
}
