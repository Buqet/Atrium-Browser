use std::collections::{HashSet, HashMap};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct CspPolicy {
    pub default_src: Option<SourceList>,
    pub script_src: Option<SourceList>,
    pub style_src: Option<SourceList>,
    pub img_src: Option<SourceList>,
    pub connect_src: Option<SourceList>,
    pub font_src: Option<SourceList>,
    pub object_src: Option<SourceList>,
    pub frame_src: Option<SourceList>,
    pub base_uri: Option<SourceList>,
    pub form_action: Option<SourceList>,
    pub frame_ancestors: Option<SourceList>,
    pub upgrade_insecure_requests: bool,
    pub block_all_mixed_content: bool,
}

impl CspPolicy {
    pub fn new() -> Self {
        CspPolicy {
            default_src: None,
            script_src: None,
            style_src: None,
            img_src: None,
            connect_src: None,
            font_src: None,
            object_src: None,
            frame_src: None,
            base_uri: None,
            form_action: None,
            frame_ancestors: None,
            upgrade_insecure_requests: false,
            block_all_mixed_content: false,
        }
    }

    pub fn parse(header: &str) -> Result<Self> {
        let mut policy = CspPolicy::new();
        for directive in header.split(';') {
            let directive = directive.trim();
            if directive.is_empty() { continue; }
            let parts: Vec<&str> = directive.split_whitespace().collect();
            if parts.is_empty() { continue; }
            let name = parts[0];
            let values: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
            match name {
                "default-src" => policy.default_src = Some(SourceList::parse(&values)),
                "script-src" => policy.script_src = Some(SourceList::parse(&values)),
                "style-src" => policy.style_src = Some(SourceList::parse(&values)),
                "img-src" => policy.img_src = Some(SourceList::parse(&values)),
                "connect-src" => policy.connect_src = Some(SourceList::parse(&values)),
                "font-src" => policy.font_src = Some(SourceList::parse(&values)),
                "object-src" => policy.object_src = Some(SourceList::parse(&values)),
                "frame-src" => policy.frame_src = Some(SourceList::parse(&values)),
                "base-uri" => policy.base_uri = Some(SourceList::parse(&values)),
                "form-action" => policy.form_action = Some(SourceList::parse(&values)),
                "frame-ancestors" => policy.frame_ancestors = Some(SourceList::parse(&values)),
                "upgrade-insecure-requests" => policy.upgrade_insecure_requests = true,
                "block-all-mixed-content" => policy.block_all_mixed_content = true,
                _ => {}
            }
        }
        Ok(policy)
    }

    pub fn allows(&self, resource_type: &str, url: &str, nonce: Option<&str>, script_hash: Option<&str>) -> bool {
        let source_list = match resource_type {
            "script" => &self.script_src,
            "style" => &self.style_src,
            "image" => &self.img_src,
            "connect" => &self.connect_src,
            "font" => &self.font_src,
            "object" => &self.object_src,
            "frame" => &self.frame_src,
            _ => &self.default_src,
        };
        match source_list {
            Some(list) => list.allows(url, nonce, script_hash),
            None => self.default_src.as_ref().map_or(true, |list| list.allows(url, nonce, script_hash))
        }
    }
}

impl Default for CspPolicy {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct SourceList {
    pub sources: HashSet<String>,
    pub allow_self: bool,
    pub allow_none: bool,
    pub allow_unsafe_inline: bool,
    pub allow_unsafe_eval: bool,
    pub allow_unsafe_hashes: bool,
    pub allow_strict_dynamic: bool,
    pub nonces: HashSet<String>,
    pub hashes: HashSet<(String, String)>,
    pub wildcard: bool,
}

impl SourceList {
    pub fn parse(values: &[String]) -> Self {
        let mut source_list = SourceList {
            sources: HashSet::new(),
            allow_self: false,
            allow_none: false,
            allow_unsafe_inline: false,
            allow_unsafe_eval: false,
            allow_unsafe_hashes: false,
            allow_strict_dynamic: false,
            nonces: HashSet::new(),
            hashes: HashSet::new(),
            wildcard: false,
        };
        for value in values {
            match value.as_str() {
                "'self'" => source_list.allow_self = true,
                "'none'" => source_list.allow_none = true,
                "'unsafe-inline'" => source_list.allow_unsafe_inline = true,
                "'unsafe-eval'" => source_list.allow_unsafe_eval = true,
                "'unsafe-hashes'" => source_list.allow_unsafe_hashes = true,
                "'strict-dynamic'" => source_list.allow_strict_dynamic = true,
                "*" => source_list.wildcard = true,
                _ => {
                    if value.starts_with("'nonce-") && value.ends_with('\'') {
                        let nonce = value[7..value.len()-1].to_string();
                        source_list.nonces.insert(nonce);
                    } else if value.starts_with("'sha256-") && value.ends_with('\'') {
                        let hash = value[8..value.len()-1].to_string();
                        source_list.hashes.insert(("sha256".to_string(), hash));
                    } else if value.starts_with("'sha384-") && value.ends_with('\'') {
                        let hash = value[8..value.len()-1].to_string();
                        source_list.hashes.insert(("sha384".to_string(), hash));
                    } else if value.starts_with("'sha512-") && value.ends_with('\'') {
                        let hash = value[8..value.len()-1].to_string();
                        source_list.hashes.insert(("sha512".to_string(), hash));
                    } else {
                        source_list.sources.insert(value.clone());
                    }
                }
            }
        }
        source_list
    }

    pub fn allows(&self, url: &str, nonce: Option<&str>, script_hash: Option<&str>) -> bool {
        if self.allow_none { return false; }
        if self.wildcard && !self.allow_strict_dynamic { return true; }
        if self.allow_self {
            if let Ok(parsed) = url::Url::parse(url) {
                if parsed.origin() == url::Url::parse("self://self").unwrap().origin() {
                }
            }
        }
        if let Some(n) = nonce {
            if self.nonces.contains(n) { return true; }
        }
        if let Some(h) = script_hash {
            for (_, hash_val) in &self.hashes {
                if *hash_val == h { return true; }
            }
        }
        for source in &self.sources {
            if self.source_matches(source, url) { return true; }
        }
        false
    }

    fn source_matches(&self, source: &str, url: &str) -> bool {
        if source == url { return true; }
        if let Ok(parsed_url) = url::Url::parse(url) {
            if source.starts_with("*.") {
                let domain = &source[2..];
                if let Some(host) = parsed_url.host_str() {
                    return host == domain || host.ends_with(&format!(".{}", domain));
                }
            }
            if source.ends_with('/') {
                return url.starts_with(source);
            }
            if let Ok(source_url) = url::Url::parse(source) {
                return parsed_url.scheme() == source_url.scheme()
                    && parsed_url.host() == source_url.host()
                    && parsed_url.port() == source_url.port()
                    && parsed_url.path().starts_with(source_url.path());
            }
        }
        false
    }
}

// Остальной код (CorsValidator, CertificateValidator) пока без изменений.

#[derive(Error, Debug)]
pub enum CorsError {
    #[error("Origin '{0}' not allowed")]
    OriginNotAllowed(String),
    #[error("Method '{0}' not allowed")]
    MethodNotAllowed(String),
    #[error("Header '{0}' not allowed")]
    HeaderNotAllowed(String),
    #[error("Credentials cannot be used with wildcard origin")]
    CredentialsWithWildcard,
}


pub struct CorsValidator {
    allowed_origins: HashSet<String>,
    allowed_methods: HashSet<String>,
    allowed_headers: HashSet<String>,
    allow_credentials: bool,
    max_age: Option<u32>,
}

impl CorsValidator {
    pub fn new() -> Self {
        Self {
            allowed_origins: HashSet::new(),
            allowed_methods: vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]
                .into_iter().map(String::from).collect(),
            allowed_headers: HashSet::new(),
            allow_credentials: false,
            max_age: None,
        }

    }

    pub fn allowed_origins(mut self, origins: Vec<&str>) -> Self {
        self.allowed_origins = origins.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn validate(&self, origin: &str, method: &str, headers: &[&str]) -> Result<()> {
        if !self.allowed_origins.is_empty() && !self.allowed_origins.contains(origin) {
            return Err(anyhow!("CORS: Origin '{}' not allowed", origin));
        }
        if !self.allowed_methods.contains(method) {
            return Err(anyhow!("CORS: Method '{}' not allowed", method));
        }
        for header in headers {
            if !self.allowed_headers.is_empty() && !self.allowed_headers.contains(*header) {
                return Err(anyhow!("CORS: Header '{}' not allowed", header));
            }
        }
        Ok(())
    }

    pub fn get_headers(&self, origin: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        if self.allowed_origins.contains(origin) || self.allowed_origins.is_empty() {
            headers.insert("Access-Control-Allow-Origin".to_string(), origin.to_string());
        }
        headers.insert(
            "Access-Control-Allow-Methods".to_string(),
            self.allowed_methods.iter().cloned().collect::<Vec<_>>().join(", "),
        );
        if !self.allowed_headers.is_empty() {
            headers.insert(
                "Access-Control-Allow-Headers".to_string(),
                self.allowed_headers.iter().cloned().collect::<Vec<_>>().join(", "),
            );
        }
        if self.allow_credentials {
            headers.insert("Access-Control-Allow-Credentials".to_string(), "true".to_string());
        }
        headers
    }
}

impl Default for CorsValidator {
    fn default() -> Self { Self::new() }
}

pub struct CertificateValidator;

impl CertificateValidator {
    pub fn new() -> Self { CertificateValidator }
    pub fn validate(&self, cert_chain: &[Vec<u8>], domain: &str) -> Result<()> {
        if cert_chain.is_empty() {
            return Err(anyhow!("Empty certificate chain"));
        }
        Ok(())
    }
}

impl Default for CertificateValidator {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csp_parse() {
        let header = "default-src 'self'; script-src 'self' https://example.com";
        let policy = CspPolicy::parse(header).unwrap();
        assert!(policy.default_src.is_some());
        assert!(policy.script_src.is_some());
    }

    #[test]
    fn test_csp_allows() {
        let header = "default-src *";
        let policy = CspPolicy::parse(header).unwrap();
        assert!(policy.allows("script", "https://example.com"));
        assert!(policy.allows("script", "https://cdn.example.com"));
    }

    #[test]
    fn test_source_list() {
        let sources = vec!["'self'".to_string(), "https://example.com".to_string()];
        let list = SourceList::parse(&sources);
        assert!(list.allow_self);
        assert!(!list.wildcard);
    }

    #[test]
    fn test_cors_validator() {
        let validator = CorsValidator::new()
            .allowed_origins(vec!["https://example.com"]);
        assert!(validator.validate("https://example.com", "GET", &[]).is_ok());
        assert!(validator.validate("https://evil.com", "GET", &[]).is_err());
    }

    #[test]
    fn test_certificate_validator() {
        let validator = CertificateValidator::new();
        assert!(validator.validate(&[], "example.com").is_err());
        assert!(validator.validate(&[vec![1, 2, 3]], "example.com").is_ok());
    }
}
