# Claude Code

Estado: estable · Estrategias: `OAuth → CLI → Web` (hoy: OAuth)

## Fuentes

`Auto` usa OAuth. Lee `%USERPROFILE%\.claude\.credentials.json`
(`claudeAiOauth.accessToken`) y consulta
`GET https://api.anthropic.com/api/oauth/usage`
(cabecera `anthropic-beta: oauth-2025-04-20`).

## Obtiene

- Sesión (5 h) y semanal (7 días): `% usado` + `resets_at`.
- Desgloses Sonnet / Opus / modelos semanales cuando la API los devuelve.
- Uso extra (`extra_usage`) como porcentaje y `$`.
- Uso por producto (Claude Code / Chats / Cowork / Otro).
- Multiplicador temporal de límite cuando existe.
- Coste **estimado** desde logs locales (ver Privacidad).

## Autenticación

Ejecuta `claude` e inicia sesión. Sin token → `NeedsAuth`.

## Dashboard y estado

- Uso: https://claude.ai/settings/usage
- Facturación: https://claude.ai/settings/billing
- Estado: https://status.anthropic.com (fila separada de la conexión)

## Limitaciones

- Sin `five_hour` en la respuesta no hay ventana de sesión (se muestra la semanal).
- El coste **no es factura**: es estimación desde tokens × pricing local.
- `429` conserva el último dato válido como stale y aplica backoff de 5 min.

## Troubleshooting

- `NeedsAuth` tras reinstalar Claude → vuelve a hacer login con `claude`.
- `ParseFailed` → exporta diagnóstico desde Acciones (no incluye secretos).

## Privacidad

El token OAuth nunca sale del equipo salvo hacia `api.anthropic.com`.
El diagnóstico copiable solo lleva porcentajes, plan y resets.
