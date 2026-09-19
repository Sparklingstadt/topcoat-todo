FROM rust:1-bookworm AS tools
RUN rustup component add rustfmt clippy

FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked && cp target/release/topcoat-todo /app/topcoat-todo

FROM mcr.microsoft.com/playwright:v1.63.0-noble@sha256:eff16c30e6f3f4af0a03fa4b706120d5e9b0891c344a27d64559aff5900a4a27 AS browser-tests
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts
COPY --from=builder /app/topcoat-todo /usr/local/bin/topcoat-todo
COPY playwright.config.ts ./
COPY tests/playwright ./tests/playwright
ENV TODO_TEST_BINARY=/usr/local/bin/topcoat-todo
CMD ["npm", "test"]

FROM debian:bookworm-slim AS runtime
RUN useradd --system --uid 10001 --create-home todo \
    && mkdir -p /app/data && chown todo:todo /app/data
WORKDIR /app
COPY --from=builder /app/topcoat-todo /app/topcoat-todo
USER todo
ENV HOST=0.0.0.0 PORT=3000 TODO_DB_PATH=/app/data/todos.sqlite3
EXPOSE 3000
CMD ["/app/topcoat-todo"]
