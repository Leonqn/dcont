FROM rust as builder
WORKDIR /usr/src/dcont
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/dcont /usr/local/bin/dcont
ENTRYPOINT ["dcont", "/etc/dcont/config.yml"]
