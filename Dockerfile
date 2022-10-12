ARG arch=amd64

FROM ${arch}/debian:bullseye-slim

ARG target=x86_64-unknown-linux-gnu
ARG profile=debug

ENV TARGET=${target}
ENV PROFILE=${profile}

COPY target/${target}/${PROFILE}/clouddns-nat-helper /bin

# run unprivileged
USER 1001

CMD ["clouddns-nat-helper"]
