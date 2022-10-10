# build
FROM rust:slim-bullseye as build
WORKDIR /app

COPY . .

RUN cargo build --release

# runtime
FROM debian:bullseye-slim
WORKDIR /app

COPY --from=build /app/target/release/docker-swarm-deploy .

EXPOSE 5123

ENTRYPOINT ["./docker-swarm-deploy"]

# TODO: Cache dependencies in a dummy build