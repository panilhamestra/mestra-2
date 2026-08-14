# rust-proxy

Proxy pro ConstruCode: loga sozinho (e reloga quando expira), decodifica as respostas turbo-stream/HTML/JSON internas do ConstruCode, e serve empreendimentos/disciplinas/itens em JSON simples pra quem consumir.

Fluxo: **chegada** (busca no ConstruCode) → **operação** (decodifica turbo-stream / extrai HTML) → **partida** (serve no endpoint).

## Estrutura dos arquivos

| Arquivo | Objetivo |
|---|---|
| `src/main.rs` | Ponto de entrada. Sobe o servidor, lê a porta (via `state.port`), inicializa logs e monta o router. Não tem lógica de negócio. |
| `src/models.rs` | Structs de dados: `Enterprise`, `Discipline`, `DisciplineItem` (formatos servidos), `SessionToken`/`StoredToken` (token de sessão do ConstruCode), `LoginCredentials`. Anotados com `ToSchema` pro Swagger. Campos numéricos/bool de `Enterprise` e `DisciplineItem` que o ConstruCode às vezes manda `null` (ou `-1`) usam `null_as_default` e tipos com sinal (`i64`) pra não quebrar o parse — `Discipline` não sofre disso porque vem de scraping de HTML, não de JSON. |
| `src/state.rs` | `AppState` — estado compartilhado entre as rotas: cliente HTTP reutilizável, `cookie_jar` (mantém a sessão com o ConstruCode entre requests — não tem relação com sessão do cliente da nossa API), URLs do ConstruCode, credenciais de login, `api_token` (nosso token de acesso) e porta. Tudo lido de variáveis de ambiente em `AppState::from_env()`; se alguma variável obrigatória não estiver setada, o servidor não sobe. |
| `src/service.rs` | Lógica de negócio: `ensure_valid_token` (usa o token salvo em `data/token.json` se ainda válido, senão loga de novo no ConstruCode e persiste), `fetch_enterprises`, `fetch_disciplinas`, `fetch_itens_disciplina`. |
| `src/utils.rs` | Utilitários. Contém o decoder do formato **turbo-stream** (Remix single-fetch) usado pelo endpoint de empreendimentos do ConstruCode — array "achatado" com referências por índice, não é JSON normal. |
| `src/routes.rs` | Camada HTTP: 3 rotas de dados protegidas por header `X-Api-Token` (`/empreendimentos`, `/empreendimentos/:id/disciplinas`, `/empreendimentos/:id/disciplinas/:sigla/itens`) + `/health` livre. Define também o `ApiDoc` (OpenAPI) com o esquema de segurança usado pelo Swagger. |
| `Cargo.toml` / `Cargo.lock` | Dependências do projeto e suas versões travadas (build reprodutível). |
| `Dockerfile` | Build multi-stage: compila o binário numa imagem com toolchain Rust (`rust:1.88-slim`), copia só o binário final pra uma imagem `debian-slim` enxuta. Usado tanto local (`docker compose`) quanto em produção. |
| `docker-compose.yml` | Sobe o serviço localmente, lendo as variáveis de ambiente do `.env`. |
| `.env` | Variáveis reais usadas localmente (URLs do ConstruCode, credenciais, `API_ACCESS_TOKEN`) — **não versionado** (`.gitignore`). Preencha a partir de `.env.example` (se existir) ou peça os valores pra quem já tem. |
| `data/token.json` | Token de sessão do ConstruCode persistido em disco (token + data de expiração), gerado automaticamente pelo primeiro request que precisar dele. Não versionado — só `data/.gitkeep` fica no git pra manter a pasta. |

## Como rodar

### Via Docker (recomendado)

Precisa de Docker Desktop instalado e do `.env` preenchido na raiz do projeto.

Subir:
```powershell
docker compose up --build
```

Derrubar:
```powershell
docker compose down
```

## Autenticação

Toda rota de dados exige o header `X-Api-Token` com o valor de `API_ACCESS_TOKEN` do `.env`. Sem o header (ou com valor errado), a resposta é `401`.

```powershell
curl http://localhost:3000/empreendimentos -H "X-Api-Token: SEU_TOKEN_AQUI"
```

Login no ConstruCode é automático: o primeiro request que precisar de sessão loga sozinho e persiste o token; requests seguintes reaproveitam até expirar.

## Endpoints

| Método | Rota | Descrição |
|---|---|---|
| `GET` | `/empreendimentos` | Lista os empreendimentos do usuário logado. |
| `GET` | `/empreendimentos/{id}/disciplinas` | Lista as disciplinas de um empreendimento. |
| `GET` | `/empreendimentos/{id}/disciplinas/{sigla}/itens` | Lista os itens (documentos/plantas) de uma disciplina, pela sigla (ex: `EST`). |
| `GET` | `/health` | Healthcheck, sem autenticação. |

```powershell
curl http://localhost:3000/empreendimentos -H "X-Api-Token: SEU_TOKEN_AQUI"
curl http://localhost:3000/empreendimentos/5963/disciplinas -H "X-Api-Token: SEU_TOKEN_AQUI"
curl http://localhost:3000/empreendimentos/5963/disciplinas/EST/itens -H "X-Api-Token: SEU_TOKEN_AQUI"
curl http://localhost:3000/health
```

Documentação interativa (Swagger UI) — abre no navegador:
```
http://localhost:3000/
```
Clica em **Authorize** (canto superior direito) e cola o `API_ACCESS_TOKEN` pra poder testar as rotas protegidas por ali.
