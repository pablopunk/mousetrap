.PHONY: build run build-release release

build:
	bash ./scripts/build-app.sh

run:
	bash ./scripts/run-app.sh

build-release:
	CONFIGURATION=release INSTALL_APP=0 bash ./scripts/build-release.sh

release:
	bash ./scripts/release.sh $(VERSION)
