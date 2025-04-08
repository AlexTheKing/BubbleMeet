FROM rust:1.82 as builder
WORKDIR /usr/src/bubble-meet
COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo install --path .

FROM debian:bookworm-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/bubble-meet /usr/local/bin/bubble-meet

ENV LOG4RS_CONFIG_PATH=/etc/bubble-meet/resources/log4rs.yml
COPY resources /etc/bubble-meet/resources

CMD ["bubble-meet"]