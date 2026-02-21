use agent_client_protocol as acp;
use claco_sdk::{ClacoResponse, ClacoSession};
use futures_util::StreamExt;
use serde_json::json;
use std::cell::Cell;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use std::cell::RefCell;
use std::collections::HashMap;
use tokio::sync::OnceCell;

macro_rules! log_file {
    ($($arg:tt)*) => {{
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/claco-agent.log") {
            let _ = writeln!(file, $($arg)*);
        }
    }};
}

struct ClacoAgent {
    next_session_id: Cell<u32>,
    session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    sessions: RefCell<HashMap<String, Arc<OnceCell<Arc<ClacoSession>>>>>,
}

impl ClacoAgent {
    fn new(
        session_update_tx: mpsc::UnboundedSender<(acp::SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        log_file!("ClacoAgent started!");
        Self {
            next_session_id: Cell::new(1),
            session_update_tx,
            sessions: RefCell::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for ClacoAgent {
    async fn initialize(
        &self,
        _arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        log_file!("Called initialize");
        Ok(acp::InitializeResponse::new(acp::ProtocolVersion::V1))
    }

    async fn authenticate(
        &self,
        _arguments: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        log_file!("Called authenticate");
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        log_file!("Called new_session!");
        let session_id = self.next_session_id.get();
        self.next_session_id.set(session_id + 1);
        let id_str = session_id.to_string();

        let cell = Arc::new(OnceCell::new());

        self.sessions
            .borrow_mut()
            .insert(id_str.clone(), cell.clone());

        let cwd = Some(arguments.cwd);

        tokio::task::spawn_local(async move {
            let _ = cell
                .get_or_try_init(|| async move {
                    log_file!("Starting background spawn...");
                    ClacoSession::spawn(cwd).await.map(Arc::new).map_err(|e| {
                        log_file!("Failed to spawn claco session: {e}");
                        acp::Error::internal_error()
                    })
                })
                .await;
            log_file!("Background spawn finished!");
        });

        log_file!("Successfully created new session {}", id_str);
        Ok(acp::NewSessionResponse::new(acp::SessionId::new(id_str)))
    }

    async fn load_session(
        &self,
        _arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        Ok(acp::LoadSessionResponse::default())
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let session_id_str = arguments.session_id.to_string();
        log_file!("Handling prompt for session {:?}", session_id_str);

        let cell = {
            let sessions = self.sessions.borrow();
            sessions.get(&session_id_str).cloned().ok_or_else(|| {
                log_file!("Session not found: {}", session_id_str);
                acp::Error::invalid_params()
            })?
        };

        log_file!("Waiting for session to be fully spawned...");
        let session = cell
            .get_or_try_init(|| async {
                log_file!("Starting lazy spawn in prompt...");
                ClacoSession::spawn(None).await.map(Arc::new).map_err(|e| {
                    log_file!("Failed to spawn claco session: {e}");
                    acp::Error::internal_error()
                })
            })
            .await
            .map_err(|_| acp::Error::internal_error())?
            .clone();

        let tx = self.session_update_tx.clone();
        let session_id = arguments.session_id.clone();
        let prompt_text = arguments
            .prompt
            .into_iter()
            .filter_map(|c| match c {
                acp::ContentBlock::Text(t) => Some(t.text),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        log_file!("Sending prompt text: {}", prompt_text);
        let stream = match session.send(&prompt_text).await {
            Ok(s) => s,
            Err(e) => {
                log_file!("Failed to send prompt: {e}");
                return Err(acp::Error::internal_error());
            }
        };

        tokio::pin!(stream);

        while let Some(resp) = stream.next().await {
            log_file!("Agent response chunk");
            let update = match resp {
                ClacoResponse::Text(text) => {
                    acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(text.into()))
                }
                ClacoResponse::ToolCall { name, args } => acp::SessionUpdate::ToolCall(
                    acp::ToolCall::new(name.clone(), name.clone()).raw_input(args),
                ),
            };

            let (ack_tx, ack_rx) = oneshot::channel();
            if tx
                .send((
                    acp::SessionNotification::new(session_id.clone(), update),
                    ack_tx,
                ))
                .is_err()
            {
                log_file!("Failed to send session notification to client");
                break;
            }
            if let Err(e) = ack_rx.await {
                log_file!("Failed waiting for ack: {e}");
                break;
            }
        }

        log_file!("Finished Stream for session: {}", session_id_str);
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, _args: acp::CancelNotification) -> Result<(), acp::Error> {
        Ok(())
    }

    async fn set_session_mode(
        &self,
        _args: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        Ok(acp::SetSessionModeResponse::default())
    }

    async fn set_session_config_option(
        &self,
        _args: acp::SetSessionConfigOptionRequest,
    ) -> Result<acp::SetSessionConfigOptionResponse, acp::Error> {
        Ok(acp::SetSessionConfigOptionResponse::new(vec![]))
    }

    async fn ext_method(&self, _args: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        let val: Arc<serde_json::value::RawValue> =
            Arc::from(serde_json::value::to_raw_value(&json!({}))?);
        Ok(val.into())
    }

    async fn ext_notification(&self, _args: acp::ExtNotification) -> Result<(), acp::Error> {
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> acp::Result<()> {
    env_logger::init();

    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = mpsc::unbounded_channel();

            let (conn, handle_io) =
                acp::AgentSideConnection::new(ClacoAgent::new(tx), outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = rx.recv().await {
                    use acp::Client;
                    let result = conn.session_notification(session_notification).await;
                    if let Err(e) = result {
                        log::error!("{e}");
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            let res = handle_io.await;
            log_file!("handle_io finished with result: {:?}", res);
            res
        })
        .await
}
