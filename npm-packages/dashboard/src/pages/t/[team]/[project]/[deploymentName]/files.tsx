import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { FileStorageView } from "@common/features/files/components/FileStorageView";
import { usePostHog } from "hooks/usePostHog";

export { getServerSideProps } from "lib/ssr";

function FileStorageWithAnalytics() {
  const { capture } = usePostHog();
  return (
    <FileStorageView
      onFilesUploaded={(count) => capture("uploaded_files", { count })}
    />
  );
}

export default withAuthenticatedPage(FileStorageWithAnalytics);
