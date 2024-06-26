# Stage 1: Build
FROM lukemathwalker/cargo-chef:latest-rust-1.76 as chef
WORKDIR /build/
# hadolint ignore=DL3008
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    lld \
    clang \
    libclang-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

FROM chef as planner
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
COPY --from=planner /build/recipe.json recipe.json
# Build our project dependecies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.
COPY . .
RUN cargo build --release

# Stage 2: Run
FROM ubuntu:22.04 as run

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /root/

COPY --from=builder /build/target/release/fuel-faucet .
COPY --from=builder /build/target/release/fuel-faucet.d .

EXPOSE 3000

# https://stackoverflow.com/a/44671685
# https://stackoverflow.com/a/40454758
# hadolint ignore=DL3025
CMD ["./fuel-faucet"]
