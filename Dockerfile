FROM debian:bullseye-slim

ARG RUST_TARGET=""
ARG PROFILE_DIR

COPY target/${RUST_TARGET}/${PROFILE_DIR}/clouddns-nat-helper /usr/local/bin/
RUN chmod +x /usr/local/bin/clouddns-nat-helper

# run unprivileged
USER 1001

CMD ["clouddns-nat-helper"]
