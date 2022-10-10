# build
FROM rust:bullseye as build
WORKDIR /app

COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir .cargo
RUN cargo vendor > .cargo/config

COPY ./src src
RUN cargo build --release

# runtime
FROM debian:bullseye
WORKDIR /app

RUN curl -sSL https://get.docker.com/ | sh

COPY --from=build /app/target/release/docker-swarm-deploy .

EXPOSE 3000

ENTRYPOINT ["./docker-swarm-deploy"]
