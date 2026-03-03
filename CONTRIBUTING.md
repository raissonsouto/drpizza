# Contribuindo

Guia para contribuir com o `drpizza`.

## Pré-requisitos

- Rust stable
- Git

## Setup local

```bash
git clone https://github.com/raissonsouto/drpizza.git
cd drpizza
cargo build
```

## Fluxo recomendado

1. Crie branch a partir de `main`
2. Implemente a mudança
3. Rode validações locais
4. Abra PR com contexto claro (problema, solução, impacto)

## Validação local

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## Build e execução

```bash
# release
make build

# debug com feature dev (logs de API)
make debug
./target/debug/drpizza --help
```
