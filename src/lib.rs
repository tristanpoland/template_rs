use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Missing placeholder: {0}")]
    MissingPlaceholder(String),
    #[error("Invalid template syntax: {0}")]
    InvalidSyntax(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

pub type Result<T> = std::result::Result<T, TemplateError>;

/// Reference to a template that can be executed
#[derive(Debug, Clone)]
pub struct TemplateRef {
    pub template: Template,
    pub dependencies: Vec<String>,
}

impl TemplateRef {
    /// Create a new executable template reference
    pub fn new(template: Template) -> Self {
        Self {
            template,
            dependencies: Vec::new(),
        }
    }

    /// Add a dependency that will be included in the rust-script execution
    pub fn with_dependency(mut self, dependency: &str) -> Self {
        self.dependencies.push(dependency.to_string());
        self
    }

    /// Execute the template with rust-script after rendering
    #[cfg(feature = "execute")]
    pub async fn execute(&self) -> Result<String> {
        use std::process::Command;
        use tempfile::NamedTempFile;
        use std::io::Write;
        use which::which;

        // Check if rust-script is installed
        which("rust-script").map_err(|_| {
            TemplateError::ExecutionError("rust-script not found. Please install it with: cargo install rust-script".into())
        })?;

        // Render the template
        let rendered = self.template.render()?;

        // Create a temporary file with the rendered content
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| TemplateError::ExecutionError(format!("Failed to create temp file: {}", e)))?;
        
        // Add dependencies as script dependencies
        let mut script_content = String::new();
        for dep in &self.dependencies {
            script_content.push_str(&format!("//! ```cargo\n//! [dependencies]\n//! {} \n//! ```\n", dep));
        }
        script_content.push_str(&rendered);

        temp_file.write_all(script_content.as_bytes())
            .map_err(|e| TemplateError::ExecutionError(format!("Failed to write temp file: {}", e)))?;

        // Execute the script
        let output = Command::new("rust-script")
            .arg(temp_file.path())
            .output()
            .map_err(|e| TemplateError::ExecutionError(format!("Failed to execute script: {}", e)))?;

        if !output.status.success() {
            return Err(TemplateError::ExecutionError(
                String::from_utf8_lossy(&output.stderr).into_owned()
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    content: String,
    placeholders: HashMap<String, String>,
    path: Option<PathBuf>,
}

impl Template {
    /// Create a new template from a string
    pub fn new(content: &str) -> Result<Self> {
        let placeholders = Self::extract_placeholders(content)?;
        Ok(Self {
            content: content.to_string(),
            placeholders,
            path: None,
        })
    }

    /// Load a template from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        let mut template = Self::new(&content)?;
        template.path = Some(path.as_ref().to_path_buf());
        Ok(template)
    }

    /// Extract placeholders from template content
    fn extract_placeholders(content: &str) -> Result<HashMap<String, String>> {
        let mut placeholders = HashMap::new();
        // Pattern for @[placeholder_name]@ - using @ symbols which are invalid in Rust
        let pattern = regex::Regex::new(r"@\[([^]]+)\]@").unwrap();
        
        for capture in pattern.captures_iter(content) {
            let placeholder = capture.get(1).unwrap().as_str().trim();
            placeholders.insert(placeholder.to_string(), String::new());
        }
        
        Ok(placeholders)
    }

    /// Set a value for a placeholder
    pub fn set(&mut self, placeholder: &str, value: &str) -> Result<()> {
        if !self.placeholders.contains_key(placeholder) {
            return Err(TemplateError::MissingPlaceholder(placeholder.to_string()));
        }
        self.placeholders.insert(placeholder.to_string(), value.to_string());
        Ok(())
    }

    /// Render the template with current placeholder values
    pub fn render(&self) -> Result<String> {
        let mut result = self.content.clone();
        
        for (placeholder, value) in &self.placeholders {
            let pattern = format!("@[{}]@", placeholder);
            if value.is_empty() {
                return Err(TemplateError::MissingPlaceholder(placeholder.clone()));
            }
            result = result.replace(&pattern, value);
        }
        
        Ok(result)
    }
}

#[derive(Debug)]
pub struct TemplateAssembler {
    templates: Vec<Template>,
}

impl TemplateAssembler {
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    /// Add a template to the assembler
    pub fn add_template(&mut self, template: Template) {
        self.templates.push(template);
    }

    /// Set a value for a placeholder across all templates
    pub fn set_global(&mut self, placeholder: &str, value: &str) -> Result<()> {
        for template in &mut self.templates {
            if template.placeholders.contains_key(placeholder) {
                template.set(placeholder, value)?;
            }
        }
        Ok(())
    }

    /// Render all templates and combine them
    pub fn render_all(&self) -> Result<String> {
        let mut result = String::new();
        for template in &self.templates {
            result.push_str(&template.render()?);
            result.push('\n');
        }
        Ok(result)
    }
}

#[cfg(all(test, feature = "execute"))]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_basic_template() -> Result<()> {
        let mut template = Template::new("fn main() { println!(\"@[message]@\"); }")?;
        template.set("message", "Hello, World!")?;
        assert_eq!(
            template.render()?,
            "fn main() { println!(\"Hello, World!\"); }"
        );
        Ok(())
    }

    #[test]
    fn test_template_execution() {
        let rt = Runtime::new().unwrap();
        
        let template = Template::new(r#"
            fn main() {
                let numbers = vec![1, 2, 3, 4, 5];
                let sum: @[number_type]@ = numbers.iter().sum();
                println!("Sum: {}", sum);
            }
        "#).unwrap();

        let mut template_ref = TemplateRef::new(template)
            .with_dependency("num = \"0.4\"");
        
        template_ref.template.set("number_type", "i32").unwrap();
        
        let output = rt.block_on(template_ref.execute()).unwrap();
        assert!(output.contains("Sum: 15"));
    }

    #[test]
    fn test_missing_placeholder() {
        let template = Template::new("@[missing]@").unwrap();
        assert!(matches!(
            template.render(),
            Err(TemplateError::MissingPlaceholder(_))
        ));
    }
}