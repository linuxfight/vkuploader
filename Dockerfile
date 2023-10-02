FROM joseluisq/rust-linux-darwin-builder:latest
ENV ARCH=aarch64-apple-darwin
RUN rustup target add $ARCH

COPY src/main.rs /root/src/main.rs
COPY Cargo.toml /root/Cargo.toml

CMD cargo build --release --target $ARCH --target-dir /app