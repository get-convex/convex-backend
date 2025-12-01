# Convex Ents Example

This demo showcases the [Convex Ents](https://labs.convex.dev/convex-ents)
library, which provides an ergonomic layer on top of Convex's database API.

## Features Demonstrated

- **Ent Schema**: Defining entities with relations (edges) between them
- **Unique Fields**: Email and slug fields with uniqueness constraints
- **Relations**: One-to-many and many-to-many relationships
- **Edge Traversal**: Reading related documents using `.edge()` and `.edges()`
- **CRUD Operations**: Creating, reading, updating, and deleting ents
- **Filtering**: Querying ents with filters

## Schema

The example implements a simple blog system with:

- **Users**: Authors with unique emails
- **Posts**: Blog posts with unique slugs, linked to authors
- **Comments**: Comments on posts, linked to both posts and authors
- **Tags**: Tags with unique slugs, many-to-many relationship with posts

## Running the Demo

```bash
just build-js
npm run dev
```
