# Imagem do CD: só a etapa final do `Dockerfile`. O GitHub Actions já
# compilou com `dx bundle` (com cache) e passa a pasta pronta
# (target/dx/placar/release/web) como contexto.
FROM debian:trixie-slim
# Certificados para a conexão HTTPS com o Turso.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# O servidor procura a interface em `public/`, ao lado do binário.
COPY . /app/
# O Cloud Run diz a porta em PORT; o Dioxus lê IP e PORT.
ENV IP=0.0.0.0 PORT=8080
EXPOSE 8080
CMD ["/app/server"]
