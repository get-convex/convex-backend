# Deploy Speed Test

Used for benchmarking CLI deploy speed.

Run this against production to get more realistic numbers.

Install "Network Link Conditioner" on Mac to try slower network speeds.
https://forums.developer.apple.com/forums/thread/690358 which doesn't work on
localhost

Try commands like this:

```
python generate-project.py --node-files 10 --v8-files 10 --file-size 10000; du -sh convex; time npx convex dev --once
```
