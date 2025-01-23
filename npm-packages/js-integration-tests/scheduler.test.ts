import { ConvexHttpClient } from "convex/browser";
import { api } from "./convex/_generated/api";
import { deploymentUrl } from "./common";

type ScheduledJobStatus =
  | "success"
  | "pending"
  | "inProgress"
  | "failed"
  | "canceled"
  | null;

describe("Scheduler in component", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  async function waitForJob(getStatus: () => Promise<ScheduledJobStatus>) {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const status = await getStatus();
      expect(status).not.toEqual(null);
      if (status === "success") {
        break;
      } else {
        expect(status).toEqual("pending");
      }
    }
  }

  test("schedule in parent", async () => {
    const scheduled = await httpClient.mutation(
      api.scheduler.scheduleInParent,
      { message: "hello" },
    );
    // Scheduled job does not exist in child.
    const statusInChild = await httpClient.query(
      api.scheduler.statusInComponent,
      { id: scheduled },
    );
    expect(statusInChild).toEqual(null);
    // Scheduled job exists in parent and eventually completes.
    await waitForJob(() =>
      httpClient.query(api.scheduler.statusInParent, { id: scheduled }),
    );
    // Message exists in parent, not in child.
    const messages = await httpClient.query(
      api.scheduler.listAllMessagesInParent,
    );
    expect(messages.length).toEqual(1);
    const messagesInChild = await httpClient.query(
      api.scheduler.listAllMessagesInComponent,
    );
    expect(messagesInChild.length).toEqual(0);
  });

  test("schedule within component", async () => {
    const scheduled = await httpClient.mutation(
      api.scheduler.scheduleWithinComponent,
      { message: "hello" },
    );
    // Scheduled job does not exist in parent.
    const statusInParent = await httpClient.query(
      api.scheduler.statusInParent,
      { id: scheduled },
    );
    expect(statusInParent).toEqual(null);
    // Scheduled job exists in child and eventually completes.
    await waitForJob(() =>
      httpClient.query(api.scheduler.statusInComponent, { id: scheduled }),
    );
    // Message exists in child, not in parent.
    const messages = await httpClient.query(
      api.scheduler.listAllMessagesInParent,
    );
    expect(messages.length).toEqual(0);
    const messagesInChild = await httpClient.query(
      api.scheduler.listAllMessagesInComponent,
    );
    expect(messagesInChild.length).toEqual(1);
  });

  test("schedule child from parent", async () => {
    const scheduled = await httpClient.mutation(
      api.scheduler.scheduleChildFromParent,
      { message: "hello" },
    );
    // Scheduled job does not exist in child.
    const statusInChild = await httpClient.query(
      api.scheduler.statusInComponent,
      { id: scheduled },
    );
    expect(statusInChild).toEqual(null);
    // Scheduled job exists in parent and eventually completes.
    await waitForJob(() =>
      httpClient.query(api.scheduler.statusInParent, { id: scheduled }),
    );
    // Message exists in child, not in parent.
    const messages = await httpClient.query(
      api.scheduler.listAllMessagesInParent,
    );
    expect(messages.length).toEqual(0);
    const messagesInChild = await httpClient.query(
      api.scheduler.listAllMessagesInComponent,
    );
    expect(messagesInChild.length).toEqual(1);
  });
});

describe("Scheduler basic tests", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  async function waitForJob(
    targetStatus: ScheduledJobStatus,
    getStatus: () => Promise<ScheduledJobStatus>,
  ) {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const status = await getStatus();
      expect(status).not.toEqual(null);
      if (status === targetStatus) {
        break;
      } else {
        expect(status).toEqual("pending");
      }
    }
  }

  test("schedule self canceling", async () => {
    const scheduled = await httpClient.mutation(
      api.scheduler.scheduleSelfCanceling,
    );
    await waitForJob("canceled", () =>
      httpClient.query(api.scheduler.statusInParent, { id: scheduled }),
    );
  });
});
