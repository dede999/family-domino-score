# Imagem do app para produção (Cloud Run ou qualquer host de containers).
#
# Primeira etapa: compila o servidor e a interface com o `dx bundle`.
# Segunda: só o binário `server` e a pasta `public/`, numa imagem pequena.

FROM rust:1.95-trixie AS build
RUN rustup target add wasm32-unknown-unknown
# O dx já compilado, em vez de compilar do zero (minutos a menos). Ele
# pede glibc 2.39 ou mais nova, por isso trixie e não bookworm.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall dioxus-cli@0.7.10 --no-confirm
WORKDIR /app
COPY . .
RUN dx bundle --platform web --release

FROM debian:trixie-slim
# Certificados para a conexão HTTPS com o Turso.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# O servidor procura a interface em `public/`, ao lado do binário.
COPY --from=build /app/target/dx/placar/release/web/ /app/
# O Cloud Run diz a porta em PORT; o Dioxus lê IP e PORT.
ENV IP=0.0.0.0 PORT=8080
EXPOSE 8080
CMD ["/app/server"]
