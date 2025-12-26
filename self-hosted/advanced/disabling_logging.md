## Redacting logs to client

The cloud-hosted product automatically redacts logs to prevent any leaking of
PII. If you would like to also redact log information in your self-hosted
deployment, set the `REDACT_LOGS_TO_CLIENT` environment variable to `true`.

## Disabling self-hosted beacon

Self-hosted builds contain a beacon to help Convex understand usage of the
product. The information collected is anonymous and minimal, containing a random
identifier plus the version of the backend in use. You may opt out of the beacon
by setting the environment variable `DISABLE_BEACON` to `true`.
