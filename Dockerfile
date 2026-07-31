FROM rust:1.87-slim AS builder

# curl: necessário pro build script do utoipa-swagger-ui baixar os assets do Swagger UI
RUN apt-get update && apt-get install -y curl ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copia manifestos primeiro pra aproveitar cache de dependências
COPY Cargo.toml Cargo.lock* ./

# Copia o código fonte
COPY src ./src

# Compila em modo release (otimizado)
RUN cargo build --release

# ---- Stage 2: runtime ----
FROM debian:bookworm-slim

# Certificados TLS, necessários pro reqwest fazer HTTPS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copia só o binário compilado do stage anterior
COPY --from=builder /app/target/release/rust-proxy .

# Render injeta a variável PORT — o main.rs já lê ela
EXPOSE 3000

CMD ["./rust-proxy"]
