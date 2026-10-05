/**
 * Raw JSON-RPC calls to the read-only MCP endpoint (task 89).
 *
 * The endpoint is stateless, so no initialize call and no session ID are
 * needed. It answers with application/json.
 */

const SERVER_URL = process.env.WDIO_SERVER_URL || 'http://localhost:3457';

export interface McpResponse<T> {
  jsonrpc: '2.0';
  id: number;
  result?: T;
  error?: { code: number; message: string };
}

export async function mcpRequest<T>(
  method: string,
  params: Record<string, unknown> = {}
): Promise<McpResponse<T>> {
  const resp = await fetch(`${SERVER_URL}/mcp`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json, text/event-stream',
    },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  });
  if (!resp.ok) {
    throw new Error(`MCP '${method}' failed (${resp.status}): ${await resp.text()}`);
  }
  return (await resp.json()) as McpResponse<T>;
}
