import { useMutation, useQuery } from "convex/react";
import { useState } from "react";
import { api } from "../convex/_generated/api";
import { Id } from "../convex/_generated/dataModel";

export default function App() {
  const users = useQuery(api.users.listUsers);
  const posts = useQuery(api.posts.listPublishedPosts);
  const tags = useQuery(api.tags.listTags);

  const createUser = useMutation(api.users.createUser);
  const createPost = useMutation(api.posts.createPost);
  const createTag = useMutation(api.tags.createTag);

  const [userName, setUserName] = useState("");
  const [userEmail, setUserEmail] = useState("");
  const [postTitle, setPostTitle] = useState("");
  const [postSlug, setPostSlug] = useState("");
  const [postContent, setPostContent] = useState("");
  const [tagName, setTagName] = useState("");
  const [tagSlug, setTagSlug] = useState("");

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    await createUser({ name: userName, email: userEmail });
    setUserName("");
    setUserEmail("");
  };

  const handleCreateTag = async (e: React.FormEvent) => {
    e.preventDefault();
    await createTag({ name: tagName, slug: tagSlug });
    setTagName("");
    setTagSlug("");
  };

  const handleCreatePost = async (e: React.FormEvent) => {
    e.preventDefault();
    if (users && users.length > 0) {
      await createPost({
        title: postTitle,
        slug: postSlug,
        content: postContent,
        authorId: users[0]._id,
        published: true,
      });
      setPostTitle("");
      setPostSlug("");
      setPostContent("");
    }
  };

  return (
    <div className="container mt-5">
      <h1 className="mb-4">Convex Ents Example</h1>
      <p className="lead">
        This demo showcases the Convex Ents library for managing relational data
        in Convex.
      </p>

      <div className="row mt-5">
        <div className="col-md-6">
          <h2>Create User</h2>
          <form onSubmit={handleCreateUser}>
            <div className="mb-3">
              <input
                type="text"
                className="form-control"
                placeholder="Name"
                value={userName}
                onChange={(e) => setUserName(e.target.value)}
                required
              />
            </div>
            <div className="mb-3">
              <input
                type="email"
                className="form-control"
                placeholder="Email (unique)"
                value={userEmail}
                onChange={(e) => setUserEmail(e.target.value)}
                required
              />
            </div>
            <button type="submit" className="btn btn-primary">
              Create User
            </button>
          </form>

          <h3 className="mt-4">Users ({users?.length ?? 0})</h3>
          <ul className="list-group">
            {users?.map(
              (user: {
                _id: Id<"users">;
                name: string;
                email: string;
                bio?: string;
              }) => (
                <li key={user._id} className="list-group-item">
                  <strong>{user.name}</strong> - {user.email}
                </li>
              ),
            )}
          </ul>
        </div>

        <div className="col-md-6">
          <h2>Create Tag</h2>
          <form onSubmit={handleCreateTag}>
            <div className="mb-3">
              <input
                type="text"
                className="form-control"
                placeholder="Tag Name"
                value={tagName}
                onChange={(e) => setTagName(e.target.value)}
                required
              />
            </div>
            <div className="mb-3">
              <input
                type="text"
                className="form-control"
                placeholder="Slug (unique)"
                value={tagSlug}
                onChange={(e) => setTagSlug(e.target.value)}
                required
              />
            </div>
            <button type="submit" className="btn btn-primary">
              Create Tag
            </button>
          </form>

          <h3 className="mt-4">Tags ({tags?.length ?? 0})</h3>
          <ul className="list-group">
            {tags?.map(
              (tag: { _id: Id<"tags">; name: string; slug: string }) => (
                <li key={tag._id} className="list-group-item">
                  <strong>{tag.name}</strong> ({tag.slug})
                </li>
              ),
            )}
          </ul>
        </div>
      </div>

      <div className="row mt-5">
        <div className="col-12">
          <h2>Create Post</h2>
          <form onSubmit={handleCreatePost}>
            <div className="mb-3">
              <input
                type="text"
                className="form-control"
                placeholder="Post Title"
                value={postTitle}
                onChange={(e) => setPostTitle(e.target.value)}
                required
              />
            </div>
            <div className="mb-3">
              <input
                type="text"
                className="form-control"
                placeholder="Slug (unique)"
                value={postSlug}
                onChange={(e) => setPostSlug(e.target.value)}
                required
              />
            </div>
            <div className="mb-3">
              <textarea
                className="form-control"
                placeholder="Post Content"
                value={postContent}
                onChange={(e) => setPostContent(e.target.value)}
                rows={4}
                required
              />
            </div>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={!users || users.length === 0}
            >
              Create Post
            </button>
            {(!users || users.length === 0) && (
              <small className="text-muted ms-2">
                Create a user first to author posts
              </small>
            )}
          </form>

          <h3 className="mt-4">Published Posts ({posts?.length ?? 0})</h3>
          <div className="row">
            {posts?.map(
              (post: {
                _id: Id<"posts">;
                title: string;
                content: string;
                author: { _id: Id<"users">; name: string };
                tags: Array<{ _id: Id<"tags">; name: string }>;
              }) => (
                <div key={post._id} className="col-md-6 mb-3">
                  <div className="card">
                    <div className="card-body">
                      <h5 className="card-title">{post.title}</h5>
                      <h6 className="card-subtitle mb-2 text-muted">
                        by {post.author.name}
                      </h6>
                      <p className="card-text">{post.content}</p>
                      <div>
                        {post.tags.map(
                          (tag: { _id: Id<"tags">; name: string }) => (
                            <span
                              key={tag._id}
                              className="badge bg-secondary me-1"
                            >
                              {tag.name}
                            </span>
                          ),
                        )}
                      </div>
                    </div>
                  </div>
                </div>
              ),
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
