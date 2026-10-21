/**
 * pi-decide extension
 *
 * Registers the decide-rs MCP server and adds system instructions so the agent
 * evaluates alternatives with decide/choice, decide/noul, decide/score before
 * posing decision questions to the user.
 *
 * The decide-rs binary must be available in PATH or at a known location.
 * Set DECIDE_RS_MODEL_PATH to point to the Julia-1 checkpoint directory.
 */

import type { ExtensionAPI, BeforeAgentStartEvent, NormalizedBuildSystemPromptOptions } from "@earendil-works/pi-coding-agent";

/** Name of the MCP server as registered in Pi. */
const MCP_SERVER_NAME = "decide-rs";

/** Default path to the decide-rs-mcp binary. */
const DEFAULT_BINARY = "decide-rs-mcp";

/** System prompt addition for decision evaluation guidance. */
const DECIDE_SYSTEM_INSTRUCTION = `
## Decision Evaluation (decide-rs)

When the user asks you to make a decision between alternatives, or when you need to evaluate options before proposing a course of action, use the **decide-rs** MCP tools instead of asking the user directly.

Available tools (namespace: \`mcp__decide_rs\`):
- \`mcp__decide_rs__decide/choice\` — Categorical choice between named options (e.g., which team, which approach, which priority)
- \`mcp__decide_rs__decide/noul\` — Yes/No decisions (e.g., is this urgent? should we proceed?)
- \`mcp__decide_rs__decide/score\` — Rubric-based scoring (e.g., frustration level, confidence, risk rating)

### When to use
- User asks "Should I do X or Y?" → use \`decide/choice\`
- User asks "Is this urgent/important/ready?" → use \`decide/noul\`
- User asks "How frustrated/risky/confident is this?" → use \`decide/score\`
- Before proposing a decision question to the user, evaluate the alternatives yourself

### How to call
Each tool takes:
- \`state\`: A string describing the situation/context (e.g., ticket summary, problem description)
- \`questions\`: An object with one question:
  - \`choice\`: { type: "choice", instructions: "...", criteria: { optionId: "description", ... } }
  - \`noul\`: { type: "noul", instructions: "...", criteria: { false: "label", true: "label" } }
  - \`score\`: { type: "score", instructions: "...", criteria: ["level1", "level2", ...] }

### Example
\`\`\`typescript
await mcp__decide_rs__decide/choice({
  state: "Ticket #4821: checkout payments failing after release. Revenue impact.",
  questions: {
    route: {
      type: "choice",
      instructions: "Which team should handle this?",
      criteria: {
        billing: "Payments, invoices, refunds",
        platform: "Checkout, API, platform outages",
        account: "Account access, identity, permissions"
      }
    }
  }
});
\`\`\`

The tool returns probabilities and a recommended choice. Present the result to the user with the model's reasoning, then ask for confirmation if needed.
`.trim();

/** Determine the path to the decide-rs-mcp binary. */
function resolveBinaryPath(): string {
  // In production, the binary is installed alongside pi or in PATH
  // During development, it might be in the project's target/release
  const envPath = process.env.DECIDE_RS_MCP_BINARY;
  if (envPath) return envPath;

  // Check if running from the decide-rs project directory
  const projectRoot = process.env.DECIDE_RS_PROJECT_ROOT;
  if (projectRoot) {
    const candidate = `${projectRoot}/target/release/decide-rs-mcp`;
    return candidate;
  }

  return DEFAULT_BINARY;
}

/** Determine the model checkpoint path. */
/** Determine the model checkpoint path (mirrors the binary's resolution: env var, then the installed location). */
function resolveModelPath(): string {
  const envPath = process.env.DECIDE_RS_MODEL_PATH;
  if (envPath) return envPath;
  const xdgDataHome = process.env.XDG_DATA_HOME;
  const dataHome = xdgDataHome ?? `${process.env.HOME ?? ""}/.local/share`;
  return `${dataHome}/decide-rs/julia-checkpoint`;
}

export default function (pi: ExtensionAPI) {
  // Register the MCP server on extension load
  const binaryPath = resolveBinaryPath();
  const modelPath = resolveModelPath();

  pi.registerMcpServer(MCP_SERVER_NAME, {
    type: "stdio",
    command: binaryPath,
    args: [],
    env: {
      DECIDE_RS_MODEL_PATH: modelPath,
    },
    exposure: "direct",
    description: "Local Julia-1 decision engine: choice, noul (yes/no), and score evaluations",
    enabled: true,
    timeout: 30,
  });

  // Add system instructions at the start of every agent run
  pi.on("before_agent_start", (event: BeforeAgentStartEvent) => {
    // Only add instructions for user-initiated runs, not custom/internal ones
    if (event.messageType !== "user") return;

    const options = event.systemPromptOptions as NormalizedBuildSystemPromptOptions;

    // Avoid duplicate registration
    const alreadyHasDecide = options.promptGuidelines?.some(
      (g: string) => g.includes("Decision Evaluation") || g.includes("decide-rs")
    );
    if (alreadyHasDecide) return;

    // Mutate the prompt guidelines in place
    options.promptGuidelines = [
      ...(options.promptGuidelines ?? []),
      DECIDE_SYSTEM_INSTRUCTION,
    ];

    // Return undefined to indicate no message override, the mutated options take effect
    return undefined;
  });

  // Log on session start (no context available for this event)
  pi.on("session_start", () => {
    // eslint-disable-next-line no-console
    console.info(`[pi-decide] Registered MCP server "${MCP_SERVER_NAME}" (${binaryPath}) with model at ${modelPath}`);
  });
}