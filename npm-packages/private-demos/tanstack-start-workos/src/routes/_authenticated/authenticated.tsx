import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/_authenticated/authenticated')({
  component: AuthenticatedPage,
});

function AuthenticatedPage() {
  return <p>Welcome!</p>;
}
