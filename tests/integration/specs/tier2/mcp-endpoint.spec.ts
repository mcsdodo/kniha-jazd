/**
 * Tier 2: Read-only MCP endpoint (task 89)
 *
 * Covers: the built image serves /mcp, and tools/list returns exactly the
 * read-only tools. The tool results and the journey rules are covered by the
 * Rust tests (mcp/tests.rs, journeys/tests.rs), not here.
 */

import { waitForAppReady } from '../../utils/app';
import { mcpRequest } from '../../utils/mcp';

interface ToolsList {
  tools: { name: string; description: string }[];
}

const EXPECTED_TOOLS = ['list_journeys', 'list_trips', 'list_vehicles'];

describe('MCP endpoint', () => {
  beforeEach(async () => {
    await waitForAppReady();
  });

  it('tools/list returns exactly the read-only tools', async () => {
    const resp = await mcpRequest<ToolsList>('tools/list');

    expect(resp.error).toBeUndefined();
    const names = resp.result!.tools.map((t) => t.name).sort();
    expect(names).toEqual(EXPECTED_TOOLS);
  });
});
