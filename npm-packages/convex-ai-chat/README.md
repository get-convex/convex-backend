# Developing AI Chat

1.  In one terminal, run `npm run dev`, to develop the backend against your dev
    deployment

    You can use team:convex project:ai-bot, this will set up environment
    variables for you

    If you don't use this project, make sure you set up `OPENAI_API_KEY` on the
    dashboard

2.  In another terminal, run `npm run watch`, this will continuously build the
    React entrypoint

3.  In another terminal, run `npm run watch-css-docs`, this will continuously
    build the Tailwind-powered css entrypoint

After you have all of these running, copy the CONVEX_URL value from the
`.env.local` file in this directory to `npm-packages/docs/.env.local` (see the
docs README.md), and run `just run-docs`.

Now whenever you make frontend changes in this project docs will reload and
reflect them.
