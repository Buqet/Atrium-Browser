



use atrium_core::WebHtmlParser;
use std::env;
use std::fs;
use std::io::{self, Write, BufRead};

fn main() {
    println!("🔍 Atrium HTML Parser Test Tool");
    println!("═══════════════════════════════\n");

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--html" => test_html(),
            "--help" | "-h" => {
                print_help();
            }
            path => {
                test_file(path);
            }
        }
    } else {
        interactive_mode();
    }
}

fn print_help() {
    println!(r#"
HTML Parser Test Tool

Usage:
  parser-test [OPTIONS] [FILE]

Options:
  --html     Test HTML parser interactively
  --help     Show this help message

Examples:
  parser-test --html
  parser-test test.html
"#);
}

fn interactive_mode() {
    println!("Enter HTML code (empty line to finish):");

    let mut html = String::new();
    read_multiline_input(&mut html);

    let mut parser = WebHtmlParser::new();
    match parser.parse(&html) {
        Ok(nodes) => {
            println!("\n✅ Parsed successfully!");
            println!("   Nodes: {}", count_nodes(&nodes));
        }
        Err(e) => {
            println!("\n❌ Parse error: {}", e);
        }
    }

    parser.print_unsupported();
}

fn test_html() {
    println!("\n📄 HTML Parser Test");
    println!("───────────────────");
    println!("Enter HTML code (empty line to finish):");

    let mut html = String::new();
    read_multiline_input(&mut html);

    let mut parser = WebHtmlParser::new();
    match parser.parse(&html) {
        Ok(nodes) => {
            println!("\n✅ Parsed successfully!");
            println!("   Nodes: {}", count_nodes(&nodes));
        }
        Err(e) => {
            println!("\n❌ Parse error: {}", e);
        }
    }

    parser.print_unsupported();
}

fn test_file(path: &str) {
    println!("\n📁 Testing file: {}", path);
    println!("─────────────────────────");

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Error reading file: {}", e);
            return;
        }
    };

    let mut parser = WebHtmlParser::new();
    match parser.parse(&content) {
        Ok(nodes) => {
            println!("✅ HTML parsed: {} nodes", count_nodes(&nodes));
        }
        Err(e) => {
            println!("❌ Parse error: {}", e);
        }
    }
    parser.print_unsupported();
}

fn count_nodes(nodes: &[atrium_core::html::HtmlNode]) -> usize {
    let mut count = nodes.len();
    for node in nodes {
        if let atrium_core::html::HtmlNode::Element { children, .. } = node {
            count += count_nodes(children);
        }
    }
    count
}

fn read_multiline_input(output: &mut String) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if let Ok(l) = line {
            if l.trim().is_empty() {
                break;
            }
            output.push_str(&l);
            output.push('\n');
        }
    }
}
