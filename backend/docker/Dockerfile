FROM debian:buster-slim

RUN true \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        ed \
        less \
        locales \
        vim-tiny \
        wget \
        curl \
        nano \
        ca-certificates \
        build-essential \
        git \
        dos2unix

COPY ./scripts /webapp/scripts

ARG RUST_VERSION=nightly
RUN sh /webapp/scripts/download-rust.sh

ARG GIT_ACCESS_TOKEN
ENV GIT_TOKEN=$GIT_ACCESS_TOKEN
COPY ./source /webapp/source
COPY ./binary /webapp/binary

EXPOSE 8002

ENTRYPOINT ["/webapp/scripts/init.sh"]
CMD ["/webapp/binary/start.sh"]
