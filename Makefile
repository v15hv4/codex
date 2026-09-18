CODEX_RS := codex-rs
PREFIX ?= $(HOME)/.local
BINDIR := $(PREFIX)/bin

.PHONY: build install release v8

v8:
	@V8_DIR="$$HOME/.cache/codex-v8/150.4.0"; \
		mkdir -p "$$V8_DIR"; \
		gh release download rusty-v8-v150.4.0 \
			--skip-existing \
			--repo openai/codex \
			--dir "$$V8_DIR" \
			--pattern 'librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz' \
			--pattern 'src_binding_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.rs' \
			--pattern 'rusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.sha256'

build: v8
	@export RUSTY_V8_ARCHIVE="$(HOME)/.cache/codex-v8/150.4.0/librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz"; \
		export RUSTY_V8_SRC_BINDING_PATH="$(HOME)/.cache/codex-v8/150.4.0/src_binding_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.rs"; \
		cd $(CODEX_RS) && cargo build --release \
			--bin codex \
			--bin codex-code-mode-host \
			--bin codex-responses-api-proxy

install: build
	@mkdir -p $(BINDIR)
	@install -m 0755 $(CODEX_RS)/target/release/codex $(BINDIR)/codex
	@install -m 0755 $(CODEX_RS)/target/release/codex-code-mode-host $(BINDIR)/codex-code-mode-host
	@install -m 0755 $(CODEX_RS)/target/release/codex-responses-api-proxy $(BINDIR)/codex-responses-api-proxy

release:
	@test -z "$$(git status --porcelain)" || (echo "Working tree must be clean" >&2; exit 2)
	@latest="$$(git tag --list 'rust-v[0-9]*.[0-9]*.[0-9]*' --sort=-version:refname | head -1)"; \
		test -n "$$latest" || (echo "No release tag found" >&2; exit 2); \
		base="$${latest#rust-v}"; patch="$${base##*.}"; prefix="$${base%.*}"; \
		version="$$prefix.$$((patch + 1))"; tag="rust-v$$version"; \
		test -z "$$(git tag --list "$$tag")" || (echo "Tag $$tag already exists" >&2; exit 2); \
		sed -i 's/^version = ".*"/version = "'"$$version"'"/' $(CODEX_RS)/Cargo.toml; \
		cd $(CODEX_RS) && cargo metadata --format-version 1 --no-deps >/dev/null; cd ..; \
		git add $(CODEX_RS)/Cargo.toml $(CODEX_RS)/Cargo.lock; \
		git commit -m "Release $$version"; \
		git push origin HEAD:main; \
		git tag -a "$$tag" -m "Release $$version"; \
		git push origin "$$tag"
