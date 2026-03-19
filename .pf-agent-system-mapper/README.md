# .pf-agent-system-mapper

This directory contains agent-system-mapper prompts and examples.

They are guidance artifacts only and have no runtime effect.

**Safe to delete at any time.**

## What's here

- `prompts/` - Standard AI agent prompts (grep/file-based)
- `prompts/lsp/` - LSP-optimized prompts (~50% fewer tokens)
- `examples/` - Framework-specific good vs bad documentation examples

## Usage

### Standard Prompts (grep-based)
```
Read .pf-agent-system-mapper/prompts/01-architecture-overview.md
and document this codebase following that methodology.
```

### LSP Prompts (recommended if LSP available)
```
Read .pf-agent-system-mapper/prompts/lsp/01-architecture-overview.md
and document this codebase following that methodology.
```

The prompt will auto-detect your framework and use the appropriate examples.

## Prompt Versions

| Type | Token Usage | Best For |
|------|-------------|----------|
| **Standard** (`prompts/`) | 15-26k tokens | No LSP server, dynamic code |
| **LSP** (`prompts/lsp/`) | 7-12k tokens | LSP available, hitting session limits |

## Framework Examples

| Framework | Good Example | Bad Example |
|-----------|--------------|-------------|
| Laravel | `examples/laravel/good-architecture-doc-example.md` | `examples/laravel/bad-architecture-doc-example.md` |
| FastAPI | `examples/fastapi/good-architecture-doc-example.md` | `examples/fastapi/bad-architecture-doc-example.md` |
| Flask | `examples/flask/good-architecture-doc-example.md` | `examples/flask/bad-architecture-doc-example.md` |
| Livewire | `examples/livewire/good-architecture-doc-example.md` | `examples/livewire/bad-architecture-doc-example.md` |
| React | `examples/react/good-architecture-doc-example.md` | `examples/react/bad-architecture-doc-example.md` |
| Vue | `examples/vue/good-architecture-doc-example.md` | `examples/vue/bad-architecture-doc-example.md` |

## Package Examples

| Package | Good Example | Bad Example |
|---------|--------------|-------------|
| Requests | `examples/packages/requests/good-architecture-doc-example.md` | `examples/packages/requests/bad-architecture-doc-example.md` |

## Source

https://github.com/peak-flow/agent-system-mapper
