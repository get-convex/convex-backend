import { useMutation, useQuery } from "convex/react";
import { api } from "../convex/_generated/api";
import { Id } from "../convex/_generated/dataModel";

export default function Admin() {
  const storedFiles = useQuery(api.admin.listFiles) || [];
  const scheduledSends = useQuery(api.admin.listScheduledSends) || [];
  const cancelMessage = useMutation(api.admin.cancelMessage);
  console.log(storedFiles);
  return (
    <main>
      <h1>Admin View</h1>

      <ul>
        {storedFiles.map((fileMetadata) => (
          <li key={fileMetadata._id}>
            <ul>
              <li>Author: {fileMetadata.author}</li>
              <li>Size: {fileMetadata.size}</li>
              <li>Type: {fileMetadata.contentType}</li>
              <li>
                Time:{" "}
                {new Date(fileMetadata._creationTime).toLocaleTimeString()}
              </li>
              <li>
                {/* @snippet start useFilePreviewComponent */}
                <FilePreview file={fileMetadata} />
                {/* @snippet end useFilePreviewComponent */}
              </li>
            </ul>
          </li>
        ))}
      </ul>

      <br />

      <ul>
        {scheduledSends
          .filter(
            (sendJob) =>
              sendJob.state.kind === "pending" ||
              sendJob.state.kind === "inProgress",
          )
          .map((sendJob) => (
            <li key={sendJob._id} style={{ display: "inline-block" }}>
              <span>{sendJob.args[0]["author"]}:</span>
              <span>{sendJob.args[0]["body"]}</span>
              <br />
              <span style={{ fontStyle: "italic" }}>
                Scheduled for:{" "}
                {new Date(sendJob.scheduledTime).toLocaleTimeString()}
              </span>
              <br />
              <button
                style={{ marginLeft: "0" }}
                onClick={(e) => {
                  e.preventDefault();
                  cancelMessage({ jobId: sendJob._id });
                }}
              >
                Cancel message.
              </button>
            </li>
          ))}
      </ul>
    </main>
  );
}

// @snippet start filePreviewComponent
function FilePreview({ file }: { file: { _id: Id<"_storage"> } }) {
  const fileUrl = useQuery(api.admin.getFileUrl, { id: file._id }) || "";
  return (
    <div>
      <img src={fileUrl} height="300px" width="auto" />
    </div>
  );
}
// @snippet end filePreviewComponent
