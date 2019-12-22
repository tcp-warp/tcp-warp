FROM rustembedded/cross:x86_64-unknown-linux-musl as BUILD

COPY . /src
WORKDIR /src

RUN apt-get update && apt-get install curl && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.sh && \
    sh rustup-init.sh -y --target x86_64-unknown-linux-musl

RUN /root/.cargo/bin/cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:3.9

COPY --from=BUILD /src/target/x86_64-unknown-linux-musl/release/tcp-warp /usr/bin/

CMD /usr/bin/tcp-warp server