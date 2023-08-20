use std::env;

use serde_json::{json, Value};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Spellesp {
    client: Client,
}

const ADD_TO_WORD_LIST_COMMAND: &str = "spellesp.ignore_spelling";

#[tower_lsp::async_trait]
impl LanguageServer for Spellesp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![ADD_TO_WORD_LIST_COMMAND.to_string()],
                    work_done_progress_options: Default::default(),
                }),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::ERROR, "spellesp initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(
                MessageType::ERROR,
                format!(
                    "spellesp command executed! {} {:?}",
                    params.command, params.arguments
                ),
            )
            .await;

        if params.command.as_str() == ADD_TO_WORD_LIST_COMMAND {
            let mut file_path = env::current_dir().unwrap();
            file_path.push(".cspell.json");

            let contents = std::fs::read_to_string(&file_path).unwrap_or_else(|_| "{}".to_string());

            // Parse the JSON contents
            let mut json_value: Value = match serde_json::from_str(&contents) {
                Ok(value) => value,
                Err(err) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("spellesp json decoding error {:?}", err),
                        )
                        .await;
                    return Ok(None);
                }
            };

            // Access or add the "words" vector in the JSON and add "abc" to it
            if let Some(config_object) = json_value.as_object_mut() {
                let words_array = config_object.entry("words").or_insert_with(|| json!([])); // Add empty array if "words" doesn't exist

                if let Some(words) = words_array.as_array_mut() {
                    words.push(json!(params.arguments.get(0)));
                }
            }

            // Convert the modified JSON back to a string
            let modified_json = match serde_json::to_string_pretty(&json_value) {
                Ok(value) => value,
                Err(err) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("spellesp json encoding error {:?}", err),
                        )
                        .await;
                    return Ok(None);
                }
            };

            if let Err(err) = std::fs::write(file_path, modified_json) {
                self.client
                    .log_message(MessageType::ERROR, format!("spellesp error {:?}", err))
                    .await;
                return Ok(None);
            }
        }

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        self.client
            .log_message(MessageType::ERROR, "spellesp code actions!")
            .await;

        let diagnostic_message_pattern =
            regex::Regex::new(r"Unknown word \((?P<word>\w+)\)").unwrap();

        let actions = params
            .context
            .diagnostics
            .into_iter()
            .filter_map(|diagnostic| {
                if let Some(captures) = diagnostic_message_pattern.captures(&diagnostic.message) {
                    let unknown_word = captures.name("word").unwrap().as_str();

                    Some(vec![CodeActionOrCommand::Command(Command {
                        title: format!("Add \"{}\" to Dictionary", unknown_word),
                        command: "spellesp.ignore_spelling".to_string(),
                        arguments: Some(vec![unknown_word.into()]),
                    })])
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        return Ok(Some(actions));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());

    let (service, socket) = LspService::new(|client| Spellesp { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
