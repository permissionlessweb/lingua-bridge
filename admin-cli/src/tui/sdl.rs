use serde_yaml::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Default bundled SDL content.
pub const DEFAULT_SDL: &str = include_str!("../../../deploy.yaml");

/// A template variable detected in the SDL (e.g., `<YOUR_ADMIN_PUBLIC_KEY>`)
#[derive(Debug, Clone)]
pub struct SdlVariable {
    pub name: String,       // The variable name without angle brackets
    pub placeholder: String, // The full placeholder as it appears in the SDL
    pub value: String,      // User-provided value (empty until filled)
    pub context: String,    // Which service/section it appears in
}

/// Parsed SDL with editable fields extracted.
pub struct SdlFile {
    pub raw: String,
    pub services: Vec<SdlService>,
    pub variables: Vec<SdlVariable>,
}

/// A service entry from the SDL.
pub struct SdlService {
    pub name: String,
    pub image: String,
    pub env_vars: Vec<EnvVar>,
    pub resources: ServiceResources,
}

/// An environment variable extracted from a service.
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

/// Resource allocation for a service.
pub struct ServiceResources {
    pub cpu: String,
    pub memory: String,
    pub storage: String,
    pub gpu: String,
}

impl SdlFile {
    /// Parse from a YAML string.
    pub fn parse(yaml: &str) -> Result<Self, String> {
        let doc: Value = serde_yaml::from_str(yaml)
            .map_err(|e| format!("YAML parse error: {}", e))?;

        let services = Self::extract_services(&doc)?;
        let variables = Self::detect_variables(yaml);

        Ok(Self {
            raw: yaml.to_string(),
            services,
            variables,
        })
    }

    /// Load from a file path, falling back to the bundled default.
    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        let content = if let Some(p) = path {
            std::fs::read_to_string(p)
                .map_err(|e| format!("Cannot read {}: {}", p.display(), e))?
        } else {
            DEFAULT_SDL.to_string()
        };
        Self::parse(&content)
    }

    /// Detect template variables in the SDL (patterns like `<VAR_NAME>` or `${VAR_NAME}`)
    fn detect_variables(yaml: &str) -> Vec<SdlVariable> {
        let mut seen: BTreeMap<String, SdlVariable> = BTreeMap::new();
        let mut current_service = String::new();

        for line in yaml.lines() {
            let trimmed = line.trim();

            // Track current service context
            if !trimmed.starts_with('-') && !trimmed.starts_with('#') && trimmed.ends_with(':') {
                let name = trimmed.trim_end_matches(':').trim();
                if !name.is_empty() && !name.contains(' ') {
                    current_service = name.to_string();
                }
            }

            // Detect <VAR> patterns (angle brackets)
            let mut start = 0;
            while let Some(open) = line[start..].find('<') {
                let abs_open = start + open;
                if let Some(close) = line[abs_open..].find('>') {
                    let abs_close = abs_open + close;
                    let placeholder = &line[abs_open..=abs_close];
                    let name = &line[abs_open + 1..abs_close];
                    // Only treat as variable if it looks like a placeholder (uppercase, underscores)
                    if !name.is_empty()
                        && name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                    {
                        seen.entry(name.to_string()).or_insert_with(|| SdlVariable {
                            name: name.to_string(),
                            placeholder: placeholder.to_string(),
                            value: String::new(),
                            context: current_service.clone(),
                        });
                    }
                    start = abs_close + 1;
                } else {
                    break;
                }
            }

            // Detect ${VAR} patterns
            start = 0;
            while let Some(dollar) = line[start..].find("${") {
                let abs_dollar = start + dollar;
                if let Some(close) = line[abs_dollar..].find('}') {
                    let abs_close = abs_dollar + close;
                    let placeholder = &line[abs_dollar..=abs_close];
                    let name = &line[abs_dollar + 2..abs_close];
                    if !name.is_empty() {
                        seen.entry(name.to_string()).or_insert_with(|| SdlVariable {
                            name: name.to_string(),
                            placeholder: placeholder.to_string(),
                            value: String::new(),
                            context: current_service.clone(),
                        });
                    }
                    start = abs_close + 1;
                } else {
                    break;
                }
            }
        }

        seen.into_values().collect()
    }

    /// Check if all variables have been filled
    pub fn all_variables_filled(&self) -> bool {
        self.variables.iter().all(|v| !v.value.is_empty())
    }

    /// Get unfilled variables
    pub fn unfilled_variables(&self) -> Vec<&SdlVariable> {
        self.variables.iter().filter(|v| v.value.is_empty()).collect()
    }

    /// Regenerate the YAML with variable substitutions applied.
    pub fn render_yaml(&self) -> String {
        let mut output = self.raw.clone();

        // Apply variable substitutions
        for var in &self.variables {
            if !var.value.is_empty() {
                output = output.replace(&var.placeholder, &var.value);
            }
        }

        // Apply env var edits by line replacement
        for svc in &self.services {
            for env in &svc.env_vars {
                let pattern = format!("{}=", env.key);
                let new_lines: Vec<String> = output
                    .lines()
                    .map(|line| {
                        let trimmed = line.trim().trim_start_matches("- ").trim_matches('"');
                        if trimmed.starts_with(&pattern) {
                            let indent = &line[..line.len() - line.trim_start().len()];
                            let dash = if line.trim().starts_with('-') { "- " } else { "" };
                            format!("{}{}\"{}={}\"", indent, dash, env.key, env.value)
                        } else {
                            line.to_string()
                        }
                    })
                    .collect();
                output = new_lines.join("\n");
            }
        }

        output
    }

    fn extract_services(doc: &Value) -> Result<Vec<SdlService>, String> {
        let mut services = Vec::new();

        let svc_map = doc.get("services")
            .and_then(|v| v.as_mapping())
            .ok_or("no 'services' section in SDL")?;

        let profiles = doc.get("profiles")
            .and_then(|v| v.get("compute"))
            .and_then(|v| v.as_mapping());

        for (name_val, svc_val) in svc_map {
            let name = name_val.as_str().unwrap_or("unknown").to_string();
            let image = svc_val.get("image")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract env vars
            let env_vars = Self::extract_env_vars(svc_val);

            // Extract resources from profiles
            let resources = if let Some(profile_map) = profiles {
                Self::extract_resources(profile_map, &name)
            } else {
                ServiceResources::default()
            };

            services.push(SdlService {
                name,
                image,
                env_vars,
                resources,
            });
        }

        Ok(services)
    }

    fn extract_env_vars(svc: &Value) -> Vec<EnvVar> {
        let mut vars = Vec::new();
        if let Some(env_list) = svc.get("env").and_then(|v| v.as_sequence()) {
            for item in env_list {
                if let Some(s) = item.as_str() {
                    if let Some((key, val)) = s.split_once('=') {
                        vars.push(EnvVar {
                            key: key.to_string(),
                            value: val.to_string(),
                        });
                    }
                }
            }
        }
        vars
    }

    fn extract_resources(profiles: &serde_yaml::Mapping, service_name: &str) -> ServiceResources {
        let key = Value::String(service_name.to_string());
        let profile = match profiles.get(&key) {
            Some(v) => v,
            None => return ServiceResources::default(),
        };

        let res = match profile.get("resources") {
            Some(v) => v,
            None => return ServiceResources::default(),
        };

        let cpu = res.get("cpu")
            .and_then(|v| v.get("units"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()).or_else(|| v.as_str().map(|s| s.to_string())))
            .unwrap_or_else(|| "1".to_string());

        let memory = res.get("memory")
            .and_then(|v| v.get("size"))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "512Mi".to_string());

        let storage = res.get("storage")
            .and_then(|v| {
                if let Some(seq) = v.as_sequence() {
                    seq.first().and_then(|item| item.get("size")).and_then(|s| s.as_str().map(|s| s.to_string()))
                } else {
                    v.get("size").and_then(|s| s.as_str().map(|s| s.to_string()))
                }
            })
            .unwrap_or_else(|| "1Gi".to_string());

        let gpu = res.get("gpu")
            .and_then(|v| v.get("units"))
            .and_then(|v| v.as_u64().map(|n| n.to_string()))
            .unwrap_or_else(|| "0".to_string());

        ServiceResources { cpu, memory, storage, gpu }
    }
}

impl Default for ServiceResources {
    fn default() -> Self {
        Self {
            cpu: "1".to_string(),
            memory: "512Mi".to_string(),
            storage: "1Gi".to_string(),
            gpu: "0".to_string(),
        }
    }
}
