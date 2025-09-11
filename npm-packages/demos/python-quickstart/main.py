import os

from convex import ConvexClient
from dotenv import load_dotenv

load_dotenv(".env.local")
CONVEX_URL = os.getenv("CONVEX_URL")
# or you can hardcode your deployment URL instead
# CONVEX_URL = "https://happy-otter-123.convex.cloud"

client = ConvexClient(CONVEX_URL)

print(client.query("tasks:get"))

for tasks in client.subscribe("tasks:get"):
    print(tasks)
    # this loop lasts forever, ctrl-c to exit it
