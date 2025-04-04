import { HealthView } from "dashboard-common/features/health/components/HealthView";

export default function Page() {
  return (
    <HealthView
      header={<h3 className="sticky top-0 mx-6 pb-2 pt-4">Health</h3>}
      PagesWrapper={({ children }) => (
        <div className="flex min-h-0 grow">{children}</div>
      )}
      PageWrapper={({ children }) => (
        <div className="max-w-full shrink-0 grow overflow-y-auto px-6 pb-4 scrollbar">
          {children}
        </div>
      )}
    />
  );
}
