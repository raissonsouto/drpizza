# Contribuindo

Guia para desenvolvedores que querem contribuir com o projeto Dr. Pizza CLI.

## Pre-requisitos

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Git

## Setup

```sh
git clone https://github.com/raissonsouto/drpizza.git
cd drpizza
cargo build
```

## Desenvolvimento

### Rodar em modo debug

```sh
# Sem logging de debug
make run ARGS="menu"
# Equivalente a: cargo run -- menu

# Com logging de debug (mostra requests, payloads, status codes)
make dev ARGS="menu"
# Equivalente a: cargo run --features dev -- menu
```

A flag `--features dev` ativa macros `dev_log!` em `src/api.rs` que imprimem detalhes das chamadas HTTP no stderr.

### Build de release

```sh
make build
# Equivalente a: cargo build --release
```

## Rodando CI localmente

O CI roda formatacao, clippy e testes. Para reproduzir localmente:

```sh
# Tudo de uma vez (mesmo que o CI do GitHub)
make ci

# Individualmente
make fmt        # Formata o codigo
make fmt-check  # Verifica formatacao (sem modificar)
make lint       # Roda clippy
make test       # Roda testes
make check      # Compilacao rapida (sem gerar binario)
```

Para ver todos os targets disponiveis:

```sh
make help
```

## Estrutura do projeto

```
src/
├── main.rs        # CLI (clap): parsing de args e dispatch de comandos
├── models.rs      # Structs de dados (Unit, MenuItem, Order, UserConfig, etc.)
├── api.rs         # Chamadas HTTP (reqwest) para a API do CardapioWeb
├── config.rs      # AppOptions, load/save de perfil e cache
├── ui.rs          # Helpers de terminal (Spinner, input, formatacao)
├── menu.rs        # Comando: exibir cardapio
├── order.rs       # Comando: fluxo interativo de pedido
├── orders.rs      # Comandos: pedido (ultimo) e pedidos (historico)
├── units.rs       # Comando: listar/gerenciar unidades
├── profile.rs     # Comando: ver/editar perfil
└── addresses.rs   # Comando: gerenciar enderecos
build.rs           # Injeta GIT_HASH no binario em compile time
```

### Fluxo de dados

1. `main.rs` faz parse dos argumentos e cria `AppOptions`
2. Cada comando busca unidades via `api::fetch_units()`
3. `ApiContext` e criado a partir da unidade selecionada (company_id, slug, session_id)
4. Dados do usuario sao carregados de `~/.drpizza` (exceto em modo `--stateless`)
5. Cache do cardapio fica em `~/.drpizza_menu_cache.json` (TTL de 30 min)

## Fazendo um PR

1. Crie um branch a partir de `main`
2. Faca suas mudancas
3. Rode `make ci` para garantir que tudo passa
4. Abra um Pull Request com descricao clara do que foi alterado

## Arquivos locais (runtime)

| Arquivo | Descricao |
|---------|-----------|
| `~/.drpizza` | Perfil do usuario (JSON) |
| `~/.drpizza_menu_cache.json` | Cache do cardapio (TTL 30 min) |

Estes arquivos nao existem no repositorio — sao criados em runtime. O modo `--stateless` ignora ambos.
