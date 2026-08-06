import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SWRConfig } from "swr";
import { CustomDomainsCard } from "../components/CustomDomainsCard";

const mockList = jest.fn();
const mockCreate = jest.fn();
const mockDelete = jest.fn();
const mockVerify = jest.fn();
const mockRetry = jest.fn();
const mockListCreds = jest.fn();

jest.mock("../lib/config", () => ({
  orchestratorUrl: () => "http://orchestrator.test",
}));

jest.mock("../lib/useOrchestratorToken", () => ({
  useAccessToken: () => "pat_test",
}));

jest.mock("../lib/orchestratorApi", () => ({
  listCustomDomains: (...a: unknown[]) => mockList(...a),
  createCustomDomain: (...a: unknown[]) => mockCreate(...a),
  deleteCustomDomain: (...a: unknown[]) => mockDelete(...a),
  verifyCustomDomain: (...a: unknown[]) => mockVerify(...a),
  retryCustomDomain: (...a: unknown[]) => mockRetry(...a),
  listDnsCredentials: (...a: unknown[]) => mockListCreds(...a),
  createDnsCredential: jest.fn(),
  deleteDnsCredential: jest.fn(),
}));

function renderCard() {
  // `provider` gives each test a fresh cache so one test's domains don't
  // leak into the next through SWR's module-level store.
  return render(
    <SWRConfig value={{ provider: () => new Map(), dedupingInterval: 0 }}>
      <CustomDomainsCard
        deploymentId={7}
        deploymentName="happy-otter-123"
        teamId={3}
      />
    </SWRConfig>,
  );
}

beforeEach(() => {
  mockList.mockReset();
  mockCreate.mockReset();
  mockDelete.mockReset();
  mockVerify.mockReset();
  mockRetry.mockReset();
  mockListCreds.mockReset();
  mockList.mockResolvedValue({
    domains: [],
    targetHost: "convex.example.com",
    routingEnabled: true,
    providers: [{ provider: "cloudflare", fields: [] }],
  });
  mockListCreds.mockResolvedValue({
    credentials: [{ id: 5, name: "cf", provider: "cloudflare", createdAt: 0 }],
    providers: [{ provider: "cloudflare", fields: [] }],
  });
});

test("shows the CNAME target the operator has to point DNS at", async () => {
  renderCard();
  await waitFor(() =>
    expect(screen.getByText("convex.example.com")).toBeInTheDocument(),
  );
});

test("adds a domain and refreshes the list", async () => {
  const user = userEvent.setup();
  mockCreate.mockResolvedValue({});
  renderCard();
  await waitFor(() => expect(mockList).toHaveBeenCalled());

  await user.type(screen.getByLabelText("Domain"), "api.example.com");
  await user.click(screen.getByRole("button", { name: "Add" }));

  await waitFor(() =>
    expect(mockCreate).toHaveBeenCalledWith(
      "http://orchestrator.test",
      "pat_test",
      7,
      "api.example.com",
      "http-01",
      null,
    ),
  );
  // Re-listed so the new row appears without a manual reload.
  await waitFor(() => expect(mockList).toHaveBeenCalledTimes(2));
});

test("surfaces the server's rejection instead of silently failing", async () => {
  const user = userEvent.setup();
  mockCreate.mockRejectedValue(
    new Error("api.example.com is already attached to a deployment"),
  );
  renderCard();
  await waitFor(() => expect(mockList).toHaveBeenCalled());

  await user.type(screen.getByLabelText("Domain"), "api.example.com");
  await user.click(screen.getByRole("button", { name: "Add" }));

  await waitFor(() =>
    expect(
      screen.getByText("api.example.com is already attached to a deployment"),
    ).toBeInTheDocument(),
  );
});

test("reports a domain as pending until a probe confirms the certificate", async () => {
  mockList.mockResolvedValue({
    domains: [
      {
        id: 1,
        deploymentId: 7,
        domain: "api.example.com",
        certState: "pending",
        createdAt: 0,
        challengeType: "http-01",
        dnsCredentialId: null,
        lastError: null,
      },
    ],
    targetHost: "convex.example.com",
    routingEnabled: true,
    providers: [{ provider: "cloudflare", fields: [] }],
  });
  renderCard();

  await waitFor(() => expect(screen.getByText("Pending")).toBeInTheDocument());
  expect(screen.queryByText("Active")).not.toBeInTheDocument();
});

test("shows why a check failed so the operator can fix DNS", async () => {
  const user = userEvent.setup();
  mockList.mockResolvedValue({
    domains: [
      {
        id: 1,
        deploymentId: 7,
        domain: "api.example.com",
        certState: "pending",
        createdAt: 0,
        challengeType: "http-01",
        dnsCredentialId: null,
        lastError: null,
      },
    ],
    targetHost: "convex.example.com",
    routingEnabled: true,
    providers: [{ provider: "cloudflare", fields: [] }],
  });
  mockVerify.mockResolvedValue({
    domain: "api.example.com",
    certState: "pending",
    error: "dns error: no such host",
  });
  renderCard();
  await waitFor(() => expect(mockList).toHaveBeenCalled());

  await user.click(screen.getByRole("button", { name: "Check" }));

  await waitFor(() =>
    expect(screen.getByText("dns error: no such host")).toBeInTheDocument(),
  );
});

test("warns when the orchestrator cannot actually route custom domains", async () => {
  mockList.mockResolvedValue({
    domains: [],
    targetHost: "convex.example.com",
    routingEnabled: false,
    providers: [],
  });
  renderCard();

  await waitFor(() =>
    expect(
      screen.getByText(/Custom domain routing is not enabled/),
    ).toBeInTheDocument(),
  );
});

test("sends the dns-01 challenge with the chosen credential", async () => {
  const user = userEvent.setup();
  mockCreate.mockResolvedValue({});
  renderCard();
  await waitFor(() => expect(mockListCreds).toHaveBeenCalled());

  await user.type(screen.getByLabelText("Domain"), "*.example.com");
  await user.selectOptions(screen.getByLabelText("Challenge"), "dns-01");
  await user.selectOptions(await screen.findByLabelText("Credential"), "5");
  await user.click(screen.getByRole("button", { name: "Add" }));

  await waitFor(() =>
    expect(mockCreate).toHaveBeenCalledWith(
      "http://orchestrator.test",
      "pat_test",
      7,
      "*.example.com",
      "dns-01",
      5,
    ),
  );
});

test("warns that a wildcard cannot use the http-01 challenge", async () => {
  const user = userEvent.setup();
  renderCard();
  await waitFor(() => expect(mockList).toHaveBeenCalled());

  await user.type(screen.getByLabelText("Domain"), "*.example.com");

  expect(
    screen.getByText(/Wildcard domains can only be validated with DNS-01/),
  ).toBeInTheDocument();
});

test("surfaces a failed issuance and offers a retry", async () => {
  const user = userEvent.setup();
  mockList.mockResolvedValue({
    domains: [
      {
        id: 1,
        deploymentId: 7,
        domain: "api.example.com",
        certState: "failed",
        createdAt: 0,
        challengeType: "dns-01",
        dnsCredentialId: 5,
        lastError: "Cloudflare rejected the token",
      },
    ],
    targetHost: "convex.example.com",
    routingEnabled: true,
    providers: [],
  });
  renderCard();

  await waitFor(() =>
    expect(
      screen.getByText("Cloudflare rejected the token"),
    ).toBeInTheDocument(),
  );
  await user.click(screen.getByRole("button", { name: "Retry" }));
  await waitFor(() => expect(mockRetry).toHaveBeenCalled());
});
