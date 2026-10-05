---
name: check
description: Uruchom pełny zestaw kontroli projektu FocusMode (fmt, clippy, testy Rust, svelte-check) i zrelacjonuj wynik. Używaj przed oznaczeniem taska jako zrobiony.
---

# /check

Uruchamiaj po kolei i zbierz wyniki (nie przerywaj po pierwszym błędzie):

1. `cargo fmt --all` (formatuje; potem `git diff --stat`, żeby pokazać, co się zmieniło)
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace` (regeneruje też `src/lib/bindings/`)
4. `npm run check`
5. Jeśli `git status` pokazuje zmiany w `src/lib/bindings/`, zaznacz to: kontrakt się zmienił, a drugą osobę trzeba poinformować (`/sync-contract`).

Kroki, których nie da się uruchomić (np. brak workspace'u przed M0), pomiń i napisz to wprost.

Raport: tabela krok → ✅/❌ + dla ❌ najważniejsze linie błędu i proponowana poprawka.
Nie wyciszaj lintów (`#[allow(...)]`) ani nie oznaczaj testów `#[ignore]`, żeby przejść — chyba że użytkownik tak zdecyduje.
