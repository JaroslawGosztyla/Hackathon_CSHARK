---
name: sync-contract
description: Zmień kontrakt IPC między UI a Rustem (DTO, komendy, eventy) spójnie po obu stronach. Używaj zawsze, gdy zmieniasz crates/focus-core/src/api.rs, dodajesz/zmieniasz komendę Tauri albo event.
argument-hint: "[opis zmiany kontraktu]"
---

# /sync-contract

Zmiana: `$ARGUMENTS`

Kontrakt jest wspólny dla osób A i B — każda zmiana musi być kompletna i ogłoszona.

1. **DTO**: zmień typy w `crates/focus-core/src/api.rs` (`#[derive(Serialize, Deserialize, TS)]`, `#[ts(export)]`,
   `#[serde(rename_all = "camelCase")]`; liczby jako `u32`, nie `u64`). Preferuj zmiany kompatybilne wstecz (nowe pole z `#[serde(default)]`).
   Nazwy komend i eventów są stałymi w `api.rs`; nie wpisuj ich ręcznie w TS.
   Sprawdź, czy `SessionSnapshot` nadal pozwala **odtworzyć cały widok** każdego okna (ADR-012) i czy komendy okien przerwy niosą `break_id`.
2. **Bindingi**: `cargo test -p focus-core` → sprawdź diff w `src/lib/bindings/`.
3. **Rust**: zaktualizuj `src-tauri/src/commands.rs` (sygnatury, rejestracja w `generate_handler!` w `lib.rs`)
   i miejsca emitujące eventy. Nowa komenda wymaga też uprawnień w `src-tauri/capabilities/` (jeśli dotyczy).
4. **TS**: zaktualizuj `src/lib/api.ts` (wrapper `invoke`/`listen` z typami z bindings). Nie wołaj `invoke` nigdzie indziej.
5. **Docs**: zaktualizuj tabele w `docs/ARCHITECTURE.md` §6 (komendy / eventy / DTO) i przykład JSON w §6.4.
   Zmiana zachowania faz → także tabela §5.4.
6. **Kompilacja obu stron**: `cargo check --workspace` i `npm run check`.
7. **Ogłoszenie**: dopisz do „Notatki między nami” w `docs/TASKS.md`: `- [data godz] (X→Y) Kontrakt: <co się zmieniło, co druga strona musi zrobić>`.
8. Zaproponuj commit `feat(contract): …` — osobny od reszty zmian, żeby łatwo go było zmergować jako pierwszy.
