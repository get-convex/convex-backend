import { element, device, by, waitFor } from "detox";

describe("App", () => {
  beforeAll(async () => {
    await device.launchApp();
  });

  beforeEach(async () => {
    await device.reloadReactNative();
  });

  it("should load", async () => {
    // The name field should always be present, so this tests that we can
    // actually load our app bundle.
    await expect(element(by.id("NameField"))).toBeVisible();
  });

  it("should show newly created message", async () => {
    // This requires a server to be running and tests that creating a message
    // eventually causes it to appear in the message list.

    await element(by.id("MessageInput")).typeText("Testy test 123");
    await element(by.id("MessageInput")).tapReturnKey();
    await expect(element(by.id("MessagesList"))).toBeVisible();
    const attr = await element(by.id("NameField")).getAttributes();
    const name = attr.text;
    await waitFor(
      element(
        by.text(`${name}: Testy test 123`).withAncestor(by.id("MessagesList"))
      )
    )
      .toBeVisible()
      .withTimeout(10000);
  });
});