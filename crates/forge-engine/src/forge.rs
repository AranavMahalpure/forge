use crate::model::{Context, Message, State};
use crate::parser::{PromptParser, Token};
use crate::{error::Result, model::Event};
use derive_setters::Setters;
use forge_provider::{Provider, Stream};
use forge_tool::Tool;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;

pub struct CodeForge {
    state: Arc<Mutex<State>>,
    tools: Vec<Rc<dyn Tool>>,
    provider: Provider,
}

#[derive(Setters, Clone, Debug)]
pub struct Prompt {
    message: String,
    files: Vec<File>,
}

#[derive(Setters, Clone,Debug)]
pub struct File {
    name: String,
    content: String,
}

impl File {
    fn new(name: String, content: String) -> Self {
        Self { name, content }
    }

    fn read(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;
        let name = path.display().to_string();
        Ok(Self::new(name, content))
    }
}

impl Prompt {
    pub fn new(message: String) -> Self {
        let mut prompt = Self {
            message: message.clone(),
            files: Vec::new(),
        };

        // Parse message to extract file paths
        let tokens = PromptParser::parse(message);
        for token in tokens {
            if let Token::FilePath(path) = token {
                if let Ok(file) = File::read(path.clone()) {
                    prompt.add_file(file);
                } else {
                    // TODO: raise error here.
                    eprintln!("Failed to read file: {}", path.display());
                }
            }
        }

        prompt
    }

    pub fn add_file(&mut self, file: File) {
        self.files.push(file);
    }
}

impl CodeForge {
    pub fn new(key: String) -> Self {
        // Add initial set of tools
        let tools = vec![
            Rc::new(forge_tool::FS) as Rc<dyn Tool>,
            Rc::new(forge_tool::Think::default()) as Rc<dyn Tool>,
        ];

        CodeForge {
            state: Arc::new(Mutex::new(State::default())),
            // TODO: add fs and think
            tools,

            // TODO: make the provider configurable
            provider: Provider::open_router(key, None, None),
        }
    }

    pub fn add_tool<T: Tool + Sync + 'static>(&mut self, tool: T) {
        self.tools.push(Rc::new(tool));
    }

    pub async fn chat(&self, prompt: Prompt) -> Result<Stream<Event>> {
        // - Create Request, update context
        //   -  Add System Message
        //   -  Add Add all tools
        //   -  Add User Message
        //   -  Add Context Files
        // - Send message to LLM and await response #001
        // - On Response, dispatch event
        // - Check response has tool_use
        // - Execute tool
        // - Dispatch Event
        // - Add tool response to context
        // - Goto #001

        // let (tx, rx) = tokio::sync::mpsc::channel(1);

        // TODO: add message to history

        let context = Context::new(Message::system(include_str!("./prompt.md").to_string()))
            .tools(self.tools.clone())
            .add_message(Message::user(prompt.message))
            .files(prompt.files);

        let stream = self.provider.chat(context.into()).await?;

        Ok(Box::new(stream.map(|message| match message {
            Ok(message) => Event::Text(message),
            Err(error) => Event::Error(format!("{}", error)),
        })))
    }

    pub fn model(self, model: String) -> Self {
        // TODO: update the provider to use the passed model
        self
    }

    /// Returns an autocomplete for a prompt containing '@'
    pub async fn files(&self) -> Result<Vec<String>> {
        todo!()
    }

    pub async fn models(&self) -> Result<Vec<String>> {
        Ok(self.provider.models().await?)
    }

    /// Resets the state of the forge without changing the model
    pub fn reset(self) -> Self {
        todo!()
    }
}
