FROM rust:1-trixie AS builder
WORKDIR /s7cmd
COPY . ./
RUN git config --global --add safe.directory /s7cmd \
&& cargo build --release

FROM debian:trixie-slim
RUN apt-get update \
&& apt-get install --no-install-recommends -y ca-certificates \
&& apt-get clean \
&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /s7cmd/target/release/s7cmd /usr/local/bin/s7cmd

RUN useradd -m -s /bin/bash s7cmd
USER s7cmd
WORKDIR /home/s7cmd/
ENTRYPOINT ["/usr/local/bin/s7cmd"]
