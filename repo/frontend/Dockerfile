# Build the Dioxus web bundle, then ship it behind nginx with an /api proxy
# to the backend service.

FROM rust:1.88-bookworm AS builder
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.118 --locked
WORKDIR /build
COPY . .
WORKDIR /build/frontend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --target wasm32-unknown-unknown -p frontend \
    && wasm-bindgen --no-typescript --target web \
       --out-dir dist/assets/dioxus \
       /build/target/wasm32-unknown-unknown/release/frontend.wasm

FROM nginx:alpine
COPY --from=builder /build/frontend/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
