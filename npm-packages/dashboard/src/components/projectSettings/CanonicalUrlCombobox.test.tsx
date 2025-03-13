import "@testing-library/jest-dom";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useUpdateCanonicalUrl } from "hooks/deploymentApi";
import { CanonicalUrlCombobox } from "./CustomDomains";

jest.mock("hooks/deploymentApi", () => ({
  useUpdateCanonicalUrl: jest.fn().mockReturnValue(jest.fn()),
}));

jest.mock("api/api", () => ({}));

const defaultProps: React.ComponentProps<typeof CanonicalUrlCombobox> = {
  label: "Canonical URL",
  defaultUrl: {
    kind: "default",
    url: "https://joyful-capybara-123.convex.cloud",
  },
  canonicalUrl: {
    kind: "loaded",
    url: "https://joyful-capybara-123.convex.cloud",
  },
  vanityDomains: [
    {
      creationTime: 1741014640,
      instanceName: "wandering-fish-513",
      requestDestination: "convexCloud",
      domain: "api.chess.convex.dev",
      verificationTime: 1741726688,
      creationTs: "2025-03-03 15:10:40.760874 UTC",
      verificationTs: "2025-03-11 20:58:08.146394 UTC",
    },
    {
      creationTime: 1736874886,
      instanceName: "wandering-fish-513",
      requestDestination: "convexSite",
      domain: "chess.convex.dev",
      verificationTime: 1741726560,
      creationTs: "2025-01-14 17:14:46.332041 UTC",
      verificationTs: "2025-03-11 20:56:00.634010 UTC",
    },
  ],
  requestDestination: "convexCloud",
};

describe("CanonicalUrlCombobox", () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it("is disabled when there are no custom domains", async () => {
    render(<CanonicalUrlCombobox {...defaultProps} vanityDomains={[]} />);
    const combobox = screen.getByRole("button");
    expect(combobox).toHaveTextContent(
      "https://joyful-capybara-123.convex.cloud (default)",
    );
    expect(combobox).toBeDisabled();
  });

  it("selects the user-chosen canonical domain when it's loaded", async () => {
    render(
      <CanonicalUrlCombobox
        {...defaultProps}
        canonicalUrl={{
          kind: "loaded",
          url: "https://api.chess.convex.dev",
        }}
      />,
    );
    const combobox = screen.getByRole("button");
    expect(combobox).toHaveTextContent("https://api.chess.convex.dev");

    await userEvent.click(combobox);
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(2);
    expect(options[0]).toHaveTextContent(
      "https://joyful-capybara-123.convex.cloud (default)",
    );
    expect(options[1]).toHaveTextContent("api.chess.convex.dev");

    // The user can set the default value, which is set as `null`
    await userEvent.click(options[0]);
    const updateCanonicalUrl = useUpdateCanonicalUrl("convexCloud");
    expect(updateCanonicalUrl).toHaveBeenCalledWith(null);
  });

  it("allows the user to select a custom domain", async () => {
    render(<CanonicalUrlCombobox {...defaultProps} />);
    const combobox = screen.getByRole("button");

    await userEvent.click(combobox);
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(2);
    expect(options[0]).toHaveTextContent(
      "https://joyful-capybara-123.convex.cloud (default)",
    );
    expect(options[1]).toHaveTextContent("https://api.chess.convex.dev");

    await userEvent.click(options[1]);
    const updateCanonicalUrl = useUpdateCanonicalUrl("convexCloud");
    expect(updateCanonicalUrl).toHaveBeenCalledWith(
      "https://api.chess.convex.dev",
    );
  });

  it("shows the canonical domain as “disconnected” when it is not one of the known domains", async () => {
    render(
      <CanonicalUrlCombobox
        {...defaultProps}
        canonicalUrl={{
          kind: "loaded",
          url: "https://api.chess.concave.example/",
        }}
      />,
    );
    const combobox = screen.getByRole("button");
    expect(combobox).toHaveTextContent(
      "https://api.chess.concave.example/ (disconnected)",
    );

    await userEvent.click(combobox);
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(3);
    expect(options[0]).toHaveTextContent(
      "https://joyful-capybara-123.convex.cloud (default)",
    );
    expect(options[1]).toHaveTextContent(
      "https://api.chess.concave.example/ (disconnected)",
    );
    expect(options[2]).toHaveTextContent("api.chess.convex.dev");
  });

  it("only shows “default” when the default url is unknown and allows the user to select it", async () => {
    render(
      <CanonicalUrlCombobox
        {...defaultProps}
        defaultUrl={{
          kind: "unknownDefault",
        }}
        canonicalUrl={{
          kind: "loaded",
          url: "https://api.chess.convex.dev",
        }}
      />,
    );
    const combobox = screen.getByRole("button");
    expect(combobox).toHaveTextContent("https://api.chess.convex.dev");

    await userEvent.click(combobox);
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(2);
    expect(options[0]).toHaveTextContent("default");
    expect(options[1]).toHaveTextContent("https://api.chess.convex.dev");

    // When selecting the default option, we send `null` as a value to the endpoint.
    await userEvent.click(options[0]);
    const updateCanonicalUrl = useUpdateCanonicalUrl("convexCloud");
    expect(updateCanonicalUrl).toHaveBeenCalledWith(null);
  });

  it("handles cases where the default URL is not known by assuming the canonical URL is the default", async () => {
    render(
      <CanonicalUrlCombobox
        {...defaultProps}
        defaultUrl={{
          kind: "unknownDefault",
        }}
      />,
    );

    const combobox = screen.getByRole("button");
    expect(combobox).toHaveTextContent(
      "https://joyful-capybara-123.convex.cloud (default)",
    );
  });

  it("shows a loading state when the selected canonical domain hasn’t been loaded yet", async () => {
    render(
      <CanonicalUrlCombobox
        {...defaultProps}
        canonicalUrl={{ kind: "loading" }}
      />,
    );

    const loading = screen.getByRole("generic", { busy: true });
    expect(loading).toBeInTheDocument();
  });
});
