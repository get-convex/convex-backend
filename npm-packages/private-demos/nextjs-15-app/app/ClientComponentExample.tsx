"use client";

export function ClientComponentExample() {
  return (
    <p className="bg-violet-300 p-4 rounded-md">
      Content with a purple background was rendered using Client Components. It
      is first rendered on the server, and then hydrated and re-rendered on the
      client.
    </p>
  );
}
