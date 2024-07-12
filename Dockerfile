FROM rust:1.78.0-bookworm as builder

WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release -p race-of-sloths-bot -p race-of-sloths-server && mv ./target/release/race-of-sloths-* ./

FROM debian:bookworm-slim

RUN apt update -y && apt install -y libssl-dev ca-certificates 

RUN useradd -ms /bin/bash app
USER app
WORKDIR /app

COPY --from=builder /usr/src/app/race-of-sloths-bot /app/race-of-sloths-bot
COPY --from=builder /usr/src/app/race-of-sloths-server /app/race-of-sloths-server
COPY ./Messages.toml /app/Messages.toml
COPY ./Messages.staging.toml /app/Messages.staging.toml
COPY ./Rocket.toml /app/Rocket.toml
COPY ./public /app/public
