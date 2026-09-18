CODEX_RS := codex-rs
PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin

PROFILE ?= release
PROFILE_DIR := $(if $(filter dev,$(PROFILE)),debug,$(PROFILE))

.PHONY: build install

build:
	@cargo build --locked --manifest-path $(CODEX_RS)/Cargo.toml --profile $(PROFILE) --package codex-cli --bin codex

install: build
	@mkdir -p $(BINDIR)
	@install -m 0755 $(CODEX_RS)/target/$(PROFILE_DIR)/codex $(BINDIR)/codex
	@echo installed $(BINDIR)/codex
