# rust-proxy

Proxy que busca dados de uma API externa, organiza/calcula em cima deles, e serve em JSON pronto pra consumir.

Fluxo: **chegada** (busca API externa) → **operação** (transforma/calcula) → **partida** (serve no endpoint).

## Estrutura dos arquivos

| Arquivo | Objetivo |
|---|---|
| `src/main.rs` | Ponto de entrada. Sobe o servidor, lê a porta (via `state.port`), inicializa logs e monta o router. Não tem lógica de negócio. |
| `src/models.rs` | Structs de dados: `ApiItemRaw`/`RatingRaw` (formato cru vindo da API externa) e `OrganizedItem` (formato que servimos pra fora, já com o campo calculado `estimated_revenue`, e anotado com `ToSchema` pro Swagger). |
| `src/state.rs` | `AppState` — estado compartilhado entre as rotas (cliente HTTP reutilizável, URL da API de origem, porta). Tudo lido de variáveis de ambiente em `AppState::from_env()`; se `SOURCE_API_URL` ou `PORT` não estiverem setadas, o servidor não sobe. |
| `src/service.rs` | Lógica de negócio, separada em 3 etapas: `fetch_raw_items` (chegada — fetch na API), `calculate_estimated_revenue` (operação — função pura `price * rating_count`), `organize_item` (partida — monta o struct de saída). `fetch_and_organize` orquestra as três. |
| `src/routes.rs` | Camada HTTP: rotas (`/dados`, `/health`, `/`) e os handlers (`get_items_handler`, `health_handler`) que chamam o `service.rs` e devolvem JSON. Define também o `ApiDoc` (OpenAPI) usado pelo Swagger. |
| `Cargo.toml` | Dependências do projeto (equivalente ao `requirements.txt` do Python). |
| `Cargo.lock` | Trava as versões exatas de cada dependência baixada — gerado automático, garante build reprodutível. |
| `Dockerfile` | Build multi-stage: compila o binário numa imagem com toolchain Rust, copia só o binário final pra uma imagem `debian-slim` enxuta. Usado tanto local (`docker compose`) quanto pelo Render. |
| `docker-compose.yml` | Sobe o serviço localmente, lendo as variáveis de ambiente do `.env`. |
| `.env` | Variáveis reais usadas localmente (`SOURCE_API_URL`, `PORT`) — não é versionado (`.gitignore`). |

## API de teste usada

[Fake Store API](https://fakestoreapi.com/products) — pública, sem autenticação, sem rate limit. Retorna produtos fake com `price` e `rating.count`, usados pra calcular `estimated_revenue = price * rating_count`.

Troque a URL em `SOURCE_API_URL` (no `.env`) quando for usar a API real.

## Como rodar

### Via Docker (recomendado)

Precisa de Docker Desktop instalado e do arquivo `.env` na raiz do projeto com:
```
SOURCE_API_URL=https://fakestoreapi.com/products
PORT=3000
```

Subir:
```powershell
docker compose up --build
```

Derrubar:
```powershell
docker compose down
```

## Como usar o endpoint

Com o servidor rodando (Docker ou local), sobe em `http://localhost:3000`.

```powershell
curl http://localhost:3000/dados
```

Resposta (exemplo, um item):

```json
[
  {
    "id": 1,
    "name": "Fjallraven - Foldsack No. 1 Backpack, Fits 15 Laptops",
    "category": "men's clothing",
    "price": 109.95,
    "rating_count": 120,
    "estimated_revenue": 13194.0
  }
]
```

Healthcheck:
```powershell
curl http://localhost:3000/health
```

Documentação interativa (Swagger UI) — abre no navegador:
```
http://localhost:3000/
```
