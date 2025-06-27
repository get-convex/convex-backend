---
title: Chef
description: "How to use Chef by Convex"
---

Chef is an AI app builder that builds complex full-stack apps. It leverages the
full power of the Convex platform to one-shot apps like Slack, Instagram, and
Notion.

This means Chef can: build real-time apps, upload files, do text search and take
advantage of Convex Components.

<CardLink
  className="convex-hero-card"
  item={{
    href: "https://chef.convex.dev",
    label: "Prompt to start an app with Convex Chef",
  }}
/>

<div className="center-image" style={{ maxWidth: "500px" }}>
  ![Chef Screenshot](/screenshots/chef_preview.png)
</div>

## Deploying to production

Chef does have a built in ability to deploy your the dev version of your app for
you to immediately share with your friends to try.

For apps intended to be built and maintained over the long term, we recommend
downloading the code and importing it into your preferred IDE. When you download
the code from Chef, your project automatically comes with
[Cursor rules for Convex](/ai.mdx), helping you keep coding with confidence.

### Download the code

<div className="center-image" style={{ maxWidth: "700px" }}>
  ![Chef Screenshot](/screenshots/chef_download.png)
</div>

At the top right of the Chef UI there is a download code button. Download the
code and you’ll get a zip file.

Unzip the file and put the folder in your desired location. We recommend
renaming the folder to the name of your app for convenience. For the rest of the
setup, open up the terminal and `cd` into your app:

```bash
cd ~/<app folder>
```

### Install dependencies

Run the following command to install all dependencies for your project

```bash
npm i
```

### Run you app

Run the following command run your app, and setup Convex if you haven’t already.

```bash
npm run dev
```

Follow any instructions to login to Convex from your machine.

<Admonition type="caution">
  You have now taken over from Chef for development of this app. Chef doesn't
  have the ability to re-import a project or track any progress from outside it.
  Going back to this project on Chef will cause conflicts in your project.
</Admonition>

### Set up the frontend build script

Chef projects don’t come with a build script. So make sure to add the following
to your `package.json` file:

```tsx
  "scripts": {
		//... other scripts
    "build": "vite build"
  },
```

### Recommended: Setup Git

In the terminal run the following three commands setup git for your app. The
downloaded code comes with a `.gitignore` file.

```bash
git init
git add --all
git commit -m "Initial commit"
```

It's also recommended you setup a remote git repository with
[GitHub](https://github.com/) if you're going to use the production hosting
guides below.

### Set up production frontend hosting

Follow one of the Convex [hosting guides](/production/hosting/hosting.mdx) to
set up frontend hosting and continuous deployment of your frontend and backend
code.

### Initialize Convex Auth for Prod

Once you have a production deployment. You need to
[set up Convex Auth for production](https://labs.convex.dev/auth/production).

## Integrations

### OpenAI

If you ask Chef to use AI, by default it will try to use the built in OpenAI
proxy with a limited number of calls. This helps you prototype your AI app idea
quickly.

However, at some point the built in number of calls will run out and you'll need
to provide your own OpenAI API Key and remove the proxy URL.

So that means you'll have to find the code that looks like this:

```typescript
const openai = new OpenAI({
  baseURL: process.env.CONVEX_OPENAI_BASE_URL,
  apiKey: process.env.CONVEX_OPENAI_API_KEY,
});
```

And remove the baseURL parameter:

```typescript
const openai = new OpenAI({
  apiKey: process.env.CONVEX_OPENAI_API_KEY,
});
```

Chef may automatically prompt you to change the environment variable. But if it
doesn't, you can change it by going to the "Database" tab. Then click on
Settings > Environment Variables and change `CONVEX_OPENAI_API_KEY` to your
[personal OpenAI key](https://platform.openai.com).

We plan on making this transition better over time.

### Resend

Chef comes with a built in way to send emails to yourself via Resend. You can
only send emails to the account you used to log into Chef. To send emails to
anyone, you have to setup your app for production with a domain name. This is a
limitation of how email providers work to combat spam.

## FAQs

### What browsers does Chef support?

Chef is best used on desktop/laptop browsers. It may work on some tablet or
mobile browsers. Chef does not work in Safari on any platform.

### How does the pricing for Chef work?

Chef pricing is primarily based on AI token usage. The free plan gives you
enough tokens to build the first version of your app in a small number of
prompts. After that you can upgrade to the Starter plan that where you can pay
for tokens as you go.

### What’s the difference between Chef and Convex?

Chef is an AI app builder that builds full-stack apps. Convex is the backend and
database that powers Chef.

### Can I import my existing app to Chef?

Chef currently doesn’t have import and GitHub integration. But you can get most
of the value by setting up the [Convex AI Rules and MCP server](/ai.mdx) in your
Agentic IDE like Cursor.

### Are there any best practices for Chef?

Yes! Check out this
[tips post written by one of our engineers](https://stack.convex.dev/chef-cookbook-tips-working-with-ai-app-builders).

### What Convex Components can Chef use?

Chef can use the
[collaborative text editor](https://www.convex.dev/components/prosemirror-sync)
component and the [presence](https://www.convex.dev/components/presence)
component. We will support more components soon. Chef supports all other Convex
features like text search, file storage, etc.

## Limitations

Chef works off a singular template with Convex, Convex Auth and React powered by
Vite. Switching these technologies is not supported by Chef.
