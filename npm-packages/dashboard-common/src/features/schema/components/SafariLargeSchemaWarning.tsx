import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Button } from "@ui/Button";
import { EmptySection } from "@common/elements/EmptySection";

// Safari struggles to render the schema graph once a deployment has a large
// number of tables, so gate rendering behind an explicit opt-in for those users.
export const SAFARI_LARGE_SCHEMA_TABLE_COUNT = 50;

export function SafariLargeSchemaWarning({
  onRenderAnyway,
}: {
  onRenderAnyway: () => void;
}) {
  return (
    <EmptySection
      Icon={ExclamationTriangleIcon}
      color="yellow"
      header="This schema may be slow to display in Safari"
      body="Safari can struggle to render schemas with a large number of tables. For the best experience, we recommend opening the dashboard in another browser."
      action={
        <Button variant="neutral" onClick={onRenderAnyway}>
          Display schema anyway
        </Button>
      }
    />
  );
}
