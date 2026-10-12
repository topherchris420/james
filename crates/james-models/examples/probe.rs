//! Ask a local model server what James would see, from any OS:
//!
//! ```text
//! cargo run -p james-models --example probe -- <provider> [port] [--chat "question"] [--image file.png]
//! ```
//!
//! `<provider>` is `lmstudio`, `ollama`, `llamacpp` or `openai`. Without a
//! port, the provider's default is used. The probe lists models, reports what
//! the server says the first chat model can do, and, only with `--chat`,
//! sends one real question and streams the answer. Everything stays on
//! loopback, through the same client James uses.

use james_models::{ChatMessage, LocalClient, LocalProvider, StreamEvent, preferred_chat_model};
use std::io::Write;
use std::time::Instant;

fn main() {
    let mut args = std::env::args().skip(1);
    let provider = match args.next().as_deref() {
        Some("lmstudio") => LocalProvider::LmStudio,
        Some("ollama") => LocalProvider::Ollama,
        Some("llamacpp") => LocalProvider::LlamaCpp,
        Some("openai") => LocalProvider::OpenAiCompatible,
        _ => {
            eprintln!(
                "usage: probe <lmstudio|ollama|llamacpp|openai> [port] [--model id] [--chat \"question\"] [--image file.png]"
            );
            std::process::exit(2);
        }
    };
    let mut port = provider.default_port();
    let (mut question, mut image, mut model) = (None, None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--chat" => question = args.next(),
            "--image" => image = args.next(),
            "--model" => model = args.next(),
            other => match other.parse() {
                Ok(value) => port = value,
                Err(_) => {
                    eprintln!("unknown argument {other}");
                    std::process::exit(2);
                }
            },
        }
    }
    let client = LocalClient::for_provider(provider, port).expect("nonzero port");
    println!("{} on 127.0.0.1:{port}", provider.label());
    let list = match client.list() {
        Ok(list) => list,
        Err(error) => {
            println!("  model list failed ({:?}): {error}", error.problem());
            std::process::exit(1);
        }
    };
    println!("  models: {:?}", list.models);
    if !list.remote.is_empty() {
        println!(
            "  run elsewhere, according to the server: {:?}",
            list.remote
        );
    }
    let Some(model) = model.or_else(|| preferred_chat_model(&list.models, &list.remote).cloned())
    else {
        println!("  no local chat model to probe");
        return;
    };
    let started = Instant::now();
    let info = client.model_info(&model);
    println!("  {model}: {info:?} ({} ms)", started.elapsed().as_millis());
    let Some(question) = question else {
        return;
    };
    let png = image.map(|path| std::fs::read(path).expect("readable image"));
    let started = Instant::now();
    let mut first = None;
    let result = client.chat_stream(
        &model,
        &[ChatMessage::new("user", question)],
        png.as_deref(),
        |event| match event {
            StreamEvent::Connected(_) => {}
            StreamEvent::Thinking => {
                print!("·");
                let _ = std::io::stdout().flush();
            }
            StreamEvent::Delta(text) => {
                first.get_or_insert_with(|| started.elapsed());
                print!("{text}");
                let _ = std::io::stdout().flush();
            }
        },
    );
    println!();
    match result {
        Ok(reply) => println!(
            "  streamed {} bytes; first text after {} ms, done after {} ms",
            reply.len(),
            first.unwrap_or_default().as_millis(),
            started.elapsed().as_millis()
        ),
        Err(error) => println!("  failed ({:?}): {error}", error.problem()),
    }
}
