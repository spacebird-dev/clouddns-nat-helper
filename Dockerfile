ARG ARCH=amd64
ARG TARGET=x86_64-unknown-linux-gnu

FROM ${ARCH}/debian:bullseye-slim
COPY target/${TARGET}/release/clouddns-nat-helper /app/

# run unprivileged
USER 1001

CMD ["/app/clouddns-nat-helper"]
