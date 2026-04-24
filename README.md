# maker

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Status](https://img.shields.io/badge/status-beta-yellow.svg)
![License](https://img.shields.io/badge/license-Proprietary-red.svg)

## Description

`maker` is a high-performance Rust web server and utility designed to streamline file templating through specification-driven prompts and Ollama. It accepts a TOML configuration file that defines the structure and requirements of the desired output.

Built for speed and modularity, `maker` leverages the following core technologies:

- **hyper**: For low-level HTTP functionality where remote prompts are utilized.
- **tower**: For composable async services and middleware management.
- **serde**: For robust serialization and deserialization of the TOML specifications.

## Usage

To use `maker`, create a TOML specification file describing the template requirements.

### Configuration Example

Create a file named `spec.toml` in your specifications folder:

```toml

# in ./specifications

model = "deepseek-v3.2:cloud"
think = "low" # also accepts think = true

contraints = [
    "you will verify all output is correct and no extra fields are appended"
]

# parameters that are used  in Ollama requests. More details can be found in 
# the Ollama API documentation
[system]
temperature = 0.2
top_p = 0.9
num_ctx = 8192
num_predict = 8000

[context]
system_prompt = "You will take user input and ..."
prompt = "You are an expert ..."


```

Upon receiving a valid request, the user request and toml specification is translated into the necessary prompt context for generating the templated file.

## License

This software is proprietary and closed-source. All rights are reserved. Unauthorized copying, distribution, or modification of this software is strictly prohibited.

```text
Copyright © 2023. All Rights Reserved.
```
