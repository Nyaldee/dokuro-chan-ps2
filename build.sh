#!/usr/bin/env bash
# Compile le portage pour Linux en release et prépare l'archive à partager :
#   target/Dokuro-chan.PS2.Linux.tar.gz
#   (Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen/dokuro_chan_ps2)
# L'archive ne contient que l'exécutable : chaque joueur fournit sa propre
# image disque (voir README.md).
# Il faut Rust (rustup), pkg-config et la bibliothèque du son ALSA :
#   sudo apt install pkg-config libasound2-dev
# Sous Windows, build_linux.bat lance ce script dans WSL.

set -euo pipefail
cd "$(dirname "$0")"

# Lancé par build_linux.bat, bash n'est ni interactif ni un shell de
# connexion : il ne lit pas ~/.profile ni ~/.bashrc, où rustup ajoute
# ~/.cargo/bin au PATH.
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null; then
    echo "Rust est introuvable : https://rustup.rs" >&2
    exit 1
fi
if ! pkg-config --exists alsa 2>/dev/null; then
    echo "La bibliothèque du son manque : sudo apt install pkg-config libasound2-dev" >&2
    exit 1
fi

# Un dossier à part : les fichiers de Windows restent dans target/release.
TARGET=target/linux
cargo build --release --target-dir "$TARGET"

# L'exécutable dans un dossier au nom du jeu, que l'archive garde : on
# l'extrait sans éparpiller de fichiers. (Pas de « : » comme dans le
# titre : Windows les refuse, et l'archive doit s'y extraire aussi.)
tar -czf target/Dokuro-chan.PS2.Linux.tar.gz -C "$TARGET/release" --transform 's,^,Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen/,' dokuro_chan_ps2
echo "target/Dokuro-chan.PS2.Linux.tar.gz est prêt."
