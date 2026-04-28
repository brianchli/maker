//! A concrete representation of a valid prompt that will
//! resolve to a valid Ollama request

use std::fmt::Write;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error, Visitor},
};
use std::fmt::Display;
use tower::BoxError;

#[derive(Serialize, Deserialize)]
pub struct TomlSpec {
    model: Option<String>,
    system: Option<System>,
    think: Option<Boolish>,
    context: TaskContext,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "filetype", rename_all = "lowercase")]
pub enum Filetype {
    Make { content: String },
    Cmake { content: String },
    Readme { content: String },
    Docker { content: String },
    Spec { content: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct System {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    prompt: String,
    constraints: Vec<String>,
}

type Options = System;
#[derive(Debug, Deserialize, Serialize)]
pub struct ResolvedPrompt {
    pub(crate) model: Option<String>,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    stream: bool,
    think: Boolish,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_alive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Options>,
}

#[derive(Debug, Deserialize, Serialize)]
enum Boolish {
    Low,
    Medium,
    High,
    #[serde(untagged)]
    Bool(bool),
}

impl Display for Filetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Filetype::Make { .. } => write!(f, "Makefile"),
            Filetype::Cmake { .. } => write!(f, "Cmake"),
            Filetype::Readme { .. } => write!(f, "Readme"),
            Filetype::Docker { .. } => write!(f, "Docker"),
            Filetype::Spec { .. } => write!(f, "Spec"),
        }
    }
}

const SCHEMA: &str = r#";use the following schema: mk=makefile, cm=cmake, dkr=docker, rdme=readme, c=constraints,; constraints are separated with '|';"#;
impl TryFrom<(TomlSpec, Filetype)> for ResolvedPrompt {
    type Error = BoxError;

    fn try_from(value: (TomlSpec, Filetype)) -> Result<Self, Self::Error> {
        let (spec, file_t) = value;
        let mut prompt = spec.context.prompt;
        prompt.push_str(SCHEMA);

        match &file_t {
            Filetype::Make { content } => write!(
                &mut prompt,
                r#";the output should be a valid mk: {};"#,
                content
            )?,
            Filetype::Cmake { content } => write!(
                &mut prompt,
                r#";the output should be a valid cm file: {};"#,
                content
            )?,
            Filetype::Readme { content } => write!(
                &mut prompt,
                r#";the output should be a valid rdme: {};"#,
                content
            )?,
            Filetype::Docker { content } => write!(
                &mut prompt,
                r#";the output file should be a valid dkr file: {};"#,
                content
            )?,
            Filetype::Spec { content } => write!(
                &mut prompt,
                r#";the output file should be a valid toml file: {};"#,
                content
            )?,
        };

        if !spec.context.constraints.is_empty() {
            prompt.push_str(" c:");
            prompt.push_str(&spec.context.constraints.join("|"));
        };

        Ok(Self {
            model: spec.model,
            system_prompt: spec.context.system_prompt,
            prompt,
            stream: false,
            think: spec.think.unwrap_or(Boolish::Bool(true)),
            keep_alive: None,
            options: spec.system,
        })
    }
}
