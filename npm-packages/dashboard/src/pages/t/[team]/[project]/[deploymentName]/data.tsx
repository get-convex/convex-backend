import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { DataView } from "@common/features/data/components/DataView";
import { usePostHog } from "hooks/usePostHog";

export { getServerSideProps } from "lib/ssr";

function DataViewWithAnalytics() {
  const { capture } = usePostHog();
  return (
    <DataView
      onTableCreated={() => capture("created_table")}
      onDocumentsAdded={(count) => capture("add_documents", { count })}
    />
  );
}

export default withAuthenticatedPage(DataViewWithAnalytics);
