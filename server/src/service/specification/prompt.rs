//! A concrete representation of a valid prompt that will
//! resolve to a valid Ollama request

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use tower::BoxError;

#[derive(Serialize, Deserialize)]
pub(crate) struct System {
    temperature: Option<f32>,
    top_p: Option<f32>,
    num_predict: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TaskContext {
    prompt: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TomlSpec {
    model: Option<String>,
    system: Option<System>,
    context: TaskContext,
    constraints: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "filetype", rename_all = "lowercase")]
pub(crate) enum Filetype {
    Make { content: String },
    Cmake { content: String },
    Readme { content: String },
    Docker { content: String },
}

const SCHEMA: &str = r#";use the following schema: mk=makefile, cm=cmake, dkr=docker, rdme=readme c=constraints; constraints are separated with '|';"#;
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
                r#";the output file should be a valid: {};"#,
                content
            )?,
        };

        if !spec.constraints.is_empty() {
            prompt.push_str(" c:");
            prompt.push_str(&spec.constraints.join("|"));
        };

        Ok(Self {
            model: spec.model,
            prompt,
            streaming: false,
            options: spec.system,
        })
    }
}

type Options = System;
#[derive(Deserialize, Serialize)]
pub(crate) struct ResolvedPrompt {
    pub(crate) model: Option<String>,
    prompt: String,
    streaming: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Options>,
}
