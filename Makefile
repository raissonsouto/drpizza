# Variables
BINARY_NAME = drpizza
INSTALL_PATH = /usr/local/bin
CARGO = cargo

.PHONY: all build debug install uninstall clean help

all: build install ## Builda e instala o binário (padrão)

build: ## Builda o binário em modo release
	@echo "Building $(BINARY_NAME) (release)..."
	$(CARGO) build --release

debug: ## Builda o binário em modo debug (sem otimizações e com logs das requisições)
	@echo "Building $(BINARY_NAME) (debug)..."
	$(CARGO) build --features dev

install: ## Instala o binário no sistema
	@echo "Installing to $(INSTALL_PATH)..."
	@if [ ! -f target/release/$(BINARY_NAME) ]; then \
		echo "Binary not found. Building first..."; \
		$(MAKE) build; \
	fi
	sudo cp target/release/$(BINARY_NAME) $(INSTALL_PATH)
	@echo "Installation complete! Run '$(BINARY_NAME)' from anywhere."

uninstall: ## Desinstala o binário do sistema
	@echo "Uninstalling $(BINARY_NAME)..."
	sudo rm -f $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "Uninstalled."

clean: ## Limpa os arquivos de build
	$(CARGO) clean

help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
