//! A concrete representation of a valid prompt that will
//! resolve to a valid Ollama request

use std::fmt::Write;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tower::BoxError;

#[derive(Serialize, Deserialize)]
pub(crate) struct TomlSpec {
    model: Option<String>,
    system: Option<System>,
    think: Option<Boolish>,
    context: TaskContext,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "filetype", rename_all = "lowercase")]
pub(crate) enum Filetype {
    Make { content: String },
    Cmake { content: String },
    Readme { content: String },
    Docker { content: String },
    Spec { content: String },
    Anki { content: String },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct System {
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
pub(crate) struct TaskContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    prompt: String,
    constraints: Vec<String>,
}

type Options = System;
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct ResolvedPrompt {
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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
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
            Filetype::Readme { .. } => write!(f, "README"),
            Filetype::Docker { .. } => write!(f, "Docker"),
            Filetype::Spec { .. } => write!(f, "Spec"),
            Filetype::Anki { .. } => write!(f, "Anki"),
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
            Filetype::Make { content } => {
                write!(&mut prompt, r#";the output should be a valid mk: {};"#, content)?
            }
            Filetype::Cmake { content } => {
                write!(&mut prompt, r#";the output should be a valid cm file: {};"#, content)?
            }
            Filetype::Readme { content } => {
                write!(&mut prompt, r#";the output should be a valid rdme: {};"#, content)?
            }
            Filetype::Docker { content } => {
                write!(&mut prompt, r#";the output file should be a valid dkr file: {};"#, content)?
            }
            Filetype::Spec { content } => write!(
                &mut prompt,
                r#";the output file should be a valid toml file: {};"#,
                content
            )?,
            Filetype::Anki { content } => write!(
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec() -> TomlSpec {
        TomlSpec {
            model: Some("llama3.2".into()),
            system: Some(System {
                temperature: Some(0.1),
                top_p: Some(0.9),
                num_ctx: Some(16384),
                num_predict: Some(2048),
            }),
            think: Some(Boolish::Medium),
            context: TaskContext {
                system_prompt: Some("Generate clean output only.".into()),
                prompt: "Generate a Makefile".into(),
                constraints: vec!["NO markdown".into(), "raw output only".into()],
            },
        }
    }

    #[test]
    fn resolved_prompt_includes_model() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "for a Rust project".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        assert_eq!(resolved.model.as_deref(), Some("llama3.2"));
    }

    #[test]
    fn resolved_prompt_is_not_streaming() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "hello".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        assert!(!resolved.stream);
    }

    #[test]
    fn resolved_prompt_includes_constraints() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "test".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        let json = serde_json::to_string(&resolved).unwrap();
        assert!(json.contains("NO markdown"));
        assert!(json.contains("raw output only"));
    }

    #[test]
    fn resolved_prompt_includes_system_prompt() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "test".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        assert_eq!(resolved.system_prompt.as_deref(), Some("Generate clean output only."));
    }

    #[test]
    fn resolved_prompt_includes_system_options() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "test".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        let json = serde_json::to_string(&resolved).unwrap();
        assert!(json.contains(r#""temperature""#));
        assert!(json.contains(r#""num_ctx""#));
    }

    #[test]
    fn resolved_prompt_produces_valid_ollama_json() {
        let spec = make_spec();
        let filetype = Filetype::Make { content: "foobar".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        let json = serde_json::to_string(&resolved).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("model").is_some());
        assert!(parsed.get("prompt").is_some());
        assert!(!parsed.get("stream").unwrap().as_bool().unwrap());
    }

    #[test]
    fn resolved_prompt_without_constraints() {
        let mut spec = make_spec();
        spec.context.constraints = vec![];
        let filetype = Filetype::Docker { content: "python app".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        assert!(resolved.prompt.contains("dkr"));
        assert!(!resolved.prompt.contains(" c:"));
    }

    #[test]
    fn resolved_prompt_no_model_no_think_no_system() {
        let spec = TomlSpec {
            model: None,
            system: None,
            think: None,
            context: TaskContext {
                system_prompt: None,
                prompt: "simple".into(),
                constraints: vec![],
            },
        };
        let filetype = Filetype::Readme { content: "overview".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        assert!(resolved.model.is_none());
        assert!(resolved.system_prompt.is_none());
        assert!(resolved.options.is_none());
    }

    #[test]
    fn filetype_display() {
        let cases = [
            (Filetype::Make { content: "".into() }, "Makefile"),
            (Filetype::Cmake { content: "".into() }, "Cmake"),
            (Filetype::Readme { content: "".into() }, "README"),
            (Filetype::Docker { content: "".into() }, "Docker"),
            (Filetype::Spec { content: "".into() }, "Spec"),
        ];
        for (ft, expected) in cases {
            assert_eq!(ft.to_string(), expected);
        }
    }

    #[test]
    fn filetype_deserializes_from_tagged_json() {
        let json = r#"{"filetype":"make", "content":"test make"}"#;
        let ft: Filetype = serde_json::from_str(json).unwrap();
        assert!(matches!(ft, Filetype::Make { .. }));
    }

    #[test]
    fn resolved_prompt_serde_roundtrip() {
        let spec = make_spec();
        let filetype = Filetype::Spec { content: "a config".into() };
        let resolved = ResolvedPrompt::try_from((spec, filetype)).unwrap();
        let json = serde_json::to_string(&resolved).unwrap();
        let back: ResolvedPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(resolved.model, back.model);
        assert_eq!(resolved.stream, back.stream);
        assert!(!back.prompt.is_empty());
    }
}
