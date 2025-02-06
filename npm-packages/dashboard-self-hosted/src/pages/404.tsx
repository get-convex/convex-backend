import { Button } from "dashboard-common/elements/Button";

export default function Custom404() {
  return (
    <div className="flex h-screen items-center justify-center">
      <div className="flex gap-2 divide-x text-content-primary">
        <h2>404</h2>
        <div className="flex items-center gap-1 pl-2">
          <p>This page could not be found.</p>
          <Button
            variant="unstyled"
            onClick={() => {
              window.location.href = "/";
            }}
            passHref
            className="flex items-center underline"
          >
            Go back.
          </Button>
        </div>
      </div>
    </div>
  );
}
