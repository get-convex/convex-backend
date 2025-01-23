# Convex HTTP actions

Demonstrates defining custom HTTP actions that talk to Convex (e.g. to hook up
to a webhook).

```
curl -v 'http://127.0.0.1:8001/getMessagesByAuthor?authorNumber=123'
curl -v -d '{ "author": "User 123", "body": "Hello world" }' \
    -H 'content-type: application/json' 'http://127.0.0.1:8001/postMessage'
```
