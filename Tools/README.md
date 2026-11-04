# Outils d'étude de Dokuro-chan (PS2)

Les outils qui ont servi à étudier *Game ni Natta yo! Dokuro-chan ~Kenkou Shindan Daisakusen~* (SLPM-66186) pour le portage. Tous lisent le disque ou l'exécutable sans rien exécuter.

## Organisation

```
dokuro-chan-ps2\
  <image>.iso          l'ISO originale (sous n'importe quel nom ; ignorée par git)
  src\                 le portage
  decompiled\          le pseudo-C du jeu (ignoré par git)
  Tools\               ces outils
    work\              fichiers de travail (ignoré par git) : exécutable sorti
                       de l'ISO, symboles, projet Ghidra, notes (etude.md),
                       images et sons exportés
```

En debug, le portage cherche aussi l'ISO à la racine du dépôt.

## Tester le portage

`Tester.bat` lance le portage en debug avec les variables de test du README (section « Tester ») choisies dans un menu : zones de la souris, vidéos du démarrage, trace des points d'entrée, histoire à un endroit, captures, appuis simulés. Il lance aussi l'audit de l'histoire (depuis le début ou un point d'entrée) et `cargo test`.

## Décompiler

```
Tools\decompile.bat <dossier de Ghidra> [préfixes]
```

1. `extract.py` sort l'exécutable `SLPM_661.86` de l'ISO dans `work\`.
2. `dwarf1_symbols.py` tire de ses informations de débogage (DWARF 1) le nom et la place de chaque fonction, dans `work\symbols.txt`.
3. Ghidra (en mode sans fenêtre) importe et analyse l'exécutable, le nomme avec `ApplySymbols.java`, puis `ExportMinigame.java` écrit le pseudo-C dans `decompiled`, en plusieurs passes (le lanceur de Ghidra refuse une ligne de commande trop longue) : un fichier par mini-jeu, `ad_mini.c`, `font.c`, `window.c`, puis le reste du jeu : `stcm2.c` (la machine des scripts), `adv.c` (l'enchaînement de l'histoire), `adv_gp.c` (décors, personnages, effets, fondus), `menus.c`, `map.c`, `battle.c`, `tenkey.c`, `save.c`, `mcs.c` (l'écran de la carte mémoire), `sound.c`, `movie.c` ; et `extra.c` pour les préfixes donnés en second argument (séparés par « : »). Le projet Ghidra déjà analysé (`work\ghidra`) est repris s'il existe.

Il faut Python, Ghidra 12 avec l'extension ghidra-emotionengine-reloaded (processeur R5900 de la PS2 ; son archive est gardée dans `work\`) et Java (JAVA_HOME ou celui configuré pour Ghidra).

## Scripts Ghidra

| Script | Rôle |
|---|---|
| `ApplySymbols.java` | nomme les fonctions d'après `symbols.txt` |
| `ExportMinigame.java` | exporte les fonctions d'un préfixe et celles qu'elles appellent, avec les données lues |
| `ExportCallers.java` | exporte les fonctions qui appellent les fonctions nommées |
| `ExportRefs.java` | exporte les fonctions qui lisent ou écrivent des adresses |
| `GrepDecomp.java` | exporte les fonctions dont le pseudo-C contient un motif |
| `ExportFunctions.java` | liste toutes les fonctions (adresse, nom, taille, fonctions appelées, pointée par une donnée), pour `coverage.py` |

## Bilan de couverture

```
analyzeHeadless … -postScript ExportFunctions.java Tools\work\functions.tsv
python -I Tools\coverage.py Tools\work\functions.tsv .
```

`coverage.py` liste les fonctions du jeu atteintes depuis `main` ou une table de fonctions que personne n'a encore décompilées (en-tête « // nom @ adresse » dans `decompiled\` ou `work\`) ni nommées dans le portage (`src\`, `docs\`), groupées par préfixe et triées par taille. Les bibliothèques de Sony (fonctions sans nom, ou entre 0x110000 et 0x150000) sont laissées de côté.
| `DecompileAt.java` | décompile les fonctions à des adresses données |
| `Disasm.java` | affiche les instructions d'une plage d'adresses |

## Étudier les scripts de l'histoire

```
 python -I Tools/uni_extract.py <image>.iso UNION/SCRIPT.UNI Tools/work/script
 python -I Tools/stcm2_table.py Tools/work/SLPM_661.86 Tools/work/symbols.txt > Tools/work/stcm2_opcodes.txt
 python -I Tools/stcm2_dump.py Tools/work/script/101.bin Tools/work/stcm2_opcodes.txt > Tools/work/script/101.txt
```

Le format des scripts et leurs instructions sont décrits dans `docs/scripts.md` ; le moteur qui les joue, dans `docs/moteur.md`.

## Scripts Python

À lancer avec `python -I` (chaque script donne son usage en tête de fichier).

| Script | Rôle |
|---|---|
| `crc32.py` | CRC32 d'un fichier (l'ISO attendue : `B9F5F0C5`) |
| `iso_list.py` | fichiers de l'ISO : secteur, taille, chemin |
| `iso_dump.py` | octets d'un fichier de l'ISO, en hexadécimal |
| `peek_disc.py` | taille, signatures et chaînes de fichiers du disque |
| `extract.py` | sort `SLPM_661.86` de l'ISO dans `work\` |
| `elf_sections.py` | sections de l'exécutable ELF |
| `dwarf1_symbols.py` | fonctions et variables de la section `.debug` |
| `peek_symbols.py` | noms de fonctions et fichiers sources cités dans l'exécutable |
| `peek_words.py` | instructions MIPS à des adresses, avec leur code d'opération |
| `exe_text.py` | les textes Shift-JIS de l'exécutable à des adresses (phrases des menus et des questions) |
| `mips_calls.py` | les instructions qui préparent des appels sur une plage d'adresses : retrouver les arguments qu'a perdus la décompilation |
| `mcs_messages.py` | la table des messages de l'écran de la carte mémoire (texte, sorte, suites de ○ et de ✕) |
| `find_word.py` | où sont rangés des pointeurs vers des adresses |
| `exe_table.py` | une table de données de l'exécutable |
| `uni_list.py` | contenu d'un paquet UNI2 (numéro, place, taille, début des données) |
| `tim2_png.py` | une image TIM2 (sortie par `uni_extract.py`) en PNG : la planche de l'écran de la carte mémoire (MEMORY.UNI 10) |
| `uni_extract.py` | les entrées d'un paquet UNI2, telles quelles (`SCRIPT.UNI` : les scripts) |
| `stcm2_table.py` | la table des instructions des scripts (`0x35bf60`) : numéro, adresse, fonction |
| `stcm2_dump.py` | un script STCM2 désassemblé en texte lisible (voir `docs/scripts.md`) |
| `uni_png.py` | images ART2 d'un paquet UNI2 en PNG (il faut Pillow) |
| `sprites_png.py` | toutes les images du jeu en PNG : les ART2 et TIM2 de chaque paquet `UNION/*.UNI`, un sous-dossier par paquet (357 images, une dizaine de secondes ; il faut Pillow) |
| `vbg_list.py` | les numéros d'entrée d'un fichier VBG, regroupés par milliers (où sont les voix, les musiques) |
| `vbg_wav.py` | une entrée d'un fichier VBG (voix, musiques) en WAV |
| `hd_list.py` | banque de sons Sony (.hd/.bd) de `SOUND.UNI`, et ses échantillons en WAV |
