# Codex / ChatGPT

Estado: estable · Estrategias: `OAuth → CLI` (hoy: OAuth)

## Fuentes

`Auto` usa OAuth. Lee `%USERPROFILE%\.codex\auth.json` (`tokens`:
`access_token`, `refresh_token`, `id_token`) y consulta
`GET https://chatgpt.com/backend-api/wham/usage`.
Si el access token caduca (<5 min de margen), lo renueva contra
`https://auth.openai.com/oauth/token` preservando las claves desconocidas
del `auth.json` en la reescritura.

## Obtiene

- Ventana primaria y secundaria, clasificadas por duración
  (≤6 h → 5 h, ≤36 h → diaria, ≥20 d → mensual, si no semanal).
- Plan derivado del `id_token` (plus/pro/...).
- Reset absoluto y relativo por ventana.

## Autenticación

Ejecuta `codex login`. Sin `auth.json` → `NeedsAuth`; `401` →
`NeedsAuth` (sesión inválida); `403` → `NeedsPermission`.

## Dashboard y estado

- Uso y límites: https://chatgpt.com/#settings
- Estado: https://status.openai.com (fila separada de la conexión)

## Limitaciones

- La renovación concurrente del token (GUI + CLI a la vez) puede
  competir escribiendo `auth.json`; hay un guard de solape en la GUI.
- Sin datos de coste: Codex no expone facturación por esta vía.

## Troubleshooting

- Tras `codex logout`, IA Usage pasa a `NeedsAuth` hasta el próximo login.
- `timeout after 12s` reiterado → revisa red/proxy hacia `chatgpt.com`.

## Privacidad

Los tokens solo viajan hacia `chatgpt.com` / `auth.openai.com`.
El diagnóstico copiable nunca incluye tokens.
