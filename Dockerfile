# builder
FROM clux/muslrust:nightly-2018-02-26 as builder

WORKDIR /app

COPY ./Cargo.toml ./Cargo.lock ./
RUN set -x && cargo fetch --locked -v

COPY ./src/ ./src/
RUN cargo build --target=x86_64-unknown-linux-musl --release --frozen -v \
    && mv target/x86_64-unknown-linux-musl/release/rs-pgfs-report ./ \
    && rm -rf Cargo.lock Cargo.toml src/ target/

# runtime
FROM alpine:3.7

WORKDIR /app

COPY --from=builder \
    /app/rs-pgfs-report \
    ./

COPY ./config/ ./config/
COPY run.sh ./

ENTRYPOINT [ "./run.sh" ]