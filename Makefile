# Variables
BINARY_NAME = drpizza
INSTALL_PATH = /usr/local/bin
CARGO = cargo

all: build install

build:
	@echo "🚧 Building $(BINARY_NAME) (Release)..."
	$(CARGO) build --release

debug:
	@echo "🐛 Building $(BINARY_NAME) (Debug)..."
	$(CARGO) build

install:
	@echo "📦 Installing to $(INSTALL_PATH)..."
	
	@if [ ! -f target/release/$(BINARY_NAME) ]; then \
		echo "Binary not found. Building first..."; \
		$(MAKE) build; \
	fi

	sudo cp target/release/$(BINARY_NAME) $(INSTALL_PATH)
	@echo "✅ Installation complete! You can now run '$(BINARY_NAME)' from anywhere."

uninstall:
	@echo "🗑️  Uninstalling $(BINARY_NAME)..."
	sudo rm -f $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "✅ Uninstalled successfully."

clean:
	$(CARGO) clean

.PHONY: all build debug install uninstall clean