# Stage 1: Build
FROM rust:1.60.0 as builder

WORKDIR /build/

COPY . .

RUN cargo build --release

# Stage 2: Run
FROM ubuntu:20.04 as run

# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /root/

COPY --from=builder /build/target/release/fuel-faucet .
COPY --from=builder /build/target/release/fuel-faucet.d .

EXPOSE ${PORT}

# https://stackoverflow.com/a/44671685
# https://stackoverflow.com/a/40454758
# hadolint ignore=DL3025
CMD exec ./fuel-faucet
