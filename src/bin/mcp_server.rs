#!/usr/bin/env rustc
//! MCP server for decide-rs.
//! This server provides decision-making capabilities via the Model Context Protocol (MCP).
//! It loads the Julia-1 engine once and serves requests over stdio transport.

use anyhow::{Context, Result};
use decide_rs::adapter::Adapter;
use decide_rs::load_engine;
use rmcp::{
    ErrorData as McpError,
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerConfig, TextContent, Tool,
    },
    service::{MaybeSendFuture, RequestContext, RoleServer},
    transport::io,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, Map};
use std::path::Path;
use std::sync::Arc;

/// Arguments for the decide/choice tool.
#[derive(Debug, Serialize, Deserialize)]
struct ChoiceArgs {
    state: String,
    questions: Value,
}

/// Arguments for the decide/noul tool.
#[derive(Debug, Serialize, Deserialize)]
struct NoulArgs {
    state: String,
    questions: Value,
}

/// Arguments for the decide/score tool.
#[derive(Debug, Serialize, Deserialize)]
struct ScoreArgs {
    state: String,
    questions: Value,
}

/// State held by the MCP server: the adapter (which contains the engine).
struct McpServerState {
    adapter: Adapter,
}

impl McpServerState {
    /// Create a new McpServerState by loading the engine from the given path and building an adapter.
    fn new(model_path: &Path) -> Result<Self> {
        let engine = load_engine(model_path)
            .with_context(|| format!("Failed to load engine from {}", model_path.display()))?;
        let adapter = Adapter::new_from_engine(engine);
        Ok(Self { adapter })
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
        let args = request.arguments.unwrap_or_default(); // args: Map<String, Value>

        async move {
            let result = match name.as_ref() {
                "decide/choice" => {
                    let args_value = Value::Object(args.clone());
                    let args: ChoiceArgs = serde_json::from_value(args_value)
                        .map_err(|e| McpError::invalid_params(format!("Failed to parse args: {}", e), None))?;
                    let state_str = args.state.as_str();
                    let answers = self
                        .state
                        .adapter
                        .run(state_str, &args.questions)
                        .map_err(|e| McpError::internal_error(format!("Adapter error: {}", e), None))?;
                    let json = self.state.adapter.answers_to_json(&answers);
                    let text = serde_json::to_string(&json)
                        .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
                    Ok(CallToolResponse::Complete(CallToolResult::success(vec![
                        ContentBlock::Text(TextContent::new(text)),
                    ])))
                }
                "decide/noul" => {
                    let args_value = Value::Object(args.clone());
                    let args: NoulArgs = serde_json::from_value(args_value)
                        .map_err(|e| McpError::invalid_params(format!("Failed to parse args: {}", e), None))?;
                    let state_str = args.state.as_str();
                    let answers = self
                        .state
                        .adapter
                        .run(state_str, &args.questions)
                        .map_err(|e| McpError::internal_error(format!("Adapter error: {}", e), None))?;
                    let json = self.state.adapter.answers_to_json(&answers);
                    let text = serde_json::to_string(&json)
                        .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
                    Ok(CallToolResponse::Complete(CallToolResult::success(vec![
                        ContentBlock::Text(TextContent::new(text)),
                    ])))
                }
                "decide/score" => {
                    let args_value = Value::Object(args.clone());
                    let args: ScoreArgs = serde_json::from_value(args_value)
                        .map_err(|e| McpError::invalid_params(format!("Failed to parse args: {}", e), None))?;
                    let state_str = args.state.as_str();
                    let answers = self
                        .state
                        .adapter
                        .run(state_str, &args.questions)
                        .map_err(|e| McpError::internal_error(format!("Adapter error: {}", e), None))?;
                    let json = self.state.adapter.answers_to_json(&answers);
                    let text = serde_json::to_string(&json)
                        .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
                    Ok(CallToolResponse::Complete(CallToolResult::success(vec![
                        ContentBlock::Text(TextContent::new(text)),
                    ])))
                }
                _ => Err(McpError::method_not_found::<rmcp::model::CallToolRequestMethod>()),
            };

            result
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        async move {
            let mut schema_map = Map::new();
            schema_map.insert("type".to_string(), Value::String("object".to_string()));
            let mut props = Map::new();
            props.insert("state".to_string(), Value::String("string".to_string()));
            props.insert("questions".to_string(), Value::String("object".to_string()));
            schema_map.insert("properties".to_string(), Value::Object(props));
            schema_map.insert("required".to_string(), Value::Array(vec![
                Value::String("state".to_string()),
                Value::String("questions".to_string()),
            ]));
            let schema: Arc<Map<String, Value>> = Arc::new(schema_map);

            let tools = vec![
                Tool::new(
                    "decide/choice",
                    "Run a choice decision",
                    schema.clone(),
                ),
                Tool::new(
                    "decide/noul",
                    "Run a noul (yes/no) decision",
                    schema.clone(),
                ),
                Tool::new(
                    "decide/score",
                    "Run a score decision",
                    schema,
                ),
            ];

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