FROM clux/muslrust:stable AS planner
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM clux/muslrust:stable AS cacher
RUN cargo install cargo-chef
COPY --from=planner /volume/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json


FROM clux/muslrust:stable AS builder
COPY . .
COPY --from=cacher /volume/target target
COPY --from=cacher /root/.cargo /root/.cargo
RUN cargo build --bin sleeper --release --target x86_64-unknown-linux-musl


FROM scratch
COPY --from=builder /volume/target/x86_64-unknown-linux-musl/release/sleeper /sleeper
USER 1000
ENTRYPOINT ["/sleeper"]
