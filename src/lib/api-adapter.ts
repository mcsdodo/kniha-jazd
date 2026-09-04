export async function apiCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const response = await fetch('/api/rpc', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-KJ-Client': '1',
    },
    body: JSON.stringify({ command, args: args ?? {} }),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text);
  }
  return response.json();
}
