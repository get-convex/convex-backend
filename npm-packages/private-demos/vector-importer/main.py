import argparse
import json
import os
from itertools import chain, islice

from dotenv import load_dotenv

from convex import ConvexClient

parser = argparse.ArgumentParser(
    prog="Vector Importer",
    description="Imports vectors from jsonl files in a specific format",
)
parser.add_argument(
    "filename",
    help="The .jsonl file (uncompressed) from https://drive.google.com/file/d/1qRJWC4kiM9xZ-oTbiqK9ii0vPciNHhkI/view?usp=drive_link",
)

args = parser.parse_args()

load_dotenv(".env.local")
load_dotenv()

client = ConvexClient(os.getenv("CONVEX_URL"))


def read_embeddings():
    with open(args.filename, "r") as f:
        for jsonline in f:
            yield json.loads(jsonline)


def chunked_embeddings(size, embeddings_json):
    for first in embeddings_json:
        yield chain([first], islice(embeddings_json, size - 1))


for chunk in chunked_embeddings(90, read_embeddings()):
    mapped = list(
        map(
            lambda jsonobj: dict(
                input=jsonobj[0]["input"], embedding=jsonobj[1]["data"][0]["embedding"]
            ),
            chunk,
        )
    )
    client.mutation("importEmbeddings:importEmbedding", dict(docs=mapped))
