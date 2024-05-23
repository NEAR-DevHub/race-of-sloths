FROM rust:1-bookworm as builder

WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo,from=rust:latest,source=/usr/local/cargo \
    --mount=type=cache,target=target \
    cargo build --release -p race-of-sloths-bot && mv ./target/release/race-of-sloths-bot ./race-of-sloths-bot

FROM debian:bookworm-slim

RUN apt update -y && apt install -y libssl-dev ca-certificates 

RUN useradd -ms /bin/bash app
USER app
WORKDIR /app

COPY --from=builder /usr/src/app/race-of-sloths-bot /app/race-of-sloths-bot
COPY ./Messages.toml /app/Messages.toml

CMD ./race-of-sloths-bot
