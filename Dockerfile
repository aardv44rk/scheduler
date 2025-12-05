#   planner
#   using Alpine to ensure link against MUSL libc
FROM rust:alpine AS planner
WORKDIR /app

RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#   cacher
FROM rust:alpine AS cacher
WORKDIR /app
RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

#   builder
FROM rust:alpine AS builder
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY . .

COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

RUN cargo build --release --target x86_64-unknown-linux-musl

#   runtime
FROM alpine:latest
WORKDIR /app

#   create non root user
RUN addgroup -S appgroup && adduser -S appuser -G appgroup

#   add dependencies (root certs for https) and sqlite
RUN apk add --no-cache ca-certificates sqlite-libs

#   setup directory permissions
RUN mkdir -p /app/data && chown -R appuser:appgroup /app/data

#   copy artifacts
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/task-scheduler /app/task-scheduler
COPY --from=builder /app/migrations /app/migrations

#   switch to non root user
USER appuser

#   set environment variables
ENV DATABASE_URL=sqlite:///app/data/tasks.db
ENV SERVER_PORT=8080
ENV APP_ENV=production

EXPOSE 8080

CMD ["./task-scheduler"]
