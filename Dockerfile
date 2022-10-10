# build
FROM rust:slim-bullseye as build
WORKDIR /app

COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY ./src src
RUN cargo build --release

# runtime
FROM debian:bullseye-slim
WORKDIR /app

COPY --from=build /app/target/release/docker-swarm-deploy .

EXPOSE 3000

ENTRYPOINT ["./docker-swarm-deploy"]
