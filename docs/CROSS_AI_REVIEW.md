# Cross-AI review

Pomysł: design pisał Claude, więc drugi model (GPT, Gemini) robi krytyczny przegląd, żeby wyłapać
ślepe plamki. Wyniki wracają do repo jako zmiany w DESIGN/ARCHITECTURE i wpisy w DECISIONS.

## Jak

1. Wklej do drugiego modelu prompt poniżej + pełną treść `docs/DESIGN.md`, `docs/ARCHITECTURE.md`,
   `docs/DECISIONS.md` i `docs/TASKS.md` (albo załącz te 4 pliki). `CLAUDE.md` i `.claude/` nie są potrzebne —
   to instrukcje dla AI, nie plan projektu.
2. Odpowiedź zapisz jako `docs/reviews/<model>-<data>.md` (surowo, bez edycji).
3. W Claude Code: „Przeczytaj `docs/reviews/<plik>` i zaproponuj, które uwagi przyjąć. Dla przyjętych
   zaktualizuj DESIGN/ARCHITECTURE/TASKS i dopisz lub zmień ADR-y w DECISIONS.md.”
4. Decyzję o każdej uwadze podejmujecie wy (przyjmij / odrzuć z powodem).

## Historia

| Data | Plik | Wynik |
|---|---|---|
| 2026-09-27 | [reviews/codex-crosscheck-2026-09-27.md](reviews/codex-crosscheck-2026-09-27.md) | F01–F16 przetworzone → DESIGN/ARCHITECTURE/TASKS v2; decyzje ADR-008…010 przyjęte, ADR-011/012 proponowane |

| 2026-10-06 | [reviews/codex-review-2026-10-06.md](reviews/codex-review-2026-10-06.md) | R01–R09 przyjęte → DESIGN/ARCHITECTURE v3, poprawki do ADR-003/009/010/011/012, DoD tasków A-01…A-06, B-02, B-05, B-12, S-01, S-02 i checklista P-4 |

Kolejny dobry moment na review: po M1 (na działającym kodzie, nie tylko na dokumentach). Recenzent widzi tylko kopię dokumentów
w `Z:\Hackathon_docs` (automatyczna synchronizacja), bez kodu. Do review kodu trzeba mu dać dostęp do repo.

## Prompt

```
Jesteś doświadczonym architektem aplikacji desktopowych w Ruście i product designerem.
Oceniasz plan projektu hackathonowego (2 osoby, 2 tygodnie, wolno używać dowolnych narzędzi AI;
implementację w dużej mierze prowadzi Claude Code) — dokumenty poniżej.

Zrób krytyczny review. Nie chwal; szukaj problemów. Odpowiedz w sekcjach:
1. Top 5 ryzyk, które mogą zabić demo (z prawdopodobieństwem i konkretną mitigacją).
2. Błędy lub nieaktualne założenia techniczne (Tauri 2, rodio, gamma ramp na Win/macOS/Linux,
   Microsoft Graph presence, ts-rs). Wskaż, co sprawdzić w dokumentacji.
3. Co wyciąć, żeby zmieścić się w czasie; co dodać, bo daje dużo „wow” małym kosztem.
4. Alternatywne podejścia, które warto rozważyć (framework, audio, tint), z uczciwym porównaniem.
5. Czy podział pracy A/B jest zbalansowany i czy kontrakt IPC pozwala pracować równolegle?
6. 3 pomysły produktowe, które wyróżnią projekt u jury.

Bądź konkretny: odwołuj się do sekcji dokumentów.

--- DESIGN.md ---
<wklej>

--- ARCHITECTURE.md ---
<wklej>

--- DECISIONS.md ---
<wklej>

--- TASKS.md ---
<wklej>
```
