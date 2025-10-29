import { actionGeneric as action } from "convex/server";

import "undici";

// Check that `require` still works with our bundler.
/* eslint-disable @typescript-eslint/no-require-imports */
const { S3Client, PutObjectCommand } = require("@aws-sdk/client-s3");
const Stripe = require("stripe");
const { google } = require("googleapis");
const twilio = require("twilio");
// https://github.com/auth0/node-auth0/blob/master/README.md#getting-error-cant-resolve-superagent-proxy-when-bundling-with-webpack
const { AuthenticationClient } = require("auth0");
/* eslint-enable @typescript-eslint/no-require-imports */

import * as sgMail from "@sendgrid/mail";

export const s3Example = action(async () => {
  const client = new S3Client({ region: "somewhere" });
  const params = {
    Bucket: "asdf",
    Key: "key",
    Body: "body",
  };
  const results = await client.send(new PutObjectCommand(params));
  console.log(results);
});

export const stripeExample = action(async () => {
  const stripe = Stripe("sk_test_4eC39HqLyjWDarjtT1zdp7dc");
  const product = await stripe.products.create({
    name: "Gold Special",
  });
  const price = await stripe.prices.create({
    product: product.id,
    unit_amount: 2000,
    currency: "usd",
  });
  const session = await stripe.checkout.sessions.create({
    line_items: [
      {
        price: price.id,
        quantity: 1,
      },
    ],
    mode: "payment",
    success_url: `https://example.com/success.html`,
    cancel_url: `https://example.com/cancel.html`,
  });
  return session.url;
});

export const googleExample = action(async () => {
  const blogger = google.blogger({
    version: "v3",
    auth: "YOUR API KEY",
  });
  const params = {
    blogId: "3213900",
  };
  const result = await blogger.blogs.get(params);
  console.log(result);
});

export const twilioExample = action(async () => {
  const accountSid = "ACXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"; // Your Account SID from www.twilio.com/console
  const authToken = "your_auth_token"; // Your Auth Token from www.twilio.com/console
  const client = new twilio(accountSid, authToken);
  await client.messages.create({
    body: "Hello from Node",
    to: "+12345678901", // Text this number
    from: "+12345678901", // From a valid Twilio number
  });
});

export const auth0Example = action(async () => {
  const client = new AuthenticationClient({ domain: "asdf", clientId: "asdf" });
  const result = await client.getProfile("asdf");
  console.log(result);
});

export const sendgridExample = action(async () => {
  sgMail.setApiKey(process.env.SENDGRID_API_KEY ?? "");
  const msg = {
    to: "test@example.com", // Change to your recipient
    from: "test@example.com", // Change to your verified sender
    subject: "Sending with SendGrid is Fun",
    text: "and easy to do anywhere, even with Node.js",
    html: "<strong>and easy to do anywhere, even with Node.js</strong>",
  };
  const response = await sgMail.send(msg);
  console.log(response);
});

export const testFilename = action(async () => {
  return __filename;
});

export const testDirname = action(async () => {
  return __dirname;
});
