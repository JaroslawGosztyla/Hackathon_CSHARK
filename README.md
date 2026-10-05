# FocusMode 🎧

Lofi radio i trener przerw w jednym: wybierasz klimat dźwięku, masz 40 minut skupienia, a potem aplikacja
wygania cię od ekranu na 5 minut łagodnego ruchu. Wieczorem może też ocieplić barwy ekranu.

Rust + Tauri 2 · Windows (pełne wsparcie), macOS / Linux (best-effort) · projekt hackathonowy.

## Start

Wymagania: Rust stable (`rustup`), Node.js LTS, zależności systemowe Tauri — lista w
[docs/ARCHITECTURE.md §14](docs/ARCHITECTURE.md#14-narzędzia-i-środowisko).
Na Windows wszystko instaluje skrypt (sprawdza, co już jest, i doinstalowuje resztę):

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-windows.ps1
```

Potem (po scaffoldzie aplikacji w T0-2):

```bash
npm install
npm run tauri dev
# tryb demo (skrócone czasy): FOCUSMODE_DEMO=1 npm run tauri dev
```

## Dokumentacja

| Plik | Zawartość |
|---|---|
| [docs/DESIGN.md](docs/DESIGN.md) | produkt, funkcje, UX, scenariusz demo, ryzyka |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | stos, crate'y, maszyna stanów, kontrakt IPC, wątki, narzędzia |
| [docs/TASKS.md](docs/TASKS.md) | milestone'y i podział pracy A/B |
| [docs/DECISIONS.md](docs/DECISIONS.md) | decyzje architektoniczne (ADR) |
| [docs/CROSS_AI_REVIEW.md](docs/CROSS_AI_REVIEW.md) | jak zrobić review planu drugim modelem AI |
| [CLAUDE.md](CLAUDE.md) | instrukcje dla Claude Code |

## Praca z Claude Code

- `CLAUDE.md` ładuje się automatycznie i zawiera zasady projektu.
- Stwórz lokalny `CLAUDE.local.md` (nie trafia do gita) z jedną linią: `Pracuję jako osoba A` albo `… B`.
- Skille: `/next-task A` (bierze kolejny task, robi go na własnej gałęzi), `/check` (fmt, clippy, testy),
  `/sync-contract <opis>` (spójna zmiana kontraktu UI↔Rust).
- Flow: gałąź na task → PR do `main` → review → squash merge. Szczegóły: [docs/TASKS.md — „Jak pracujemy”](docs/TASKS.md).
