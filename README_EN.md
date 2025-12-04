# CHAT-NP

English | [Русский](README.md)

Local chatbot based on GGUF language models with preset support and REST API.

## Description

CHAT-NP is an application for working with local language models in GGUF format. The program provides two operating modes:

1. **Interactive mode** — console chat with model and preset selection via convenient menu
2. **Server mode** — REST API for integration with other applications

## Key Features

### Preset System

Presets are ready-made configurations for specialized tasks. Each preset includes:

- **System prompt** — role and context for the model
- **Instruction** — detailed task execution instructions
- **Examples** — input data and expected output examples (few-shot learning)
- **Negative prompt** — what the model should NOT do
- **Response format** — output format specification
- **Generation parameters** — max_tokens, stop_on_newline
- **Current date** — automatic date insertion for time-context tasks

### Built-in Presets

1. **price_classifier** — product price classification (cheap/expensive) with explanation
2. **assistant** — friendly helper for general questions
3. **translator_ru_en** — Russian to English translation
4. **code_reviewer** — code analysis and review
5. **sentiment** — text sentiment analysis (positive/negative/neutral)
6. **summarizer** — text summarization
7. **date_extractor** — date extraction from text with automatic relative date calculation

## Installation

### Requirements

- Rust 1.70+
- GGUF model (e.g., Qwen, LLaMA, Mistral)

### Build

```bash
cargo build --release
```

### Model Setup

1. Download a GGUF model (e.g., from [Hugging Face](https://huggingface.co/models?library=gguf))
2. Place the `.gguf` file in the program's root directory (next to the executable)
3. The program will automatically find all `.gguf` files on startup

**Recommended models:**
- [Qwen3-1.7B-Q4_K_M.gguf](https://huggingface.co/Qwen/Qwen3-1.7B-GGUF) — compact and fast
- [Qwen3-4B-Q4_K_M.gguf](https://huggingface.co/Qwen/Qwen3-4B-GGUF) — balance of quality and speed
- Any other GGUF models (LLaMA, Mistral, Phi, etc.)

## Usage

### Interactive Mode

```bash
chat-np.exe
```

The program will automatically find all `.gguf` files in the current directory and offer to:
1. Select a model (if there are multiple)
2. Select a preset or work in free chat mode
3. Start dialogue

**Menu navigation:**
- `↑/↓` — move through list
- `Enter` — select
- `Esc` — exit

### Server Mode

```bash
chat-np.exe --server
```

Server will start on `http://127.0.0.1:3000`

#### API Endpoints

**GET /models** — list of available models
```bash
curl http://127.0.0.1:3000/models
```

**GET /presets** — list of available presets
```bash
curl http://127.0.0.1:3000/presets
```

**POST /chat** — send request to model
```bash
curl -X POST http://127.0.0.1:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "iPhone 15", "preset": "price_classifier"}'
```

Request parameters:
- `prompt` (required) — request text
- `preset` (optional) — preset name
- `model` (optional) — model name
- `system_prompt` (optional) — system prompt (if not using preset)
- `max_tokens` (optional) — maximum tokens in response

## Preset Configuration

Presets are stored in `presets.json` file. You can edit existing ones or add new ones.

### Preset Example

```json
{
  "name": "price_classifier",
  "description": "Product price classifier",
  "system_prompt": "You are a product classifier for auctions.",
  "instruction": "Determine if the product is CHEAP or EXPENSIVE.",
  "examples": [
    {"input": "bread", "output": "CHEAP - mass market product"},
    {"input": "iPhone 15", "output": "EXPENSIVE - expensive electronics"}
  ],
  "response_format": "Format: CATEGORY - brief explanation",
  "max_tokens": 50,
  "stop_on_newline": true,
  "include_current_date": false
}
```

### Preset Parameters

- `name` — unique preset name
- `description` — user description
- `system_prompt` — main model role
- `instruction` — detailed instructions (optional)
- `examples` — array of examples for few-shot learning (optional)
- `negative_prompt` — what NOT to do (optional)
- `response_format` — response format (optional)
- `max_tokens` — maximum tokens in response
- `stop_on_newline` — stop generation on newline
- `include_current_date` — add current date to prompt (for date-related tasks)

## API Testing

Use `test-api.bat` for quick testing of all presets:

```bash
test-api.bat
```

## Technologies

- **llama-cpp-2** — GGUF model support
- **axum** — web framework for REST API
- **crossterm** — interactive console menu
- **chrono** — date and time handling
- **serde/serde_json** — JSON serialization/deserialization

## License

MIT
