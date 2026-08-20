.PHONY: help version build build-mac build-linux test-linux doctor-linux run run-mac install-linux build-release build-release-mac release release-mac

VERSION := $(shell cat VERSION)
OS := $(shell uname -s)

help:
	@echo "Mousetrap build system"
	@echo ""
	@echo "Current version: $(VERSION)"
	@echo "Detected OS: $(OS)"
	@echo ""
	@echo "Available targets:"
	@echo "  make build-mac                  Build macOS app (debug)"
	@echo "  make run-mac                    Build and run macOS app"
	@echo "  make build-release-mac          Build macOS release artifact"
	@echo "  make release-mac                Release macOS version from main"
	@echo "  make build-linux                Build Linux binary (release)"
	@echo "  make test-linux                 Run Linux tests"
	@echo "  make doctor-linux               Check Linux runtime environment"
	@echo "  make install-linux              Install binary to ~/.local/bin"
	@echo "  make version                    Show current version"

build-mac:
	bash ./scripts/build-app.sh

run-mac:
	bash ./scripts/run-app.sh

build-release-mac:
	CONFIGURATION=release INSTALL_APP=0 bash ./scripts/build-release.sh

release-mac:
	bash ./scripts/release.sh $(VERSION)

build-linux:
	cargo build --release --manifest-path packages/linux/Cargo.toml

test-linux:
	cargo test --manifest-path packages/linux/Cargo.toml

doctor-linux:
	cargo run --manifest-path packages/linux/Cargo.toml -- doctor

install-linux: build-linux
	install -m755 packages/linux/target/release/mousetrap "$$HOME/.local/bin/mousetrap"

ifeq ($(OS),Darwin)
build: build-mac
run: run-mac
build-release: build-release-mac
release: release-mac
else
build: build-linux
run:
	@echo "Run the daemon with: mousetrap daemon (see README)"
	@exit 1
build-release: build-linux
release:
	@echo "Linux release publishing not implemented yet"
	@exit 1
endif

version:
	@echo $(VERSION)
