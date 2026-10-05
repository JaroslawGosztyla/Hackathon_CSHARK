# Spikes

Krótkie, wyrzucalne eksperymenty rozstrzygające ryzyka techniczne (M0). **Nie są częścią workspace'u ani aplikacji.**
Wnioski trafiają do `docs/ARCHITECTURE.md` / `docs/DECISIONS.md`; kod zostaje tylko jako referencja.

| Spike | Task | Wynik |
|---|---|---|
| [audio-spike](audio-spike/) | T0-10 | rodio 0.22.2: brown noise, fade, Stop w trakcie fade, dekodowanie, reconnect po błędzie urządzenia ✅; stabilność czasowa do powtórki na fizycznym sprzęcie (T0-11) |

```powershell
cd spikes\audio-spike
cargo test --release
cargo run --release -- devices
cargo run --release -- scenario         # ~20 s, gra szum i ton
cargo run --release -- soak 2400        # 40 min; w trakcie odłącz słuchawki / wyłącz urządzenie w mmsys.cpl
cargo run --release -- rawcpal 60       # surowy cpal bez rodio: tempo urządzenia, oczekiwane ratio ≈ 1.000
cargo run --release -- diag 2400 60     # metronom (pik co sekundę audio) + pauzy callbacku
cargo run --release --features boost -- diag 2400 60   # to samo z MMCSS i bez power throttlingu
```

**Uwaga:** na maszynie wirtualnej (VirtualBox) zegar audio jest emulowany i pływa, więc `rawcpal`/`diag` nie dają tam sensownych wyników.
