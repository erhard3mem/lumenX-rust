# LumenX — native Desktop-GUI (Rust + egui)

Komplette Neuimplementierung von LUMENX als Desktop-Anwendung.
Kein Server, kein Browser: Spiellogik und Oberfläche laufen in einem Binary.

```
src/game.rs   Spielregeln (UI-unabhängig, mit Tests)
src/main.rs   GUI mit eframe/egui
packaging/    .desktop-Datei, Icon, AppRun für das AppImage
```

## Starten (Entwicklung)

```bash
cargo run --release
cargo test           # Regeltests
```

Systemvoraussetzungen zum Bauen unter Linux (Debian/Ubuntu):

```bash
sudo apt install build-essential libxkbcommon-dev libgl1-mesa-dev \
                 libx11-dev libxcursor-dev libxrandr-dev libxi-dev libwayland-dev
```

(Mit Rust allein kompiliert es meistens schon, weil eframe die Grafik-
bibliotheken zur Laufzeit lädt. Die Pakete oben schaden aber nicht.)

## AppImage bauen

```bash
./build-appimage.sh
# → LumenX-x86_64.AppImage
```

Das Skript:

1. baut mit `cargo build --release`
2. legt `build/LumenX.AppDir/` an:
   ```
   LumenX.AppDir/
   ├── AppRun              # startet usr/bin/lumenx
   ├── lumenx.desktop      # Name, Icon, Kategorie (Terminal=false)
   ├── lumenx.svg          # Icon (+ .DirIcon)
   └── usr/bin/lumenx
   ```
3. lädt einmalig `appimagetool` nach `build/`
4. packt das AppDir zu `LumenX-<arch>.AppImage`

FUSE wird zum Bauen nicht benötigt (`APPIMAGE_EXTRACT_AND_RUN=1`).

### Kompatibilität mit älteren Distributionen

Ein AppImage läuft nur auf Systemen mit **gleicher oder neuerer glibc** als
die Build-Maschine. Für maximale Portabilität auf einem alten System bauen,
z. B. Ubuntu 20.04 (glibc 2.31) per Docker:

```bash
docker run --rm -v "$PWD":/src -w /src ubuntu:20.04 bash -c '
  apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    curl build-essential file libxkbcommon-dev libgl1-mesa-dev &&
  curl -sSf https://sh.rustup.rs | sh -s -- -y &&
  . "$HOME/.cargo/env" && ./build-appimage.sh'
```

Ein statischer musl-Build ist bei GUI-Apps keine Alternative: OpenGL- und
Fensterbibliotheken werden zur Laufzeit per `dlopen` geladen, und das klappt
mit statischem musl nicht.

## AppImage ausführen

```bash
chmod +x LumenX-x86_64.AppImage
./LumenX-x86_64.AppImage
```

Fehlt FUSE 2 (z. B. Ubuntu 22.04+ ohne `libfuse2`): `sudo apt install libfuse2`
oder mit `--appimage-extract-and-run` starten.

Laufzeitabhängigkeiten (auf jedem Desktop vorhanden): libGL/libEGL,
libxkbcommon sowie X11- oder Wayland-Bibliotheken.

## Spielregeln (wie im Original)

- 60 Karten: 5 Farben (Gelb, Grün, Blau, Rot, Lila) × Werte 1–12, je 10 Karten.
- Vor jedem Spiel tippt jeder seine Stichanzahl. Computer tippen zufällig 0–5.
- Ausspielen: beliebige Karte. Danach muss bedient werden, sonst freie Wahl.
- Gelb ist Trumpf. Der Stichgewinner spielt als Nächster aus.
- 4 Spiele pro Match; ein Stich zählt 10 / 20 / 30 / 40 Punkte;
  jeder Stich Abweichung vom Tipp kostet 5 Punkte.
- Im Original sieht man die Werte der eigenen Karten nicht, nur die Farben.
  Das ist Standard, lässt sich aber über „Eigene Werte verdeckt“ abschalten.

### Korrekturen gegenüber der JS-Version

- Jedes der 4 Spiele wird neu gegeben, mit neuem Tipp. Im JS waren die Karten
  nach Spiel 1 aufgebraucht, und Stiche wurden nicht zurückgesetzt.
- Bedienpflicht prüft die **ausgespielte** Farbe. Im JS wurde für den
  menschlichen Spieler die Farbe der aktuell führenden Karte geprüft, was nach
  einem Trumpf falsch war.
- Spielreihenfolge im Uhrzeigersinn ab dem Ausspielenden
  (JS: aufsteigende Spielernummer).
