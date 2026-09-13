# IA Usage brand assets

The files in `source/` are the only editable production masters. Files under `generated/` and platform-specific destinations are produced by `npm run brand:generate` and must not be edited by hand.

The official **IA Usage** symbol uses the `I` as the first quota bar, while the rising bars are contained by the `A`. It is flat, high-contrast and intentionally reduced for 16 px use.

## Palette

| Token | Value | Role |
|---|---|---|
| Background | `#18181B` | app icon and dark canvas |
| Surface | `#202023` | cards and secondary surfaces |
| Primary | `#FF7043` | identity accent and quota emphasis |
| Primary hover | `#FF835F` | interactive state only |
| Text | `#F5F5F5` | primary symbol and type on dark |
| Muted | `#71717A` | supporting copy |
| Data / Success | `#34D399` | small functional state only |

Blue, violet, gradients, glow and glass effects are explicitly outside the identity.

## Official variants

- `ia-usage-app-icon.svg`: dark container, complete symbol; Windows, Android, VS Code and touch icons.
- `ia-usage-mark.svg`: full logomark for light backgrounds.
- `ia-usage-small-mark.svg`: reduced dark-surface mark for 16–32 px UI use.
- `ia-usage-mono.svg`: one ink; tray, status bar and notification contexts.
- `ia-usage-wordmark-dark.svg` and `ia-usage-wordmark-light.svg`: horizontal product lockups.

Web favicon derivatives are intentionally different: 48 px uses the app icon, 32 px uses the color small mark on a dark badge, and 16 px uses the mono small mark on a dark badge. Do not generate all three by shrinking the 256 px icon.

Android adaptive icons keep background, foreground and monochrome layers separate. The foreground layers use an 18% inset inside the 108 dp canvas; they must not contain a second rounded-square background.

The Windows tray is functional: no data shows the mono mark; live data shows the rounded percentage with a severity ring.

The tagline is not part of the product identity and must never appear inside an app icon, launcher, favicon, tray icon, notification glyph or VSIX icon.

Provider logos and functional UI icons are separate systems and must never be replaced by an IA Usage brand asset.

## Review sheets

- `generated/brand-directions.png`: three explored directions and final recommendation.
- `generated/brand-applications.png`: selected direction on every product surface.
- `generated/brand-legibility-sheet.png`: actual-size 16–256 px checks.
