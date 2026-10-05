---
name: next-task
description: Weź następny task z docs/TASKS.md dla osoby A lub B, zrealizuj go na własnej gałęzi, sprawdź i oznacz jako zrobiony.
argument-hint: "[A|B] [opcjonalnie ID taska, np. A-03]"
disable-model-invocation: true
---

# /next-task

Argumenty: `$ARGUMENTS` (osoba `A` lub `B`, opcjonalnie ID taska).

1. Ustal osobę: z argumentów, a jeśli ich brak — z `CLAUDE.local.md`. Jeśli dalej nie wiadomo, zapytaj.
2. Przeczytaj `docs/TASKS.md`. Wybierz task:
   - podany ID, albo
   - pierwszy `[ ]` tej osoby w **najwcześniejszym niezamkniętym milestonie**, którego zależności („Zależy od”) są `[x]`.
   Nie wybieraj tasków z M3 (stretch), jeśli bramka M2 nie jest zielona — chyba że użytkownik tak powie.
3. **Gałąź** (sekcja „Git i gałęzie” w `CLAUDE.md`):
   - `git status` — brudne drzewo robocze → zapytaj, co z tymi zmianami (nie stashuj ani nie kasuj na własną rękę),
   - `git switch main && git pull --ff-only`,
   - `git switch -c <a|b>/<ID>-<krótki-slug>` (np. `a/T0-3-workspace`). Jeśli gałąź taska już istnieje — przełącz się na nią.
4. Pokaż użytkownikowi: ID, treść, DoD i krótki plan (3–6 kroków, pliki, które zmienisz). Poczekaj na OK,
   jeśli task dotyka kontraktu IPC albo plików drugiej osoby; w przeciwnym razie działaj.
5. Oznacz task `[~] (X)` w TASKS.md.
6. Zaimplementuj, trzymając się zasad z `CLAUDE.md` i `docs/ARCHITECTURE.md`. Zmiana kontraktu → `/sync-contract`
   (osobny commit, docelowo osobny PR mergowany pierwszy).
7. Uruchom `/check`. Napraw, co się da; czego się nie da — opisz.
8. Oznacz `[x]` (albo zostaw `[~]` z notką, co zostało). Jeśli coś jest potrzebne od drugiej osoby —
   dopisz w „Notatki między nami” z datą i godziną.
9. Zaproponuj wiadomość commita (Conventional Commits) i tytuł/opis PR-a. **Nie commituj, nie pushuj i nie otwieraj PR-a
   bez prośby.** Na prośbę: commit → `git push -u origin <gałąź>` → `gh pr create` (gdy brak `gh`, podaj link
   `https://github.com/JaroslawGosztyla/Hackathon_CSHARK/compare/main...<gałąź>`).
