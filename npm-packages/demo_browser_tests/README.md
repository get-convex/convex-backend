# Demo Browser Tests

This directory contains a number of tests which use puppeteer to drive headless
chromium to make sure that the in-browser behavior of our demo apps actually
works!

## Current Oddities + Limitations

1.  The selectors are a little funky, indirect, and dependent on our current
    demo code structure instead of, say, element ids. So therefore they're a bit
    fragile. Right now, this is preferrable to complicating our demo code with
    ids that aren't used for anything in-demo (and are only used for testing).
2.  `users-and-auth` uses authentication with auth0. Yes, we do test this, but
    it's with a test account that jamie has created at auth0 for this testing
    app specifically.

## Dashboard tests

There are dashboard tests here too!

## Platform (public management API) test

These too, using dashboard browser automation helpers code to go through an
OAuth flow to get a OAuth team token.
