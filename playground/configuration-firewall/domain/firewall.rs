#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceMode {
    ReadOnly,
    Write,
}

impl ServiceMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ServiceMode::ReadOnly => "ReadOnly",
            ServiceMode::Write => "Write",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceConfiguration {
    pub endpoint: String,
    pub retry_limit: i32,
    pub mode: ServiceMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationIssue {
    pub code: &'static str,
    pub key: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigurationLoad {
    Loaded(ServiceConfiguration),
    Rejected(Vec<ConfigurationIssue>),
}

fn parse_endpoint(raw: Option<&str>) -> Result<String, ConfigurationIssue> {
    match raw {
        None => Err(ConfigurationIssue {
            code: "missing",
            key: "Service:Endpoint",
        }),
        Some(text) => {
            let trimmed = text.trim();
            let rest = trimmed
                .strip_prefix("https://")
                .or_else(|| trimmed.strip_prefix("http://"));
            match rest {
                Some(authority) if !authority.is_empty() && !authority.starts_with('/') => {
                    if authority.contains('/') {
                        Ok(trimmed.to_string())
                    } else {
                        Ok(format!("{trimmed}/"))
                    }
                }
                _ => Err(ConfigurationIssue {
                    code: "invalid-uri",
                    key: "Service:Endpoint",
                }),
            }
        }
    }
}

fn parse_retry_limit(raw: Option<&str>) -> Result<i32, ConfigurationIssue> {
    match raw {
        None => Err(ConfigurationIssue {
            code: "missing",
            key: "Service:RetryLimit",
        }),
        Some(text) => match text.trim().parse::<i32>() {
            Ok(limit) if (1..=10).contains(&limit) => Ok(limit),
            _ => Err(ConfigurationIssue {
                code: "invalid-range",
                key: "Service:RetryLimit",
            }),
        },
    }
}

fn parse_mode(raw: Option<&str>) -> Result<ServiceMode, ConfigurationIssue> {
    match raw {
        None => Err(ConfigurationIssue {
            code: "missing",
            key: "Service:Mode",
        }),
        Some("read-only") => Ok(ServiceMode::ReadOnly),
        Some("write") => Ok(ServiceMode::Write),
        Some(_) => Err(ConfigurationIssue {
            code: "invalid-mode",
            key: "Service:Mode",
        }),
    }
}

fn lookup<'a>(values: &[(&'a str, &'a str)], target: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(key, _)| *key == target)
        .map(|(_, value)| *value)
}

pub fn load_configuration(values: &[(&str, &str)]) -> ConfigurationLoad {
    let endpoint = parse_endpoint(lookup(values, "Service:Endpoint"));
    let retry_limit = parse_retry_limit(lookup(values, "Service:RetryLimit"));
    let mode = parse_mode(lookup(values, "Service:Mode"));

    match (endpoint, retry_limit, mode) {
        (Ok(ep), Ok(limit), Ok(m)) => ConfigurationLoad::Loaded(ServiceConfiguration {
            endpoint: ep,
            retry_limit: limit,
            mode: m,
        }),
        (ep_res, limit_res, mode_res) => {
            let issues: Vec<ConfigurationIssue> = [
                ep_res.err(),
                limit_res.err(),
                mode_res.err(),
            ]
            .into_iter()
            .flatten()
            .collect();
            ConfigurationLoad::Rejected(issues)
        }
    }
}

pub fn canonical_configuration(result: &ConfigurationLoad) -> String {
    match result {
        ConfigurationLoad::Loaded(cfg) => {
            format!(
                "loaded|{}|{}|{}",
                cfg.endpoint,
                cfg.retry_limit,
                cfg.mode.as_str()
            )
        }
        ConfigurationLoad::Rejected(issues) => {
            let parts: Vec<String> = issues
                .iter()
                .map(|issue| format!("{}:{}", issue.code, issue.key))
                .collect();
            format!("rejected|{}", parts.join(";"))
        }
    }
}
