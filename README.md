# Rust Template System

A flexible and type-safe template system for Rust code generation and execution. This system allows you to create, manage, and execute Rust code templates with customizable placeholders and dependencies.

## Features

- Template creation from strings or files
- Placeholder management with validation
- Template assembly and composition
- Optional template execution using rust-script
- Dependency management for executed templates
- Error handling with custom error types

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
template-rs = "0.1.0"

# Optional: Enable execution features
[features]
execute = ["tempfile", "which"]
```

## Usage

### Basic Template Creation

```rust
use template_rs::Template;

let mut template = Template::new("fn main() { println!(\"@[message]@\"); }")?;
template.set("message", "Hello, World!")?;
let rendered = template.render()?;
```

### Loading Templates from Files

```rust
use template_rs::Template;

let mut template = Template::from_file("templates/hello.tmrs")?;
template.set("message", "Hello from file!")?;
let rendered = template.render()?;
```

### Template Execution

When the `execute` feature is enabled, you can run templates without compiling them using rust-script (This is slower at runtime than compiled rust but it can be handy in situations where real time execution is required). In order for this to work the end user will need rust script installed with:
```bash
cargo install rust-script
```

Usage:

```rust
use template_rs::TemplateRef;

let template = Template::new(r#"
    fn main() {
        let sum: @[number_type]@ = vec![1, 2, 3, 4, 5].iter().sum();
        println!("Sum: {}", sum);
    }
"#)?;

let mut template_ref = TemplateRef::new(template)
    .with_dependency("num = \"0.4\"");

template_ref.template.set("number_type", "i32")?;
let output = template_ref.execute().await?;
```

### Template Assembly

Combine multiple templates:

```rust
use template_rs::TemplateAssembler;

let mut assembler = TemplateAssembler::new();
assembler.add_template(template1);
assembler.add_template(template2);
assembler.set_global("shared_value", "42")?;
let combined = assembler.render_all()?;
```

## Placeholder Syntax

Placeholders use the format `@[placeholder_name]@`:

```rust
let template = r#"
    fn @[function_name]@() {
        println!("@[message]@");
    }
"#;
```

## Error Handling

The system provides custom error types for different failure scenarios:

- `TemplateError::Io`: File system related errors
- `TemplateError::Parse`: Template parsing errors
- `TemplateError::MissingPlaceholder`: Unset placeholder errors
- `TemplateError::InvalidSyntax`: Template syntax errors
- `TemplateError::ExecutionError`: Template execution errors

## Examples

### Data Processing Template

```rust
let template = r#"
#[derive(Debug)]
struct @[struct_name]@ {
    @[fields]@
}

fn main() {
    let data = @[struct_name]@ {
        @[field_values]@
    };
    println!("{:?}", data);
}
"#;

let mut t = Template::new(template)?;
t.set("struct_name", "User")?;
t.set("fields", "name: String,\n    age: u32")?;
t.set("field_values", r#"name: "Alice".to_string(),
    age: 30"#)?;
```

### CLI Application Template

```rust
let template = r#"
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "@[description]@")]
struct Args {
    @[arguments]@
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);
}
"#;
```

## Dependencies

Required dependencies:
- `thiserror`: Error handling
- `regex`: Placeholder extraction

Optional dependencies (with `execute` feature):
- `tempfile`: Temporary file handling
- `which`: Binary detection

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Safety Notes

- When using template execution, ensure input validation is performed
- Be cautious with user-provided content in templates