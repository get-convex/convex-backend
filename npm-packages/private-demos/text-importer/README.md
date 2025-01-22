All this does is import a ton of text into a table with a text index.

There's some very simply python parsing to extract embeddings specifically from
this file:
https://drive.google.com/file/d/1qRJWC4kiM9xZ-oTbiqK9ii0vPciNHhkI/view?usp=drive_link.
That's a set of embeddings that originated from
https://www.kaggle.com/datasets/stephanst/wikipedia-simple-openai-embeddings
which was MIT licensed at the time it was downloaded.

Download the file from drive or kaggle, extract the archive into a .jsonl file,
then run the script with:

1. `poetry install`
2. `just rush update`
3. `npx convex dev --once`
4. `poetry run python main.py <path_to_jsonl>`

You can adapt the python code to parse other formats if you'd like. The main
purpose of this is to test bulk imports, particularly with vectors.
