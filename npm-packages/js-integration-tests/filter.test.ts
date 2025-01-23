import { ConvexHttpClient } from "convex/browser";
import { ConvexReactClient } from "convex/react";
import { api } from "./convex/_generated/api";
import { awaitQueryResult, opts } from "./test_helpers";
import { deploymentUrl } from "./common";

describe("HTTPClient", () => {
  let httpClient: ConvexHttpClient;

  beforeEach(() => {
    httpClient = new ConvexHttpClient(deploymentUrl);
  });
  afterEach(async () => {
    await httpClient.mutation(api.cleanUp.default);
  });

  test("Filter a table", async () => {
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "channel1",
      text: "hello",
    });
    const documents = await httpClient.query(api.messages.listMessages, {
      channel: "channel1",
    });
    expect(documents).toHaveProperty("length", 1);
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "channel1",
      text: "hello",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "channel2",
      text: "hello",
    });
    const documents2 = await httpClient.query(api.messages.listMessages, {
      channel: "channel1",
    });
    expect(documents2).toHaveProperty("length", 2);
    const documents3 = await httpClient.query(api.messages.listMessages, {
      channel: "channel2",
    });
    expect(documents3).toHaveProperty("length", 1);
  });

  test("Advanced filtering operations", async () => {
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c1",
      text: "t1",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c1",
      text: "t2",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c1",
      text: "t3",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c2",
      text: "t1",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c2",
      text: "t2",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c2",
      text: "t3",
    });
    await httpClient.mutation(api.messages.sendMessage, {
      channel: "c2",
      text: "t4",
    });
    const allDocs = await httpClient.query(api.messages.listMessagesInRange, {
      lower: "aa",
      lowerEqual: true,
      upper: "zz",
      upperEqual: true,
    });
    expect(allDocs).toHaveProperty("length", 7);
    const docs1 = await httpClient.query(api.messages.listMessages, {
      channel: "c1",
    });
    expect(docs1).toHaveProperty("length", 3);
    const docs2 = await httpClient.query(api.messages.listMessages, {
      channel: "c2",
    });
    expect(docs2).toHaveProperty("length", 4);

    const inclusiveDocs = await httpClient.query(
      api.messages.listMessagesInRange,
      {
        lower: "c1",
        lowerEqual: true,
        upper: "c2",
        upperEqual: true,
      },
    );
    expect(inclusiveDocs).toHaveProperty("length", 7);
    const exclusiveDocs1 = await httpClient.query(
      api.messages.listMessagesInRange,
      {
        lower: "c1",
        lowerEqual: false,
        upper: "c2",
        upperEqual: true,
      },
    );
    expect(exclusiveDocs1).toHaveProperty("length", 4);
    const exclusiveDocs2 = await httpClient.query(
      api.messages.listMessagesInRange,
      {
        lower: "c1",
        lowerEqual: true,
        upper: "c2",
        upperEqual: false,
      },
    );
    expect(exclusiveDocs2).toHaveProperty("length", 3);
    const exclusiveDocs3 = await httpClient.query(
      api.messages.listMessagesInRange,
      {
        lower: "c1",
        lowerEqual: false,
        upper: "c2",
        upperEqual: false,
      },
    );
    expect(exclusiveDocs3).toHaveProperty("length", 0);
  });
});

describe("ConvexReactClient", () => {
  let reactClient: ConvexReactClient;
  beforeEach(() => {
    reactClient = new ConvexReactClient(deploymentUrl, opts);
  });
  afterEach(async () => {
    await reactClient.mutation(api.cleanUp.default);
    await reactClient.close();
  });

  test("Subscribe to a filter", async () => {
    await reactClient.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "hello",
    });

    const watch = reactClient.watchQuery(api.messages.listMessages, {
      channel: "channel",
    });
    const resultWith4Documents = awaitQueryResult(
      watch,
      (result) => result.length === 4,
    );
    await reactClient.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "hello",
    });
    await reactClient.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "hello",
    });
    await reactClient.mutation(api.messages.sendMessage, {
      channel: "channel",
      text: "terminal",
    });
    const finalResult = await resultWith4Documents;
    expect(finalResult[3].text).toEqual("terminal");
  });
});
