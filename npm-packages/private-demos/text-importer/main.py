import argparse
import json
import os
from itertools import chain, islice

from convex import ConvexClient
from dotenv import load_dotenv

parser = argparse.ArgumentParser(
    prog="Text Importer",
    description="Imports text files from dataset files a specific format",
)
parser.add_argument(
    "filename",
    help="A file from .jsonl file (uncompressed) from https://www.dropbox.com/sh/f0q1o7tbfuissm8/AAAkB-JggUKL7KFCtl1nsRf1a?dl=0",
)

args = parser.parse_args()

load_dotenv(".env.local")
load_dotenv()

client = ConvexClient(os.getenv("CONVEX_URL"))


def read_documents():
    with open(args.filename, "r") as f:
        for jsonline in f:
            yield json.loads(jsonline)


def chunked_embeddings(size, embeddings_json):
    for first in embeddings_json:
        yield chain([first], islice(embeddings_json, size - 1))


for chunk in chunked_embeddings(90, read_documents()):
    mapped = list(
        map(
            lambda jsonobj: dict(
                text=jsonobj["text"],
            ),
            chunk,
        )
    )
    client.mutation("importDocuments:importDocuments", dict(docs=mapped))
