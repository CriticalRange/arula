# Multi-Provider Configuration

## Overview

ARULA CLI now supports multiple AI provider configurations that persist between switches. You can configure multiple providers (OpenAI, Anthropic, Ollama, OpenRouter, Z.AI, and custom providers) and switch between them without losing their individual settings.

## Configuration File Structure

The configuration is stored in `~/.arula/config.yaml` with the following structure:

```yaml
active_provider: "openai"
providers:
  openai:
    model: "gpt-4"
    api_url: "https://api.openai.com/v1"
    api_key: "sk-..."

  anthropic:
    model: "claude-3-opus-20240229"
    api_url: "https://api.anthropic.com"
    api_key: "sk-ant-..."

  ollama:
    model: "llama3"
    api_url: "http://localhost:11434"
    api_key: ""

  openrouter:
    model: "openai/gpt-4o"
    api_url: "https://openrouter.ai/api/v1"
    api_key: "sk-or-..."

  "z.ai coding plan":
    model: "glm-4.6"
    api_url: "https://api.z.ai/api/coding/paas/v4"
    api_key: ""

  custom:
    model: "my-custom-model"
    api_url: "http://localhost:8080"
    api_key: "custom-key"
```

## Features

### 1. Provider Switching
- Switch between providers without losing their configurations
- Each provider maintains its own model, API URL, and API key settings
- Switching is instant - just select a different provider from the menu

### 2. Automatic Defaults
- When you switch to a new provider for the first time, default settings are automatically applied:
  - **OpenAI**: `gpt-3.5-turbo` model
  - **Anthropic**: `claude-3-sonnet-20240229` model
  - **Ollama**: `llama2` model (local)
  - **OpenRouter**: `openai/gpt-4o` model
  - **Z.AI**: `glm-4.6` model
  - **Custom**: Fully configurable

### 3. Custom Providers
You can add custom AI providers with full control over:
- Model name
- API URL
- API key

### 4. Legacy Config Migration
Old single-provider configs are automatically migrated to the new format when loaded. No data is lost during the migration.

## Usage

### Via Menu System

1. Start ARULA and press `m` or type `/menu`
2. Select "Settings" option
3. Navigate to "Provider" and press Enter
4. Use arrow keys to select a provider
5. Press Enter to confirm

The selected provider's model and API URL will be automatically set to defaults if it's the first time using that provider.

### Configuring Each Provider

1. Switch to the provider you want to configure
2. In the Settings menu:
   - **Model**: Select or enter the model name
   - **API URL**: Only editable for custom providers
   - **API Key**: Enter your API key (or leave empty to use environment variables)

### Custom Provider Setup

1. Switch to "custom" provider
2. Set all three fields:
   - Model: Your custom model name
   - API URL: Your API endpoint
   - API Key: Your authentication key

### Environment Variables

You can still use environment variables for API keys:
- `OPENAI_API_KEY` for OpenAI
- `ANTHROPIC_API_KEY` for Anthropic
- `OPENROUTER_API_KEY` for OpenRouter
- `ZAI_API_KEY` for Z.AI
- `CUSTOM_API_KEY` for custom providers

If an API key is not set in the config, ARULA will check these environment variables.

## Example Workflow

```bash
# Start with OpenAI (default)
arula-cli

# Configure OpenAI
# Menu > Settings > Model > gpt-4
# Menu > Settings > API Key > sk-...

# Switch to Anthropic
# Menu > Settings > Provider > anthropic
# Menu > Settings > Model > claude-3-opus
# Menu > Settings > API Key > sk-ant-...

# Switch back to OpenAI
# Menu > Settings > Provider > openai
# Your OpenAI settings (gpt-4, api key) are preserved!

# Add a custom provider
# Menu > Settings > Provider > custom
# Menu > Settings > Model > my-model
# Menu > Settings > API URL > http://localhost:8080
# Menu > Settings > API Key > my-key
```

## Benefits

1. **No Configuration Loss**: Switch providers freely without losing settings
2. **Quick Experimentation**: Test different providers and models easily
3. **Multiple Workflows**: Use different providers for different tasks
4. **Environment Flexibility**: Dev/staging/prod configurations side by side
5. **Backward Compatible**: Old configs automatically migrate

## Technical Details

### Config Structure

- `active_provider`: String - Currently selected provider
- `providers`: HashMap<String, ProviderConfig> - All provider configurations
  - Each provider has:
    - `model`: String - Model name/identifier
    - `api_url`: Option<String> - API endpoint URL
    - `api_key`: String - Authentication key

### Migration

Legacy configs with the old `ai` structure:
```yaml
ai:
  provider: "openai"
  model: "gpt-4"
  api_url: "https://api.openai.com/v1"
  api_key: "sk-..."
```

Are automatically converted to:
```yaml
active_provider: "openai"
providers:
  openai:
    model: "gpt-4"
    api_url: "https://api.openai.com/v1"
    api_key: "sk-..."
```

## API

### Config Methods

- `config.switch_provider(name)` - Switch active provider
- `config.get_model()` - Get current provider's model
- `config.set_model(model)` - Set current provider's model
- `config.get_api_key()` - Get current provider's API key
- `config.set_api_key(key)` - Set current provider's API key
- `config.get_api_url()` - Get current provider's API URL
- `config.set_api_url(url)` - Set current provider's API URL (custom only)
- `config.get_provider_names()` - List all configured providers
- `config.add_custom_provider(name, model, url, key)` - Add custom provider

### Example

```rust
use arula_cli::config::Config;

let mut config = Config::load_or_default()?;

// Switch to Anthropic
config.switch_provider("anthropic")?;
config.set_model("claude-3-opus-20240229");
config.set_api_key("sk-ant-...");
config.save()?;

// Switch to OpenAI
config.switch_provider("openai")?;
// OpenAI settings are preserved from before

// Add custom provider
config.add_custom_provider(
    "my-llm",
    "custom-model-v1",
    "http://localhost:8080/v1",
    "my-secret-key"
)?;
config.save()?;
```
