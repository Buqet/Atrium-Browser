use anyhow::{Result, anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs;
use regex::Regex;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmlFile {
    
    pub version: Option<String>,
    
    pub metadata: HashMap<String, String>,
    
    pub initial_keys: HashSet<String>,
    
    pub languages: HashMap<String, Language>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    
    pub names: Vec<String>,
    
    pub translations: HashMap<String, Translation>,
    
    pub imports: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Translation {
    
    String(String),
    
    Plural {
        
        forms: Vec<String>,
        
        values: Vec<String>,
    },
    
    Alias(String),
    
    Contextual {
        context: String,
        value: String,
    },
}

impl SmlFile {
    
    pub fn new() -> Self {
        SmlFile {
            version: None,
            metadata: HashMap::new(),
            initial_keys: HashSet::new(),
            languages: HashMap::new(),
        }
    }

    
    pub fn with_version(version: &str) -> Self {
        let mut file = SmlFile::new();
        file.version = Some(version.to_string());
        file
    }

    
    pub fn add_initial_key(&mut self, key: &str) {
        self.initial_keys.insert(key.to_string());
    }

    
    pub fn add_translation(&mut self, lang_code: &str, key: &str, translation: Translation) -> Result<()> {
        let lang = self.languages
            .entry(lang_code.to_string())
            .or_insert_with(|| Language {
                names: Vec::new(),
                translations: HashMap::new(),
                imports: Vec::new(),
            });
        
        lang.translations.insert(key.to_string(), translation);
        Ok(())
    }

    
    pub fn add_language_name(&mut self, lang_code: &str, name: &str) {
        if let Some(lang) = self.languages.get_mut(lang_code) {
            lang.names.push(name.to_string());
        }
    }

    
    pub fn validate(&self) -> Result<()> {
        
        for (lang_code, lang) in &self.languages {
            for key in lang.translations.keys() {
                
                if let Some(translation) = lang.translations.get(key) {
                    if matches!(translation, Translation::Alias(_)) {
                        continue;
                    }
                }
                
                if !self.initial_keys.contains(key) {
                    
                    let base_key = key.split(" -> ").next().unwrap_or(key);
                    if !self.initial_keys.contains(base_key) {
                        return Err(anyhow!(
                            "Language '{}' has translation for unknown key: '{}'",
                            lang_code,
                            key
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    
    pub fn get_translation(&self, lang_code: &str, key: &str) -> Option<&Translation> {
        self.languages
            .get(lang_code)
            .and_then(|lang| lang.translations.get(key))
    }

    
    pub fn get_string(&self, lang_code: &str, key: &str) -> Option<&str> {
        match self.get_translation(lang_code, key)? {
            Translation::String(s) => Some(s),
            _ => None,
        }
    }

    
    pub fn get_plural(&self, lang_code: &str, key: &str, count: u64) -> Option<&str> {
        match self.get_translation(lang_code, key)? {
            Translation::Plural { forms, values } => {
                
                let index = match count {
                    1 => forms.iter().position(|f| f == "one"),
                    _ => {
                        if count % 10 >= 2 && count % 10 <= 4 && (count % 100 < 10 || count % 100 >= 20) {
                            forms.iter().position(|f| f == "few")
                        } else {
                            forms.iter().position(|f| f == "many")
                        }
                    }
                };
                
                index.and_then(|i| values.get(i)).map(|s| s.as_str())
            }
            Translation::String(s) => Some(s),
            _ => None,
        }
    }

    
    pub fn serialize(&self) -> String {
        let mut output = String::new();

        
        if let Some(ref version) = self.version {
            output.push_str(&format!("[version {}]\n", version));
        }

        for (key, value) in &self.metadata {
            output.push_str(&format!("[{}] -> {}\n", key, value));
        }

        if !self.metadata.is_empty() || self.version.is_some() {
            output.push('\n');
        }

        
        if !self.initial_keys.is_empty() {
            output.push_str("[Initial]\n");
            for key in &self.initial_keys {
                output.push_str(&format!("{}\n", key));
            }
            output.push('\n');
        }

        
        for (code, lang) in &self.languages {
            if !lang.imports.is_empty() {
                output.push_str(&format!("[Language, {}] -> {}\n", 
                    code, 
                    lang.imports.join(", ")));
            } else {
                output.push_str(&format!("[Language, {}]\n", code));
            }

            for name in &lang.names {
                output.push_str(&format!("  name -> {}\n", name));
            }

            for (key, translation) in &lang.translations {
                match translation {
                    Translation::String(value) => {
                        if value.contains('\n') {
                            output.push_str(&format!("{} =\n", key));
                            for line in value.lines() {
                                output.push_str(&format!("    {}\n", line));
                            }
                        } else {
                            output.push_str(&format!("{} = {}\n", key, value));
                        }
                    }
                    Translation::Plural { forms, values } => {
                        let forms_str = forms.join("|");
                        let values_str = values.join(" | ");
                        output.push_str(&format!("{}.{} = {}\n", key, forms_str, values_str));
                    }
                    Translation::Alias(target) => {
                        output.push_str(&format!("{} = {}\n", key, target));
                    }
                    Translation::Contextual { context, value } => {
                        output.push_str(&format!("{} -> {} = {}\n", key, context, value));
                    }
                }
            }

            output.push('\n');
        }

        output
    }

    
    pub fn load_with_imports<P: AsRef<Path>>(path: P, loaded: &mut HashSet<String>) -> Result<Self> {
        let path = path.as_ref();
        
        
        let path_str = path.to_string_lossy().to_string();
        if loaded.contains(&path_str) {
            return Err(anyhow!("Circular import detected: {}", path_str));
        }
        loaded.insert(path_str.clone());

        let content = fs::read_to_string(path)
            .context(format!("Failed to read file: {}", path_str))?;

        let parser = SmlParser::new()?;
        let mut file = parser.parse(&content)?;

        
        let imports_to_process: Vec<(String, String)> = file
            .languages
            .iter()
            .flat_map(|(code, lang)| {
                lang.imports.iter().map(|import_path| {
                    (code.clone(), import_path.clone())
                })
            })
            .collect();

        for (code, import_path_str) in imports_to_process {
            let import_path = Path::new(path.parent().unwrap_or(Path::new("")))
                .join(&import_path_str);

            if import_path.exists() {
                let imported = Self::load_with_imports(&import_path, loaded)?;

                
                for (imp_code, lang) in imported.languages {
                    let target_code = if imp_code.is_empty() { &code } else { &imp_code };
                    let target = file.languages
                        .entry(target_code.clone())
                        .or_insert_with(|| Language {
                            names: Vec::new(),
                            translations: HashMap::new(),
                            imports: Vec::new(),
                        });

                    for (key, translation) in lang.translations {
                        target.translations.insert(key, translation);
                    }
                }
            }
        }

        Ok(file)
    }
}

impl Default for SmlFile {
    fn default() -> Self {
        Self::new()
    }
}


pub struct SmlParser {
    metadata_regex: Regex,
    import_regex: Regex,
    contextual_regex: Regex,
    plural_regex: Regex,
    alias_regex: Regex,
}

impl SmlParser {
    pub fn new() -> Result<Self> {
        Ok(SmlParser {
            metadata_regex: Regex::new(r"^\[(\w+)\](?:\s*->\s*(.+))?$")?,
            import_regex: Regex::new(r"^\[Language,\s*([^\]]+)\](?:\s*->\s*(.+))?$")?,
            contextual_regex: Regex::new(r"^([^=]+?)\s*->\s*(\w+)\s*=\s*(.+)$")?,
            plural_regex: Regex::new(r"^([^.=]+)\.([^.]+)\s*=\s*(.+)$")?,
            alias_regex: Regex::new(r"^([^=]+)\s*=\s*([a-zA-Z_][a-zA-Z0-9_-]*)$")?,
        })
    }

    
    pub fn parse(&self, content: &str) -> Result<SmlFile> {
        let mut file = SmlFile::new();
        let mut current_section = Section::None;
        let mut current_lang_code: Option<String> = None;
        let mut multiline_key: Option<String> = None;
        let mut multiline_value: Option<String> = None;

        for line in content.lines() {
            
            if let Some(ref key) = multiline_key {
                if line.starts_with("    ") || line.starts_with("\t") {
                    
                    if let Some(ref mut value) = multiline_value {
                        value.push('\n');
                        value.push_str(line.trim_start());
                    }
                    continue;
                } else {
                    
                    if let (Some(key), Some(value)) = (multiline_key.take(), multiline_value.take()) {
                        if let Some(ref code) = current_lang_code {
                            if let Some(lang) = file.languages.get_mut(code) {
                                lang.translations.insert(key, Translation::String(value));
                            }
                        }
                    }
                }
            }

            let line = line.trim();
            
                        let comment_str: &str = "//";
            if line.is_empty() || line.starts_with('#') || line.starts_with(comment_str) {
                continue;
            }

            
            if line.starts_with('[') {
                
                if let Some(caps) = self.metadata_regex.captures(line) {
                    let key = &caps[1];
                    if key == "version" {
                        if let Some(value) = caps.get(2) {
                            file.version = Some(value.as_str().trim().to_string());
                        }
                    } else if key == "Initial" {
                        current_section = Section::Initial;
                        current_lang_code = None;
                        continue;
                    } else {
                        file.metadata.insert(key.to_string(), caps.get(2).map(|m| m.as_str().trim().to_string()).unwrap_or_default());
                    }
                    continue;
                }

                
                if let Some(caps) = self.import_regex.captures(line) {
                    let code = caps[1].trim().to_string();
                    current_lang_code = Some(code.clone());
                    current_section = Section::Language;

                    let lang = file.languages
                        .entry(code)
                        .or_insert_with(|| Language {
                            names: Vec::new(),
                            translations: HashMap::new(),
                            imports: Vec::new(),
                        });

                    
                    if let Some(imports) = caps.get(2) {
                        lang.imports = imports.as_str()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect();
                    }
                    continue;
                }
            }

            
            match current_section {
                Section::Initial => {
                    
                    file.initial_keys.insert(line.to_string());
                }
                Section::Language => {
                    if let Some(ref code) = current_lang_code {
                        
                        if line.starts_with("name ->") {
                            if let Some(lang) = file.languages.get_mut(code) {
                                let name = line.trim_start_matches("name ->").trim();
                                lang.names.push(name.to_string());
                            }
                            continue;
                        }

                        
                        if let Some(caps) = self.contextual_regex.captures(line) {
                            let key = caps[1].trim();
                            let context = caps[2].trim();
                            let value = caps[3].trim();
                            
                            if let Some(lang) = file.languages.get_mut(code) {
                                lang.translations.insert(
                                    format!("{} -> {}", key, context),
                                    Translation::Contextual {
                                        context: context.to_string(),
                                        value: value.to_string(),
                                    },
                                );
                            }
                            continue;
                        }

                        
                        if let Some(caps) = self.plural_regex.captures(line) {
                            let key = caps[1].trim();
                            let forms_str = caps[2].trim();
                            let values_str = caps[3].trim();

                            let forms: Vec<String> = forms_str.split('|').map(|s| s.trim().to_string()).collect();
                            let mut values: Vec<String> = values_str
                                .split(" | ")
                                .map(|s| s.trim().to_string())
                                .collect();
                            
                            
                            if let Some(first) = values.first().cloned() {
                                for v in &mut values {
                                    if v == "-" || v == "-||-" {
                                        *v = first.clone();
                                    }
                                }
                            }

                            if let Some(lang) = file.languages.get_mut(code) {
                                lang.translations.insert(
                                    key.to_string(),
                                    Translation::Plural { forms, values },
                                );
                            }
                            continue;
                        }

                        
                        if let Some(eq_pos) = line.find('=') {
                            let key = line[..eq_pos].trim();
                            let value = line[eq_pos + 1..].trim();

                            if let Some(lang) = file.languages.get_mut(code) {
                                let dq: char = '\u{0022}';
                                let nl: char = '\u{000A}';
                                let has_space = value.contains(' ');
                                let has_quote = value.chars().any(|c| c == dq);
                                let starts_nl = value.starts_with(nl);
                                if !has_space && !has_quote && !starts_nl {
                                    if value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                                        lang.translations.insert(key.to_string(), Translation::Alias(value.to_string()));
                                        continue;
                                    }
                                }

                                
                                if value.is_empty() {
                                    multiline_key = Some(key.to_string());
                                    multiline_value = Some(String::new());
                                } else {
                                    
                                    let value = value.trim_matches('"').to_string();
                                    lang.translations.insert(key.to_string(), Translation::String(value));
                                }
                            }
                        }
                    }
                }
                Section::None => {}
            }
        }

        
        if let (Some(key), Some(value)) = (multiline_key, multiline_value) {
            if let Some(ref code) = current_lang_code {
                if let Some(lang) = file.languages.get_mut(code) {
                    lang.translations.insert(key, Translation::String(value));
                }
            }
        }

        Ok(file)
    }
}

impl Default for SmlParser {
    fn default() -> Self {
        Self::new().unwrap()
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    None,
    Initial,
    Language,
}


pub fn parse_sml(content: &str) -> Result<SmlFile> {
    let parser = SmlParser::new()?;
    parser.parse(content)
}


pub fn validate_sml(file: &SmlFile) -> Result<()> {
    file.validate()
}


pub fn serialize_sml(file: &SmlFile) -> String {
    file.serialize()
}


pub fn load_sml<P: AsRef<Path>>(path: P) -> Result<SmlFile> {
    let content = fs::read_to_string(path)
        .context("Failed to read SML file")?;
    parse_sml(&content)
}


pub struct Localization {
    sml: SmlFile,
    current_lang: String,
    fallback_lang: Option<String>,
}

impl Localization {
    pub fn new(sml: SmlFile, lang: &str) -> Self {
        Localization {
            sml,
            current_lang: lang.to_string(),
            fallback_lang: Some("en".to_string()),
        }
    }

    pub fn with_fallback(mut self, fallback: &str) -> Self {
        self.fallback_lang = Some(fallback.to_string());
        self
    }

    
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.sml
            .get_string(&self.current_lang, key)
            .or_else(|| self.fallback_lang.as_ref().and_then(|f| self.sml.get_string(f, key)))
            .unwrap_or(key)
    }

    
    pub fn ngettext<'a>(&'a self, singular: &'a str, plural: &'a str, count: u64) -> &'a str {
        
        if let Some(result) = self.sml.get_plural(&self.current_lang, singular, count) {
            return result;
        }

        
        if count == 1 { singular } else { plural }
    }

    
    pub fn set_language(&mut self, lang: &str) {
        self.current_lang = lang.to_string();
    }

    
    pub fn current_language(&self) -> &str {
        &self.current_lang
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sml() {
        let content = r#"
[author] -> Atrium Team

[Initial]
greeting
farewell

[Language, en]
name -> English
greeting = Hello
farewell = Goodbye

[Language, ru]
name -> Русский
greeting = Привет
farewell = До свидания
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        assert_eq!(file.metadata.get("author"), Some(&"Atrium Team".to_string()));
        assert!(file.initial_keys.contains("greeting"));
        assert!(file.initial_keys.contains("farewell"));

        
        assert!(file.languages.contains_key("en"));
        assert!(file.languages.contains_key("ru"));
    }

    #[test]
    fn test_parse_pluralization() {
        let content = r#"
[Initial]
items

[Language, ru]
items.one|few|many = один предмет | несколько предмета | много предметов
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        match file.get_translation("ru", "items") {
            Some(Translation::Plural { forms, values }) => {
                assert_eq!(forms, &vec!["one".to_string(), "few".to_string(), "many".to_string()]);
                assert_eq!(values.len(), 3);
            }
            _ => panic!("Expected plural translation"),
        }
    }

    #[test]
    fn test_parse_contextual() {
        let content = r#"
[Initial]
button

[Language, en]
button -> label = Click
button -> tooltip = Click me
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        match file.get_translation("en", "button -> label") {
            Some(Translation::Contextual { context, value }) => {
                assert_eq!(context, "label");
                assert_eq!(value, "Click");
            }
            _ => panic!("Expected contextual translation"),
        }
    }

    #[test]
    fn test_parse_multiline() {
        let content = r#"
[Initial]
description

[Language, en]
description = This is a simple description value.
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        let value = file.get_string("en", "description").unwrap();
        assert!(value.contains("description"));
    }

    #[test]
    fn test_parse_alias() {
        let content = r#"
[Initial]
hello
greeting

[Language, en]
hello = Hello
greeting = hello
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        match file.get_translation("en", "greeting") {
            Some(Translation::Alias(target)) => {
                assert_eq!(target, "hello");
            }
            _ => panic!("Expected alias translation"),
        }
    }

    #[test]
    fn test_validate() {
        let mut file = SmlFile::new();
        file.add_initial_key("greeting");
        file.add_translation("en", "greeting", Translation::String("Hello".to_string())).unwrap();

        assert!(file.validate().is_ok());

        
        file.add_translation("en", "unknown", Translation::String("Unknown".to_string())).unwrap();
        assert!(file.validate().is_err());
    }

    #[test]
    fn test_serialize() {
        let mut file = SmlFile::with_version("1.0");
        file.add_initial_key("test");
        file.add_translation("en", "test", Translation::String("Test".to_string())).unwrap();

        let serialized = file.serialize();
        
        assert!(serialized.contains("[version 1.0]"));
        assert!(serialized.contains("[Initial]"));
        assert!(serialized.contains("[Language, en]"));
        assert!(serialized.contains("test = Test"));
    }

    #[test]
    fn test_localization() {
        let mut file = SmlFile::new();
        file.add_initial_key("hello");
        file.add_translation("en", "hello", Translation::String("Hello".to_string())).unwrap();
        file.add_translation("ru", "hello", Translation::String("Привет".to_string())).unwrap();

        let loc = Localization::new(file, "ru");
        assert_eq!(loc.get("hello"), "Привет");

        let mut loc = Localization::new(SmlFile::new(), "en");
        assert_eq!(loc.get("hello"), "hello"); 
    }

    #[test]
    fn test_parse_imports() {
        let content = r#"
[Initial]
key1

[Language, en] -> common.sml, extra.sml
"#;

        let parser = SmlParser::new().unwrap();
        let file = parser.parse(content).unwrap();

        let en = file.languages.get("en").unwrap();
        assert!(en.imports.contains(&"common.sml".to_string()));
        assert!(en.imports.contains(&"extra.sml".to_string()));
    }
}
