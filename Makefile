.PHONY: help version build build-mac build-linux run run-mac doctor-linux config-linux package-linux build-release build-release-mac release release-mac

VERSION := $(shell cat VERSION)
OS := $(shell uname -s)

help:
	@echo "Mousetrap build system"
	@echo ""
	@echo "Current version: $(VERSION)"
	@echo "Detected OS: $(OS)"
	@echo ""
	@echo "Available targets:"
	@echo "  make build-mac            Build macOS app (debug)"
	@echo "  make run-mac              Build and run macOS app"
	@echo "  make build-release-mac    Build macOS release artifact"
	@echo "  make release-mac          Release macOS version from main"
	@echo "  make build-linux          Install Linux package in editable mode"
	@echo "  make doctor-linux         Check Linux runtime dependencies"
	@echo "  make config-linux         Create default Linux config"
	@echo "  make package-linux        Build Linux release bundle"
	@echo "  make version              Show current version"

build-mac:
	bash ./scripts/build-app.sh

run-mac:
	bash ./scripts/run-app.sh

build-release-mac:
	CONFIGURATION=release INSTALL_APP=0 bash ./scripts/build-release.sh

release-mac:
	bash ./scripts/release.sh $(VERSION)

build-linux:
	cd packages/linux && python3 -m pip install --user -e .

doctor-linux:
	cd packages/linux && python3 -m mousetrap_hyprland.cli doctor

config-linux:
	cd packages/linux && python3 -m mousetrap_hyprland.cli init-config

package-linux:
	bash ./scripts/build-linux-bundle.sh

ifeq ($(OS),Darwin)
build: build-mac
run: run-mac
build-release: build-release-mac
release: release-mac
else
build: build-linux
run:
	@echo "Use Hyprland bindings or packages/linux/activate.sh on Linux"
	@exit 1
build-release: package-linux
release:
	@echo "Linux release publishing not implemented yet; use make package-linux"
	@exit 1
endif

version:
	@echo $(VERSION)
