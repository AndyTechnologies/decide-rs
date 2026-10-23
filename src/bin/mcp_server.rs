//! MCP server for decide-rs.
//! Provides decision-making capabilities via the Model Context Protocol (MCP):
//! loads the Julia-1 engine once and serves requests over stdio transport.
//! Decision logic lives in `decide_rs::mcp`; this binary only maps results
//! and errors into rmcp types.

use anyhow::{Context, Result};
use decide_rs::adapter::Adapter;
use decide_rs::load_engine;
use decide_rs::mcp;
use rmcp::{
    ErrorData as McpError,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::{MaybeSendFuture, RequestContext, RoleServer},
    transport::io,
};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// State held by the MCP server: the adapter (which contains the engine).
struct McpServerState {
    adapter: Adapter,
}

impl McpServerState {
    /// Create a new McpServerState by loading the engine from the given path and building an adapter.
    fn new(model_path: &Path) -> Result<Self> {
        let engine = load_engine(model_path)
            .with_context(|| format!("Failed to load engine from {}", model_path.display()))?;
        Ok(Self {
            adapter: Adapter::new_from_engine(engine),
        })
    }
}

/// Map a tool error into an rmcp protocol error.
fn tool_error(err: mcp::ToolError) -> McpError {
    match err {
        mcp::ToolError::UnknownTool(name) => {
            McpError::invalid_params(format!("unknown tool \"{name}\""), None)
        }
        mcp::ToolError::Schema(errors) => {
            let details: Vec<Value> = errors
                .iter()
                .map(|e| serde_json::json!({"path": e.path, "message": e.message}))
                .collect();
            McpError::invalid_params(
                "request rejected before inference",
                Some(serde_json::json!({ "errors": details })),
            )
        }
        mcp::ToolError::Engine(src) => McpError::internal_error(format!("{src:#}"), None),
    }
}

/// The MCP server that holds our state and implements the ServerHandler trait.
struct McpServer {
    state: Arc<McpServerState>,
}

impl ServerHandler for McpServer {
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResponse, McpError>> + MaybeSendFuture + '_ {
        let name = request.name.clone();
        let args = request.arguments.map(Value::Object);
        let adapter = &self.state.adapter;

        async move {
            let answers = mcp::run_tool(
                adapter,
                name.as_ref(),
                args.as_ref().unwrap_or(&Value::Null),
            )
            .map_err(tool_error)?;
            // structuredContent carries the same JSON; text content keeps it
            // readable for clients that render text only.
            Ok(CallToolResponse::Complete(CallToolResult::structured(
                answers,
            )))
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        async move {
            let tools = mcp::tool_specs()
                .into_iter()
                .map(|(name, description, schema)| {
                    let map = schema.as_object().cloned().expect("tool schema is an object");
                    Tool::new(name, description, map)
                })
                .collect();
            Ok(ListToolsResult {
                tools,
                cache_scope: None,
                meta: None,
                next_cursor: None,
                result_type: None,
                ttl_ms: None,
            })
        }
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("decide-rs MCP server: call decide/choice, decide/noul, or decide/score with state and questions.")
    }
}

fn main() -> Result<()> {
    // Model resolution order: DECIDE_RS_MODEL_PATH, the installed
    // ~/.local/share/decide-rs/julia-checkpoint, then ./julia-checkpoint.
    let model_path = decide_rs::resolve_model_path();

    eprintln!("Loading model from: {}", model_path.display());

    let state = Arc::new(McpServerState::new(&model_path)
        .with_context(|| "Failed to initialize MCP server state")?);

    // Create the handler.
    let handler = McpServer { state };

    // Serve the handler over stdio transport.
    let (read_stream, write_stream) = io::stdio();
    let transport = (read_stream, write_stream);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        // serve_server resolves once initialization completes; `waiting()` keeps
        // the process alive until the transport closes (stdin EOF / Ctrl+C).
        let service = rmcp::serve_server(handler, transport).await?;
        service
            .waiting()
            .await
            .map_err(|e| anyhow::anyhow!(e))
            .map(|_| ())?;
        anyhow::Ok(())
    })?;

    Ok(())
}