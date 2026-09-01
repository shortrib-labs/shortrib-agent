use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex as SyncMutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use keycard_rmcp::{
    authorized_streamable_http_transport,
    rmcp::{
        RoleClient, ServiceExt,
        model::{CallToolRequestParams, JsonObject, Tool},
        service::{Peer, RunningService},
        transport::{
            AuthorizationRequest, auth::OAuthState,
            streamable_http_client::StreamableHttpClientTransportConfig,
        },
    },
};
use reqwest::Url;
use rig::{
    tool::server::ToolServerHandle,
    tool::{DynamicTool, ToolErrorKind, ToolExecutionError, ToolOutput},
};
use serde_json::json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, OnceCell, RwLock},
};

use crate::state::UserKey;

const DEFAULT_MCP_URL: &str =
    "https://google-calendar-mcp-hrlw48pl49z9x55u3a2kremvhe.mcp.gateway.context.cloud/mcp/v1";
const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:3000/oauth/callback";
const DEFAULT_CALLBACK_BIND: &str = "127.0.0.1:3000";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const AUTHORIZATION_TTL: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Config {
    mcp_url: String,
    redirect_uri: Url,
    callback_bind: SocketAddr,
}

impl Config {
    fn from_env() -> Result<Self> {
        Self::from_values(|name| std::env::var(name).ok())
    }

    fn from_values(get: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let mcp_url = get("GOOGLE_CALENDAR_MCP_URL").unwrap_or_else(|| DEFAULT_MCP_URL.to_owned());
        let redirect_uri = get("GOOGLE_CALENDAR_MCP_REDIRECT_URI")
            .unwrap_or_else(|| DEFAULT_REDIRECT_URI.to_owned())
            .parse::<Url>()
            .context("GOOGLE_CALENDAR_MCP_REDIRECT_URI must be an absolute URL")?;
        if redirect_uri.scheme() != "http" && redirect_uri.scheme() != "https" {
            bail!("GOOGLE_CALENDAR_MCP_REDIRECT_URI must use http or https");
        }

        let callback_bind = get("GOOGLE_CALENDAR_MCP_CALLBACK_BIND")
            .unwrap_or_else(|| DEFAULT_CALLBACK_BIND.to_owned())
            .parse()
            .context("GOOGLE_CALENDAR_MCP_CALLBACK_BIND must be an IP socket address")?;

        Ok(Self {
            mcp_url,
            redirect_uri,
            callback_bind,
        })
    }
}

/// Per-request context handed to the calendar tools through rig's
/// [`rig::tool::ToolContext`]: which user is asking, and a slot the tools
/// fill in when that user still has to authorize.
#[derive(Clone)]
pub(crate) struct CalendarSession {
    user: UserKey,
    authorization_url: Arc<SyncMutex<Option<String>>>,
}

impl CalendarSession {
    pub(crate) fn new(user: UserKey) -> Self {
        Self {
            user,
            authorization_url: Arc::new(SyncMutex::new(None)),
        }
    }

    /// The authorization link a tool asked to send to the user, if any.
    pub(crate) fn authorization_url(&self) -> Option<String> {
        self.authorization_url.lock().ok()?.clone()
    }

    fn request_authorization(&self, url: String) {
        if let Ok(mut slot) = self.authorization_url.lock() {
            *slot = Some(url);
        }
    }
}

struct PendingAuthorization {
    user: UserKey,
    authorization_url: String,
    oauth: OAuthState,
    created_at: Instant,
}

struct UserConnection {
    _client: RunningService<RoleClient, ()>,
    peer: Peer<RoleClient>,
}

pub(crate) struct GoogleCalendarMcp {
    config: Config,
    pending: Mutex<HashMap<String, PendingAuthorization>>,
    connections: RwLock<HashMap<UserKey, Arc<UserConnection>>>,
    /// The agent's tool server. Calendar tools are added to it once the
    /// first user authorizes; see [`Self::register_calendar_tools`].
    tools: ToolServerHandle,
    calendar_tools_registered: OnceCell<()>,
}

impl GoogleCalendarMcp {
    /// Start the service and register [`CONNECT_TOOL`] with `tools`.
    pub(crate) async fn from_env(tools: ToolServerHandle) -> Result<Arc<Self>> {
        let config = Config::from_env()?;
        let listener = TcpListener::bind(config.callback_bind)
            .await
            .with_context(|| {
                format!(
                    "failed to bind Google Calendar OAuth callback listener at {}",
                    config.callback_bind
                )
            })?;
        let service = Arc::new(Self {
            config,
            pending: Mutex::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            tools,
            calendar_tools_registered: OnceCell::new(),
        });
        service
            .tools
            .add_dynamic_tool(connect_tool(Arc::clone(&service)))
            .await;
        tokio::spawn(Arc::clone(&service).serve_callbacks(listener));

        Ok(service)
    }

    /// Discover the server's tools over `peer` and add them to the agent's
    /// tool server, once per process. The server only answers `tools/list`
    /// for a caller with a Google grant, so discovery rides on the first
    /// user to authorize; the resulting tools are user-agnostic and resolve
    /// the requesting user's connection from the [`CalendarSession`] in the
    /// tool context at call time. A failed attempt is retried on the next
    /// authorization.
    async fn register_calendar_tools(self: &Arc<Self>, peer: &Peer<RoleClient>) -> Result<()> {
        self.calendar_tools_registered
            .get_or_try_init(|| async {
                let tools = peer
                    .list_all_tools()
                    .await
                    .context("failed to discover Google Calendar tools")?;
                for tool in tools {
                    self.tools
                        .add_dynamic_tool(dynamic_tool(tool, Arc::clone(self)))
                        .await;
                }
                Ok(())
            })
            .await
            .map(|_| ())
    }

    /// The user's MCP connection, or the authorization URL they must visit
    /// before one can be established.
    async fn peer_for(&self, user: &UserKey) -> Result<Result<Peer<RoleClient>, String>> {
        if let Some(connection) = self.connections.read().await.get(user) {
            return Ok(Ok(connection.peer.clone()));
        }

        let mut pending = self.pending.lock().await;
        pending.retain(|_, authorization| authorization.created_at.elapsed() < AUTHORIZATION_TTL);
        if let Some(authorization) = pending.values().find(|pending| &pending.user == user) {
            return Ok(Err(authorization.authorization_url.clone()));
        }

        let mut oauth = OAuthState::new(&self.config.mcp_url, None)
            .await
            .context("failed to initialize Google Calendar OAuth discovery")?;
        oauth
            .start_authorization(
                AuthorizationRequest::new(self.config.redirect_uri.as_str())
                    .with_client_name("shortrib-agent")
                    .with_application_type("web"),
            )
            .await
            .context("failed to start Google Calendar OAuth authorization")?;
        let authorization_url = oauth
            .get_authorization_url()
            .await
            .context("failed to create Google Calendar authorization URL")?;
        let state = oauth_state(&authorization_url)?;

        pending.insert(
            state,
            PendingAuthorization {
                user: user.clone(),
                authorization_url: authorization_url.clone(),
                oauth,
                created_at: Instant::now(),
            },
        );
        Ok(Err(authorization_url))
    }

    async fn serve_callbacks(self: Arc<Self>, listener: TcpListener) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let service = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(error) = service.handle_callback_connection(stream).await {
                            tracing::warn!(error = %error, "Google Calendar OAuth callback failed");
                        }
                    });
                }
                Err(error) => {
                    tracing::error!(error = %error, "Google Calendar OAuth callback listener stopped");
                    return;
                }
            }
        }
    }

    async fn handle_callback_connection(self: &Arc<Self>, mut stream: TcpStream) -> Result<()> {
        let result =
            match tokio::time::timeout(CALLBACK_TIMEOUT, read_request_target(&mut stream)).await {
                Ok(target) => target.and_then(|target| self.callback_url(&target)),
                Err(_) => Err(anyhow!("OAuth callback request timed out")),
            };

        let response = match result {
            Ok(callback_url) => match self.complete_authorization(&callback_url).await {
                Ok(()) => http_response(
                    "200 OK",
                    "Google Calendar is connected. You can return to Slack and send your message again.",
                ),
                Err(error) => {
                    tracing::warn!(error = %error, "Google Calendar authorization could not be completed");
                    http_response(
                        "400 Bad Request",
                        "Google Calendar authorization failed. Return to Slack and try again.",
                    )
                }
            },
            Err(error) => {
                tracing::warn!(error = %error, "invalid Google Calendar OAuth callback request");
                http_response("400 Bad Request", "Invalid OAuth callback request.")
            }
        };

        stream.write_all(response.as_bytes()).await?;
        stream.shutdown().await?;
        Ok(())
    }

    fn callback_url(&self, request_target: &str) -> Result<String> {
        let request_url = Url::parse(&format!("http://callback{request_target}"))
            .context("callback request target is not a valid URL")?;
        if request_url.path() != self.config.redirect_uri.path() {
            bail!("callback request path does not match configured redirect URI");
        }

        let mut callback_url = self.config.redirect_uri.clone();
        callback_url.set_query(request_url.query());
        Ok(callback_url.into())
    }

    async fn complete_authorization(self: &Arc<Self>, callback_url: &str) -> Result<()> {
        let state = callback_state(callback_url)?;
        let Some(mut pending) = self.pending.lock().await.remove(&state) else {
            bail!("OAuth callback state is unknown or expired");
        };

        pending
            .oauth
            .handle_callback_url(callback_url)
            .await
            .context("OAuth code exchange failed")?;
        let manager = pending
            .oauth
            .into_authorization_manager()
            .ok_or_else(|| anyhow!("OAuth authorization did not produce a manager"))?;
        let transport = authorized_streamable_http_transport(
            manager,
            StreamableHttpClientTransportConfig::with_uri(self.config.mcp_url.as_str()),
        );
        let client = ().serve(transport).await.context("MCP handshake failed")?;
        let peer = client.peer().clone();

        self.connections.write().await.insert(
            pending.user,
            Arc::new(UserConnection {
                _client: client,
                peer: peer.clone(),
            }),
        );
        if let Err(error) = self.register_calendar_tools(&peer).await {
            tracing::warn!(error = %error, "Google Calendar tools could not be registered");
        }
        Ok(())
    }
}

/// Name of the tool the model calls to start a user's authorization.
pub(crate) const CONNECT_TOOL: &str = "connect_google_calendar";

/// The one tool that is always registered: it lets the model request a
/// user's Google Calendar authorization before the calendar tools exist,
/// and reports "already connected" afterwards.
fn connect_tool(service: Arc<GoogleCalendarMcp>) -> DynamicTool {
    DynamicTool::new(
        CONNECT_TOOL,
        "Connect the user's Google Calendar. Call this when the user asks about their calendar \
         and no calendar tools are available, or a calendar tool reports the user is not \
         connected. It sends the user a private authorization link.",
        json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        move |context, _arguments| {
            let service = Arc::clone(&service);
            let session = context.get::<CalendarSession>().cloned();
            Box::pin(async move {
                let session = session.ok_or_else(|| {
                    ToolExecutionError::new(
                        ToolErrorKind::Other,
                        "Google Calendar tools require a CalendarSession in the tool context",
                    )
                })?;
                match service.peer_for(&session.user).await {
                    Ok(Ok(peer)) => {
                        service
                            .register_calendar_tools(&peer)
                            .await
                            .map_err(|error| {
                                tracing::warn!(
                                    error = %error,
                                    "Google Calendar tools could not be registered"
                                );
                                ToolExecutionError::new(
                                    ToolErrorKind::Provider,
                                    "Google Calendar tools could not be loaded",
                                )
                            })?;
                        Ok(ToolOutput::text(
                            "Google Calendar is connected and its tools are available.",
                        ))
                    }
                    Ok(Err(authorization_url)) => {
                        session.request_authorization(authorization_url);
                        Ok(ToolOutput::text(
                            "An authorization link has been sent to the user privately. Ask them \
                             to open it, then repeat their request once they have authorized.",
                        ))
                    }
                    Err(error) => Err(ToolExecutionError::new(
                        ToolErrorKind::Provider,
                        format!("Google Calendar authorization could not be started: {error:#}"),
                    )),
                }
            })
        },
    )
}

fn dynamic_tool(tool: Tool, service: Arc<GoogleCalendarMcp>) -> DynamicTool {
    let name = tool.name.to_string();
    let call_name = name.clone();
    let parameters = tool.schema_as_json_value();
    let description = tool.description.unwrap_or_default().to_string();

    DynamicTool::new(name, description, parameters, move |context, arguments| {
        let service = Arc::clone(&service);
        let call_name = call_name.clone();
        let session = context.get::<CalendarSession>().cloned();
        Box::pin(async move {
            let Some(session) = session else {
                return Err(ToolExecutionError::new(
                    ToolErrorKind::Other,
                    "Google Calendar tools require a CalendarSession in the tool context",
                ));
            };
            let peer = match service.peer_for(&session.user).await {
                Ok(Ok(peer)) => peer,
                Ok(Err(authorization_url)) => {
                    session.request_authorization(authorization_url);
                    return Err(ToolExecutionError::new(
                        ToolErrorKind::PermissionDenied,
                        "Google Calendar is not connected for this user",
                    )
                    .with_model_feedback(
                        "The user has not connected their Google Calendar yet. An authorization \
                         link has been sent to them privately. Ask them to open it, then repeat \
                         their request once they have authorized.",
                    ));
                }
                Err(error) => {
                    return Err(ToolExecutionError::new(
                        ToolErrorKind::Provider,
                        format!("Google Calendar authorization could not be started: {error:#}"),
                    ));
                }
            };
            let arguments: JsonObject = serde_json::from_value(arguments).map_err(|error| {
                ToolExecutionError::new(
                    ToolErrorKind::InvalidArgs,
                    format!("Google Calendar tool arguments must be a JSON object: {error}"),
                )
                .with_source(error)
            })?;
            let result = peer
                .call_tool(CallToolRequestParams::new(call_name).with_arguments(arguments))
                .await
                .map_err(|error| {
                    ToolExecutionError::new(
                        ToolErrorKind::Provider,
                        "Google Calendar MCP request failed",
                    )
                    .with_source(error)
                })?;
            let output = ToolOutput::json(
                serde_json::to_value(&result).map_err(ToolExecutionError::from_error)?,
            );

            if result.is_error == Some(true) {
                Err(ToolExecutionError::new(
                    ToolErrorKind::Provider,
                    "Google Calendar reported a tool execution error",
                )
                .with_model_output(output))
            } else {
                Ok(output)
            }
        })
    })
}

fn oauth_state(authorization_url: &str) -> Result<String> {
    query_parameter(authorization_url, "state")
        .ok_or_else(|| anyhow!("authorization URL did not contain OAuth state"))
}

fn callback_state(callback_url: &str) -> Result<String> {
    query_parameter(callback_url, "state")
        .ok_or_else(|| anyhow!("OAuth callback did not contain state"))
}

fn query_parameter(url: &str, name: &str) -> Option<String> {
    Url::parse(url)
        .ok()?
        .query_pairs()
        .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
}

async fn read_request_target(stream: &mut TcpStream) -> Result<String> {
    let mut request = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            bail!("OAuth callback connection closed before sending a request");
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if request.len() > 8192 {
            bail!("OAuth callback request headers are too large");
        }
    }

    let request = std::str::from_utf8(&request).context("OAuth callback request is not UTF-8")?;
    let Some(line) = request.lines().next() else {
        bail!("OAuth callback request is empty");
    };
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        bail!("OAuth callback must use GET");
    }
    let target = parts
        .next()
        .ok_or_else(|| anyhow!("OAuth callback request target is missing"))?;
    Ok(target.to_owned())
}

fn http_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_uses_documented_defaults() {
        let config = Config::from_values(|_| None).unwrap();

        assert_eq!(config.mcp_url, DEFAULT_MCP_URL);
        assert_eq!(config.redirect_uri.as_str(), DEFAULT_REDIRECT_URI);
        assert_eq!(config.callback_bind, DEFAULT_CALLBACK_BIND.parse().unwrap());
    }

    #[test]
    fn callback_rebuilds_the_registered_redirect_uri() {
        let config = Config {
            mcp_url: DEFAULT_MCP_URL.to_owned(),
            redirect_uri: "https://agent.example/oauth/callback".parse().unwrap(),
            callback_bind: DEFAULT_CALLBACK_BIND.parse().unwrap(),
        };
        let service = GoogleCalendarMcp {
            config,
            pending: Mutex::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            tools: rig::tool::server::ToolServer::new().run(),
            calendar_tools_registered: OnceCell::new(),
        };

        assert_eq!(
            service
                .callback_url("/oauth/callback?code=abc&state=xyz")
                .unwrap(),
            "https://agent.example/oauth/callback?code=abc&state=xyz"
        );
        assert!(service.callback_url("/wrong?state=xyz").is_err());
    }

    #[test]
    fn state_is_extracted_without_logging_callback_secrets() {
        assert_eq!(
            callback_state("https://agent.example/callback?code=secret&state=user-state").unwrap(),
            "user-state"
        );
    }
}
