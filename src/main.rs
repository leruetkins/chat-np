use anyhow::Result;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::params::LlamaModelParams,
    model::LlamaModel,
    context::LlamaContext,
};
use std::io::{self, Write};
use std::fs;
use std::sync::{Arc, Mutex};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
    execute,
    cursor,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

fn select_model(models: &[String]) -> Result<Option<String>> {
    // Clear screen
    // execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::All), cursor::MoveTo(0, 0))?;
    
    // Create options list (models + exit)
    let mut model_options: Vec<String> = models.to_vec();
    model_options.push("✕ Выход".to_string());
    
    println!("Найдено моделей: {}\n", models.len());
    println!("Используйте ↑/↓ для навигации, Enter для выбора\n");
    
    let mut selected = 0;
    
    // Display list
    let mut stdout = io::stdout();
    let mut display_list = |selected_idx: usize, stdout: &mut std::io::Stdout| -> Result<()> {
        execute!(stdout, cursor::MoveUp(model_options.len() as u16), cursor::MoveToColumn(0))?;
        for (i, model) in model_options.iter().enumerate() {
            let line = if i == selected_idx {
                format!("→ {}", model)
            } else {
                format!("  {}", model)
            };
            // Clear line and print
            execute!(
                stdout,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                crossterm::style::Print(line),
                cursor::MoveToNextLine(1)
            )?;
        }
        Ok(())
    };

    // Initial display
    for model in model_options.iter() {
        println!("  {}", model);
    }
    display_list(selected, &mut stdout)?;


    execute!(stdout, cursor::Hide)?;
    enable_raw_mode()?;

    // Clear event buffer before starting
    while event::poll(std::time::Duration::from_millis(10))? {
        let _ = event::read()?;
    }

    loop {
        if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
            // Ignore key release and repeat events
            if kind != event::KeyEventKind::Press {
                continue;
            }
            
            match code {
                KeyCode::Up => {
                    if selected > 0 {
                        selected -= 1;
                        display_list(selected, &mut stdout)?;
                    }
                }
                KeyCode::Down => {
                    if selected < model_options.len() - 1 {
                        selected += 1;
                        display_list(selected, &mut stdout)?;
                    }
                }
                KeyCode::Enter => {
                    disable_raw_mode()?;
                    execute!(stdout, cursor::Show)?;
                    println!();
                    
                    // Check if "Exit" is selected
                    if selected == model_options.len() - 1 {
                        println!("Выход из программы.\n");
                        return Ok(None);
                    }
                    
                    return Ok(Some(models[selected].clone()));
                }
                KeyCode::Esc => {
                    disable_raw_mode()?;
                    execute!(stdout, cursor::Show)?;
                    println!("\nВыход из программы.\n");
                    return Ok(None);
                }
                _ => {}
            }
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct Preset {
    name: String,
    description: String,
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    instruction: Option<String>,
    #[serde(default)]
    examples: Option<Vec<PromptExample>>,
    #[serde(default)]
    negative_prompt: Option<String>,
    #[serde(default)]
    response_format: Option<String>,
    max_tokens: usize,
    stop_on_newline: bool,
    #[serde(default)]
    include_current_date: bool,
}

#[derive(Deserialize, Serialize, Clone)]
struct PromptExample {
    input: String,
    output: String,
}

impl Preset {
    fn build_full_prompt(&self, user_input: &str) -> String {
        let mut parts = Vec::new();
        
        // System prompt (main role)
        if !self.system_prompt.is_empty() {
            parts.push(self.system_prompt.clone());
        }
        
        // Current date (if required)
        if self.include_current_date {
            use chrono::{Local, Datelike};
            let now = Local::now();
            let date_str = now.format("%d.%m.%Y").to_string();
            let weekday = match now.date_naive().weekday() {
                chrono::Weekday::Mon => "понедельник",
                chrono::Weekday::Tue => "вторник",
                chrono::Weekday::Wed => "среда",
                chrono::Weekday::Thu => "четверг",
                chrono::Weekday::Fri => "пятница",
                chrono::Weekday::Sat => "суббота",
                chrono::Weekday::Sun => "воскресенье",
            };
            parts.push(format!("Сегодня: {} ({})", date_str, weekday));
        }
        
        // Instruction (detailed instructions)
        if let Some(instruction) = &self.instruction {
            parts.push(instruction.clone());
        }
        
        // Examples (few-shot examples)
        if let Some(examples) = &self.examples {
            if !examples.is_empty() {
                let examples_text = examples.iter()
                    .map(|ex| format!("Вход: {}\nВыход: {}", ex.input, ex.output))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                parts.push(format!("Примеры:\n{}", examples_text));
            }
        }
        
        // Negative prompt (what NOT to do)
        if let Some(negative) = &self.negative_prompt {
            parts.push(format!("НЕ ДЕЛАЙ: {}", negative));
        }
        
        // Response format (output format)
        if let Some(format) = &self.response_format {
            parts.push(format!("Формат ответа: {}", format));
        }
        
        // Combine everything together
        let full_system = parts.join("\n\n");
        
        // Add user input
        format!("{}\n\nВход: {}\nВыход:", full_system, user_input)
    }
}

#[derive(Deserialize, Serialize)]
struct PresetsConfig {
    presets: Vec<Preset>,
}

#[derive(Deserialize)]
struct ChatRequest {
    prompt: String,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    max_tokens: Option<usize>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    preset: Option<String>,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Serialize)]
struct ModelsResponse {
    models: Vec<String>,
}

#[derive(Serialize)]
struct PresetsResponse {
    presets: Vec<PresetInfo>,
}

#[derive(Serialize)]
struct PresetInfo {
    name: String,
    description: String,
}

struct AppState {
    backend: LlamaBackend,
    available_models: Vec<String>,
}

fn load_presets() -> Vec<Preset> {
    match fs::read_to_string("presets.json") {
        Ok(content) => {
            match serde_json::from_str::<PresetsConfig>(&content) {
                Ok(config) => config.presets,
                Err(e) => {
                    eprintln!("Ошибка парсинга presets.json: {}", e);
                    vec![]
                }
            }
        }
        Err(_) => {
            eprintln!("Файл presets.json не найден");
            vec![]
        }
    }
}

async fn models_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    (StatusCode::OK, Json(ModelsResponse { 
        models: state.available_models.clone() 
    }))
}

async fn presets_handler(_state: State<Arc<AppState>>) -> impl IntoResponse {
    let presets = load_presets();
    let presets_info: Vec<PresetInfo> = presets.iter()
        .map(|p| PresetInfo {
            name: p.name.clone(),
            description: p.description.clone(),
        })
        .collect();
    
    (StatusCode::OK, Json(PresetsResponse { presets: presets_info }))
}

async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    let model_name = req.model.as_ref()
        .or_else(|| state.available_models.first())
        .cloned()
        .unwrap_or_default();

    if !state.available_models.contains(&model_name) {
        return (StatusCode::BAD_REQUEST, Json(ChatResponse { 
            response: format!("Model '{}' not found. Available models: {:?}", model_name, state.available_models) 
        }));
    }

    // Load presets on each request (so changes apply without restart)
    let presets = load_presets();
    
    // Determine parameters from preset or request
    let (prompt, max_tokens, stop_on_newline) = if let Some(preset_name) = &req.preset {
        if let Some(preset) = presets.iter().find(|p| p.name == *preset_name) {
            (
                preset.build_full_prompt(&req.prompt),
                req.max_tokens.unwrap_or(preset.max_tokens),
                preset.stop_on_newline,
            )
        } else {
            return (StatusCode::BAD_REQUEST, Json(ChatResponse { 
                response: format!("Preset '{}' not found. Use /presets to see available presets", preset_name) 
            }));
        }
    } else {
        let system_prompt = req.system_prompt.clone().unwrap_or_default();
        let prompt = if system_prompt.is_empty() {
            req.prompt.clone()
        } else {
            format!("{}\n\n{}", system_prompt, req.prompt)
        };
        (
            prompt,
            req.max_tokens.unwrap_or(100),
            false,
        )
    };

    // Load model for each request (simplified version)
    let model_params = LlamaModelParams::default().with_n_gpu_layers(0);
    let model = match LlamaModel::load_from_file(&state.backend, &model_name, &model_params) {
        Ok(m) => m,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
            response: format!("Failed to load model: {}", e) 
        })),
    };

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(std::num::NonZero::new(2048));
    
    let mut ctx = match model.new_context(&state.backend, ctx_params) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
            response: format!("Failed to create context: {}", e) 
        })),
    };

    ctx.clear_kv_cache();

    let tokens = match model.str_to_token(&prompt, llama_cpp_2::model::AddBos::Always) {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
            response: format!("Tokenization error: {}", e) 
        })),
    };

    let mut batch = LlamaBatch::new(512, 1);
    let last_index = tokens.len() - 1;
    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == last_index;
        if let Err(e) = batch.add(*token, i as i32, &[0], is_last) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
                response: format!("Batch error: {}", e) 
            }));
        }
    }

    if let Err(e) = ctx.decode(&mut batch) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
            response: format!("Decode error: {}", e) 
        }));
    }

    let mut result = String::new();
    let mut pos = tokens.len() as i32;

    for _ in 0..max_tokens {
        let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
        
        let token = candidates
            .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap())
            .map(|c| c.id())
            .unwrap();

        if token == model.token_eos() {
            break;
        }

        let piece = match model.token_to_str(token, llama_cpp_2::model::Special::Tokenize) {
            Ok(p) => p,
            Err(_) => {
                // Skip tokens with decoding errors (incomplete UTF-8 sequences)
                batch.clear();
                if let Err(e) = batch.add(token, pos, &[0], true) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
                        response: format!("Batch add error: {}", e) 
                    }));
                }
                pos += 1;
                
                if let Err(e) = ctx.decode(&mut batch) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
                        response: format!("Decode error: {}", e) 
                    }));
                }
                continue;
            }
        };
        
        result.push_str(&piece);

        if stop_on_newline && result.contains('\n') {
            break;
        }

        batch.clear();
        if let Err(e) = batch.add(token, pos, &[0], true) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
                response: format!("Batch add error: {}", e) 
            }));
        }
        pos += 1;
        
        if let Err(e) = ctx.decode(&mut batch) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ChatResponse { 
                response: format!("Decode error: {}", e) 
            }));
        }
    }

    (StatusCode::OK, Json(ChatResponse { response: result.trim().to_string() }))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let server_mode = args.len() > 1 && args[1] == "--server";
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("CHAT-NP v{}", VERSION);
    println!("Local LLM Chat with Preset System & REST API");
    println!();
    
    // Find all .gguf files in current directory
    let mut models = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "gguf" {
                if let Some(name) = path.file_name() {
                    models.push(name.to_string_lossy().to_string());
                }
            }
        }
    }
    
    if models.is_empty() {
        println!("Ошибка: не найдено ни одного .gguf файла в текущей директории!");
        println!("Положите GGUF модель рядом с программой.");
        return Ok(());
    }

    let backend = LlamaBackend::init()?;

    if server_mode {
        println!("Найдено моделей: {}", models.len());
        for model in &models {
            println!("  - {}", model);
        }
        
        let presets = load_presets();
        println!("\nЗагружено пресетов: {}", presets.len());
        for preset in &presets {
            println!("  - {} ({})", preset.name, preset.description);
        }
        
        println!("\nПресеты будут автоматически перечитываться из presets.json при каждом запросе");
        println!("Запуск веб-сервера...\n");
        
        let state = Arc::new(AppState {
            backend,
            available_models: models.clone(),
        });

        let app = Router::new()
            .route("/models", axum::routing::get(models_handler))
            .route("/presets", axum::routing::get(presets_handler))
            .route("/chat", post(chat_handler))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = "127.0.0.1:3000";
        println!("Сервер запущен на http://{}", addr);
        println!("\nДоступные эндпоинты:");
        println!("  GET  /models  - список доступных моделей");
        println!("  GET  /presets - список доступных пресетов");
        println!("  POST /chat    - отправка запроса к модели");
        println!("\nПримеры запросов:");
        println!(r#"curl http://127.0.0.1:3000/models"#);
        println!(r#"curl http://127.0.0.1:3000/presets"#);
        println!(r#"curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d '{{"prompt": "iPhone 15", "preset": "price_classifier"}}'"#);
        println!(r#"curl -X POST http://127.0.0.1:3000/chat -H "Content-Type: application/json" -d '{{"prompt": "Что такое Rust?", "preset": "assistant"}}'"#);
        println!("\nДля остановки нажмите Ctrl+C\n");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        
        return Ok(());
    }
    
    // Main loop to allow returning to model selection
    loop {
        let model_path = if models.len() == 1 {
            println!("Найдена модель: {}\n", models[0]);
            models[0].clone()
        } else {
            match select_model(&models)? {
                Some(path) => path,
                None => return Ok(()), // Exit program
            }
        };
        
        println!("Загрузка модели {}...", model_path);
        
        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(0);
        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)?;

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(std::num::NonZero::new(2048));
    
    let mut ctx = model.new_context(&backend, ctx_params)?;
    
    // Load presets and let user choose
    let presets = load_presets();
    
    let selected_preset = if presets.is_empty() {
        println!("Пресеты не найдены. Работа в режиме свободного чата.\n");
        None
    } else if presets.len() == 1 {
        println!("Найден пресет: {} - {}\n", presets[0].name, presets[0].description);
        Some(presets[0].clone())
    } else {
        // Clear screen
        execute!(io::stdout(), crossterm::terminal::Clear(crossterm::terminal::ClearType::All), cursor::MoveTo(0, 0))?;
        
        // Create options list (presets + control options)
        let mut preset_options: Vec<String> = presets.iter()
            .map(|p| format!("{} - {}", p.name, p.description))
            .collect();
        preset_options.push("Без пресета (свободный чат)".to_string());
        preset_options.push("← Назад (выбрать другую модель)".to_string());
        preset_options.push("✕ Выход".to_string());
        
        println!("Найдено пресетов: {}\n", presets.len());
        println!("Используйте ↑/↓ для навигации, Enter для выбора\n");
        
        let mut selected = 0;
        let mut stdout = io::stdout();
        
        let mut display_list = |selected_idx: usize, stdout: &mut std::io::Stdout| -> Result<()> {
            execute!(stdout, cursor::MoveUp(preset_options.len() as u16), cursor::MoveToColumn(0))?;
            for (i, option) in preset_options.iter().enumerate() {
                let line = if i == selected_idx {
                    format!("→ {}", option)
                } else {
                    format!("  {}", option)
                };
                execute!(
                    stdout,
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                    crossterm::style::Print(line),
                    cursor::MoveToNextLine(1)
                )?;
            }
            Ok(())
        };

        // Initial display
        for option in preset_options.iter() {
            println!("  {}", option);
        }
        display_list(selected, &mut stdout)?;

        execute!(stdout, cursor::Hide)?;
        enable_raw_mode()?;

        // Clear event buffer before starting
        while event::poll(std::time::Duration::from_millis(10))? {
            let _ = event::read()?;
        }

        let choice = loop {
            if let Event::Key(KeyEvent { code, kind, .. }) = event::read()? {
                // Ignore key release and repeat events
                if kind != event::KeyEventKind::Press {
                    continue;
                }
                
                match code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                            display_list(selected, &mut stdout)?;
                        }
                    }
                    KeyCode::Down => {
                        if selected < preset_options.len() - 1 {
                            selected += 1;
                            display_list(selected, &mut stdout)?;
                        }
                    }
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        execute!(stdout, cursor::Show)?;
                        println!();
                        break selected;
                    }
                    KeyCode::Esc => {
                        disable_raw_mode()?;
                        execute!(stdout, cursor::Show)?;
                        println!("\nВыход из программы.\n");
                        return Ok(()); // Exit program
                    }
                    _ => {}
                }
            }
        };

        disable_raw_mode()?;
        execute!(stdout, cursor::Show)?;
        println!();

        // Handle selection
        if choice == preset_options.len() - 1 {
            // Exit
            println!("Выход из программы.\n");
            return Ok(());
        } else if choice == preset_options.len() - 2 {
            // Back - return to model selection
            println!("Возврат к выбору модели...\n");
            continue; // Continue loop, return to model selection
        } else if choice == preset_options.len() - 3 {
            // No preset
            println!("Режим свободного чата.\n");
            None
        } else {
            // Preset selected
            let preset = presets[choice].clone();
            println!("Выбран пресет: {} - {}\n", preset.name, preset.description);
            Some(preset)
        }
    };

        if let Some(ref preset) = selected_preset {
            println!("Примеры запросов для этого пресета:");
            if let Some(examples) = &preset.examples {
                for (i, ex) in examples.iter().take(3).enumerate() {
                    println!("  {}. {}", i + 1, ex.input);
                }
            }
            println!();
        } else {
            println!("Режим свободного чата. Введите ваш запрос.\n");
        }

        loop {
            print!("Запрос (или 'exit'): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }
            if input.eq_ignore_ascii_case("exit") {
                break;
            }

            // Clear KV cache before each request
            ctx.clear_kv_cache();

            let prompt = if let Some(ref preset) = selected_preset {
                preset.build_full_prompt(input)
            } else {
                input.to_string()
            };

            let tokens = model.str_to_token(&prompt, llama_cpp_2::model::AddBos::Always)?;

            let mut batch = LlamaBatch::new(512, 1);
            let last_index = tokens.len() - 1;
            for (i, token) in tokens.iter().enumerate() {
                let is_last = i == last_index;
                batch.add(*token, i as i32, &[0], is_last)?;
            }

            ctx.decode(&mut batch)?;

            let mut result = String::new();
            let mut pos = tokens.len() as i32;
            
            // Use max_tokens and stop_on_newline from preset if available
            let max_tokens = if let Some(ref preset) = selected_preset {
                preset.max_tokens
            } else {
                100
            };
            
            let stop_on_newline = if let Some(ref preset) = selected_preset {
                preset.stop_on_newline
            } else {
                false
            };
            
            for _ in 0..max_tokens {
                let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
                
                let token = candidates
                    .max_by(|a, b| a.logit().partial_cmp(&b.logit()).unwrap())
                    .map(|c| c.id())
                    .unwrap();

                if token == model.token_eos() {
                    break;
                }

                let piece = model.token_to_str(token, llama_cpp_2::model::Special::Tokenize)?;
                result.push_str(&piece);

                // Stop on newline if configured
                if stop_on_newline && result.contains('\n') {
                    break;
                }
                
                // Stop after complete JSON object (for JSON presets)
                let trimmed = result.trim();
                if trimmed.starts_with('{') && trimmed.ends_with('}') {
                    // Check if it's a complete JSON by counting braces
                    let open_braces = trimmed.chars().filter(|&c| c == '{').count();
                    let close_braces = trimmed.chars().filter(|&c| c == '}').count();
                    if open_braces == close_braces && open_braces > 0 {
                        break;
                    }
                }

                batch.clear();
                batch.add(token, pos, &[0], true)?;
                pos += 1;
                ctx.decode(&mut batch)?;
            }

            // Trim whitespace
            println!("→ {}\n", result.trim());
        }
        
        // Inner loop finished, continue outer loop for new model selection
    }
}
