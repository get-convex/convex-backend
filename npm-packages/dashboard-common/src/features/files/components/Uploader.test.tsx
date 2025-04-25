import { ConvexProvider } from "convex/react";
import { act, render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import udfs from "@common/udfs";
import * as nextRouter from "next/router";
import { mockConvexReactClient } from "@common/lib/mockConvexReactClient";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import { mockDeploymentInfo } from "@common/lib/mockDeploymentInfo";
import {
  Uploader,
  useUploadFiles,
} from "@common/features/files/components/Uploader";

const mockRouter = jest
  .fn()
  .mockImplementation(() => ({ route: "/", query: {} }));
(nextRouter as any).useRouter = mockRouter;

// @ts-expect-error
global.fetch = jest.fn(() =>
  Promise.resolve({
    ok: true,
    json: () => Promise.resolve({ storageId: "storageID1" }),
  }),
);

const generateUploadUrl = () => "https://upload/url";

const mockClient = mockConvexReactClient()
  .registerMutationFake(udfs.fileStorageV2.generateUploadUrl, generateUploadUrl)
  .registerQueryFake(udfs.components.list, () => []);

describe("Uploader", () => {
  beforeEach(jest.clearAllMocks);

  function UploaderWithLogic() {
    const useUploadFilesResult = useUploadFiles();
    return <Uploader useUploadFilesResult={useUploadFilesResult} />;
  }

  const setup = () =>
    act(() =>
      render(
        <DeploymentInfoContext.Provider value={mockDeploymentInfo}>
          <ConvexProvider client={mockClient}>
            <UploaderWithLogic />
          </ConvexProvider>
        </DeploymentInfoContext.Provider>,
      ),
    );

  it("should upload", async () => {
    const { getByTestId } = await setup();
    const user = userEvent.setup();
    const uploader = getByTestId("uploader");
    const file = new File(["hello"], "filename");
    await user.upload(uploader, file);
    expect(global.fetch).toHaveBeenCalledTimes(1);
    expect(global.fetch).toHaveBeenCalledWith("https://upload/url", {
      body: file,
      headers: undefined,
      method: "POST",
    });
  });
});
