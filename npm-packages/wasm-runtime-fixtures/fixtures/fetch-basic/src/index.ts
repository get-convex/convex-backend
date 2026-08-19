export async function fetchText(url: string) {
  const response = await fetch(url);
  return {
    ok: response.ok,
    status: response.status,
    text: await response.text(),
  };
}

export async function fetchJson(url: string) {
  const response = await fetch(url);
  return await response.json();
}

export async function postEcho(url: string, body: string) {
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "text/plain",
      "x-convex": "1",
    },
    body,
  });

  return {
    status: response.status,
    text: await response.text(),
  };
}

export async function fetchInParallel(urls: string[]) {
  const responses = await Promise.all(urls.map((url) => fetch(url)));
  return Promise.all(responses.map((response) => response.text()));
}

export async function fetchFailure(url: string) {
  try {
    await fetch(url);
    return "unexpected-success";
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}
