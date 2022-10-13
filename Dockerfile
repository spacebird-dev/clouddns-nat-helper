FROM rust:1.64 as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo install --path .

FROM debian:bullseye-slim

COPY --from=builder /usr/local/cargo/bin/clouddns-nat-helper /usr/local/bin/clouddns-nat-helper

# run unprivileged
USER 1001

CMD ["clouddns-nat-helper"]
