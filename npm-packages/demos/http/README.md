# HTTP Action Example App

This example demonstrates how to use Convex
[HTTP actions](https://docs.convex.dev/functions/http-actions).

## Running the App

To run the web app:

```
npm install
npm run dev
```

To call the endpoints (e.g. using `curl`):

```
export DEPLOYMENT_NAME="tall-sheep-123"
curl "https://$DEPLOYMENT_NAME.convex.site/getMessagesByAuthor?authorNumber=123"
curl -d '{ "author": "User 123", "body": "Hello world" }' \
    -H 'content-type: application/json' "https://$DEPLOYMENT_NAME.convex.site/postMessage"
```
