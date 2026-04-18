.PHONY: help build build-mac run run-mac build-release build-release-mac release

VERSION := $(shell cat VERSION)
OS := $(shell uname -s)

help:
	@echo "Mousetrap build system"
	@echo ""
	@echo "Current version: $(VERSION)"
	@echo "Detected OS: $(OS)"
	@echo ""
	@echo "Available targets:"
	@echo "  make build-mac        Build macOS app (debug)"
	@echo "  make run-mac          Build and run macOS app"
	@echo "  make build-release-mac Build macOS release (signed + notarized)"
	@echo "  make release-mac VERSION=x.y.z  Release macOS version"
	@echo "  make build-linux      Build Linux package (install)"
	@echo "  make version          Show current version"

# macOS targets
build-mac:
	bash ./scripts/build-app.sh

run-mac:
	bash ./scripts/run-app.sh

build-release-mac:
	CONFIGURATION=release INSTALL_APP=0 bash ./scripts/build-release.sh

release-mac:
	bash ./scripts/release.sh $(VERSION)

# Linux targets
build-linux:
	@echo "Installing Linux/Hyprland package..."
	cd packages/linux && python3 -m pip install --user -e .

# Convenience aliases (OS-specific defaults)
ifeq ($(OS),Darwin)
build: build-mac
run: run-mac
build-release: build-release-mac
release: release-mac
else
build: build-linux
run:
	@echo "Run target not yet implemented for Linux"
	@exit 1
build-release:
	@echo "Release build not yet implemented for Linux"
	@exit 1
release:
	@echo "Release not yet implemented for Linux"
	@exit 1
endif

version:
	@echo $(VERSION)
