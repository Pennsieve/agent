# Invoking `make` will compile the Pennsieve agent and generate
# an installable package for the current operating system (including Windows).

.PHONY: all clean compile docs os package
.DEFAULT_GOAL := all

RELEASE_NAME := pennsieve
VERSION := $(shell ci/unix/version.sh)
BUILD_DIR := $(shell mktemp -d)
PLATFORM := $(strip $(shell uname))

# Build the agent.
compile:
	cargo build --release

copy_bin: compile
	cp "target/release/$(RELEASE_NAME)" "$(BUILD_DIR)"

# Build a distributable package.
package: copy_bin
ifeq ($(PLATFORM), Darwin)
	sh scripts/unix/mac_build_local.sh "$(RELEASE_NAME)" "$(VERSION)" "$(BUILD_DIR)"
else ifeq ($(PLATFORM), Linux)
	sh ci/unix/linux_build.sh "$(RELEASE_NAME)" "$(VERSION)" "$(BUILD_DIR)"
else ifeq ($(findstring NT,$(PLATFORM)),NT)
	cargo wix --no-build \
		--bin-path "C:/Program Files (x86)/WiX Toolset v3.11/bin" \
		--license ci/windows/wix/License.rtf \
		--nocapture \
		--product-name "Pennsieve" \
		--binary-name "$(RELEASE_NAME)" \
		ci/windows/wix/main.wxs
else
	@echo "Unsupported platform: $(PLATFORM)"
endif

# Build end-user documentation.
docs:
	$(MAKE) -C "docs" all

# Remove generated end-user documentation and packages.
clean:
ifeq ($(PLATFORM), Darwin)
	rm -f "$(RELEASE_NAME)*.pkg"
else ifeq ($(PLATFORM), Linux)
	rm -f "$(RELEASE_NAME)*.deb"
else ifeq ($(findstring NT,$(PLATFORM)),NT)
	rm -Rf target/wix
endif
	$(MAKE) -C "docs" clean

os:
	@echo "$(PLATFORM)"

all: package
