build:
    cargo build --release

install: build
    install -D -m 755 target/release/flatplay {{executable_directory()}}/flatplay
