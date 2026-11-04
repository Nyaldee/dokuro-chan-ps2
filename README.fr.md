# Game ni Natta yo! Dokuro-chan: Kenkou Shindan Daisakusen — portage de l'original

<p align="center">
  <img src="dokuro-chan.webp" alt="ぴぴるぴるぴるぴぴるぴ〜♪" width="200">
</p>

[English](README.md) · Français · [日本語](README.jp.md)

Portage Windows et Linux non officiel de *Game ni Natta yo! Dokuro-chan ~Kenkou Shindan Daisakusen~* (PS2), écrit en Rust à partir de son code décompilé. Il cherche à jouer exactement comme l'original ; seules les vraies erreurs de son code sont corrigées, et toutes sont listées plus bas.

Le jeu complet est porté : l'écran titre ; la chambre de Sakura (le menu principal, avec la liste des fins vues, l'album, le menu des mini-jeux et le mode challenge) ; l'histoire (texte, voix, décors, personnages, caméra, fondus, choix, carte des journées, mini-jeux, résultats de la journée, répartition des points, fins) et le scénario bonus ; les options ; les sauvegardes (douze fichiers, depuis le menu du jeu ; « つづきから » dans la chambre) ; les vidéos (les logos et l'ouverture au démarrage, l'ouverture au titre, les cinématiques de l'histoire, celles de l'album), décodées par la bibliothèque ps2-video (MPEG-2 en Rust pur). L'état détaillé est dans [docs/moteur.md](docs/moteur.md).

Ce dépôt ne contient **aucun fichier sous droits** : ni ressources du jeu, ni code d'origine. Le portage lit l'image disque que vous fournissez, posée à côté de son exécutable, et vérifie son CRC32 avant de démarrer. L'image peut aussi être ailleurs : indiquez son chemin (ou celui du dossier qui la contient) dans `"assets"` → `"path"` de la sauvegarde ; un chemin relatif part de l'exécutable. Son CRC32 est vérifié au premier lancement (quelques secondes), puis l'image est reconnue à sa taille et à sa date (`"assets"` → `"checked"`).

La sauvegarde va dans `Documents\My Games` sous Windows (le dossier Documents tel que Windows le connaît, déplacé ou synchronisé compris) et dans `~/.local/share` sous Linux. Avec un fichier `portable.txt` à côté de l'exécutable, elle va plutôt dans un dossier `saves` à côté de lui.

## Les mini-jeux

Les mini-jeux tels que les lance leur menu dans le jeu :

- **ドクロちゃん捕縛アミ** (*Le filet à capturer Dokuro-chan*) : Dokuro sort des neuf buissons ; l'attraper au filet vide sa jauge, un coup dans le vide la remplit. Gagné si la jauge est vide avant la fin des 40 secondes.
- **アドバンスド徒競走** (*La course à pied avancée*) : une course contre Dokuro. ○ en rafale fait courir, ↑ saute les haies, ↓ glisse ; les obstacles sonnent Sakura, une haie l'arrête. Gagné en arrivant le premier.
- **水着で神経衰弱** (*Memory en maillot de bain*) : un memory contre Dokuro, qui retient les cartes vues. Une paire rapporte 2 points et fait rejouer ; le joker rapporte 1 point et montre toutes les cartes un instant. Gagné à 8 points.
- **桜くん私物キャッチ** (*Attrape les affaires de Sakura-kun*) : Dokuro jette les affaires de Sakura par la fenêtre. Un objet qui touche le sol ou une bombe attrapée coûte un des trois cœurs. Gagné en tenant 50 secondes.
- **バッティングエスカリボルグ** (*Batting Escalibolg*) : Dokuro frappe Sakura avec sa batte, en arrêtant trois jauges (puissance, timing, angle). Gagné en dépassant 600 m en trois essais.

Ils se jouent au fil de l'histoire, puis depuis le menu des mini-jeux de la chambre de Sakura une fois joués dans l'histoire. Ce menu propose aussi le mode challenge (les cinq jeux à la suite, plus durs, chacun rapportant des points, comptés en têtes de mort ; réussi à 100), le choix du personnage qui encourage Sakura et les tutoriels. Le menu du challenge s'ouvre une fois une fin vue ; le réussir y ouvre le scénario bonus, « 12歳の世界！ », dont la fin ramène à la chambre.

## Jouer

- Fichier nécessaire : l'image ISO du disque japonais (SLPM-66186, CRC32 `B9F5F0C5`), sous n'importe quel nom finissant par `.iso`.
- Posez-la à côté de `dokuro_chan_ps2.exe` (`dokuro_chan_ps2` sous Linux) et lancez l'exécutable.
- Se joue à la manette comme sur PS2 : A pour ○, B pour ×, X pour □, Y pour △, LB et RB pour L1 et R1, les gâchettes pour L2 et R2, la croix ou le stick gauche ; Start + Select quitte. Dans l'histoire, △ lance ou arrête l'avance rapide, START la lecture automatique, R1 passe de l'avance des pages lues (既読) à l'avance forcée (強制), ✕, □ ou L1 les arrêtent ; leur signe paraît en haut à droite. La manette vibre comme la DualShock 2 quand la caméra tremble : le gros moteur par le moteur basse fréquence, le petit par le moteur haute fréquence (sous Linux, il faut pouvoir écrire dans le périphérique `/dev/input/event*` de la manette).
- Au clavier : Entrée ou Z pour ○, Retour arrière ou X pour ×, Échap pour × (pour □, le menu, dans l'histoire), C pour □, V pour △, Q, W, A et S pour L1, R1, L2 et R2 (A est la lecture automatique dans l'histoire), les flèches. Pendant la course, Espace saute et Ctrl glisse ; ailleurs, Espace vaut START.
- Dans l'histoire, comme dans un visual novel sur PC : clic gauche, Entrée ou la molette vers le bas pour la suite ; clic droit (✕) pour cacher les fenêtres ; la molette vers le haut (L1) pour l'historique, où la molette fait défiler (△ y fait revenir l'histoire à la page choisie, □ ouvre le menu) ; Échap pour le menu (aussi pendant un choix et sur la carte, comme dans le jeu) ; Ctrl tenu pour l'avance rapide, le temps qu'on le tient ; A pour la lecture automatique.
- Pendant une vidéo : START (Espace) l'arrête, comme dans le jeu ; Échap et le clic aussi. Le jeu n'en tient compte qu'après la première demi-seconde.
- À la souris, la même règle partout : survoler une entrée de menu, une réponse, une valeur des options, un fichier de la carte mémoire, une vignette, un onglet ou une feuille de l'album, une case de la répartition des points, un lieu de la carte ou le portrait du personnage qui encourage Sakura y mène le curseur ; un clic dessus vaut ○, un clic à côté ne fait rien. Là où le jeu attend n'importe quelle touche, un clic n'importe où suffit : au titre, il vaut START ; dans l'histoire, il fait avancer le texte. Le bouton droit vaut ×. Un clic sur une jauge des options (volumes, opacités, vitesses) la règle directement. Dans les mini-jeux, le filet vise le buisson survolé, le memory choisit la carte survolée, Sakura suit la souris dans le jeu des affaires. Le pointeur est l'Escalibolg de Dokuro (une image du jeu restée inutilisée) ; il se cache dès qu'on joue au clavier ou à la manette, ou après 3 secondes sans toucher la souris.
- L'écran de la PS2 (640 × 448) est affiché en 4:3, à 60 images par seconde.
- Alt+Entrée ou F11 : plein écran sans bordure.
- Sauvegarde (le mode d'affichage, ce que le jeu garde d'une partie à l'autre — fins vues, challenge réussi, mini-jeux joués, pages lues, réglages des options, images de l'album vues —, les douze parties sauvegardées, et le chemin et la vérification de l'image disque) : `Documents\My Games\Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen\save.json` sous Windows, `~/.local/share/Game ni Natta yo! Dokuro-chan Kenkou Shindan Daisakusen/save.json` sous Linux.

### Bugs du jeu corrigés

Le portage corrige les erreurs que prouve le code du jeu (jamais ce qui pourrait être voulu). Pour chacune : ce que fait le jeu, pourquoi c'est une erreur, ce que fait le portage.

- Challenge échoué, l'écran ne s'assombrit pas : pour la réussite, le jeu assombrit le plan 5 par `AD_BlackOutStart(10, 5, 0, 0x50)` (couche 0, opacité 0x50) ; pour l'échec, il appelle `AD_BlackOutStart(10, 5, 0x50, 0x7f)`, soit la couche 0x50, alors qu'un plan n'en a que deux. Rien de visible ne s'assombrit (le jeu écrit hors de sa table des fondus), et la fermeture du message, qui efface la couche 0, rend l'écran sombre d'un coup avant de l'éclaircir. Le portage assombrit l'écran comme pour la réussite (0x50 en 10 images).
- Les messages d'information ne rétrécissent pas en se fermant : `AD_InformationWinDel` enchaîne deux étapes (10 images, puis un rétrécissement jusqu'à 0 × 0), mais le moteur des fenêtres (`AD_WindowProc`) ne remet pas son compte d'images à zéro d'une étape à l'autre. La seconde commence donc déjà finie et le message disparaît d'un coup ; le même oubli fait finir d'un coup l'étape donnée à une fenêtre en train de s'ouvrir. Le portage remet le compte à zéro à chaque étape : les messages (du challenge comme de l'histoire) rétrécissent.
- Course : `MiniRunHitDown`, commune aux deux coureuses, compare leur hauteur à 294, le sol du couloir de Sakura. Dokuro, sur son propre sol, passe pour être en l'air et, à hauteur d'une haie, repasse un instant dans sa pose de saut. Le portage compare chaque coureuse à son propre sol.
- Options : en quittant la fenêtre « スキップ・オートプレイ設定 », le jeu met le curseur sur l'entrée 4 de la fenêtre de jeu puis, dans la même image, le ramène dans les quatre entrées de la fenêtre qu'il quitte : il revient tout en haut, sur « スキップモード », et le son du déplacement suit celui du retour. La même vérification ajoute ce son après celui de la validation en ouvrant le détail d'une avance. Le portage laisse le curseur sur l'entrée 4, avec le seul son du retour (ou de la validation).
- Tremblement de la caméra : en passant à la sorte 0x1b (sur les trois axes, en 8 images), le jeu oublie le facteur du passage sur le zoom, qui saute d'un coup au nouveau tremblement au lieu de s'y fondre comme pour les autres sortes. Le portage applique le facteur.
- Fenêtres : quand une action en remplace une autre en cours d'animation (par exemple une fenêtre qui se ferme alors qu'elle s'ouvre encore), `AD_WinActSet` calcule où en était l'étape par `(float)(compte / durée)`, une division entière faite avant la conversion : elle vaut 0 tant que l'étape n'est pas finie, alors que le calcul qui suit (départ + avancée × (but − départ)) attend une fraction. La fenêtre revient donc d'un coup au début de son étape. Le portage divise en nombres décimaux : la fenêtre repart de là où elle en était.
- Album, image bonus : après l'image bonus gagnée en complétant l'album, le jeu doit attendre 0x14 images que le noir tombe avant de montrer l'album, mais son test est inversé (il ne décompte que si le compte est déjà sous 1, et passe sinon) : l'album paraît aussitôt, sans que le noir ait eu le temps de tomber. Le portage attend les 0x14 images.
- Lecture automatique, le signe en haut à droite : les étiquettes « オート », « 強制スキップ » et « 既読スキップ » font 119 × 21 pixels dans l'image `win2`, en lignes 2, 26 et 50. Le jeu découpe bien les deux dernières, mais prend « オート » depuis la ligne 0 : deux lignes vides en haut, et les deux lignes de son bord du bas en moins, si bien que l'étiquette paraît coupée. Le portage la découpe depuis la ligne 2.
- Bruitages en boucle : le jeu garde le numéro de la boucle en place dans chacune de ses deux places, et ne les remet à zéro qu'au démarrage (`AD_SoundInit`). En retournant dans la chambre pendant une scène où une boucle tourne, le son se coupe mais son numéro reste ; dans la partie suivante, ouvrir les options ou la carte mémoire depuis le menu du jeu la relance au retour (`AD_LoopSERePlay`), et elle tourne jusqu'à ce qu'un script arrête cette place. Le portage oublie les boucles en place en quittant l'histoire.

### Ce que le jeu contient sans s'en servir

L'étude de l'exécutable et des 35 scripts du disque (le bilan de `Tools/coverage.py`) montre du code présent mais inaccessible, probablement abandonné en cours de développement. Le portage ne le reprend pas, puisque le jeu ne peut pas l'atteindre.

- Un mode combat (fonctions `AD_Dcb*`, environ 40 Ko de code). On y choisit un personnage, puis on appuie au bon moment : un repère tourne sur un cercle où sont posées des cibles, et chaque cible touchée rapporte des points. Autour : points de vie, jauge, cut-ins plein écran, renforts, tours, une arme en fond (`dcbgun`), des écrans de victoire et de défaite (`dcb_win`, `dcb_lose`). Il se jouerait sur le décor et avec les personnages de l'histoire. L'histoire le lancerait par un lot de sorte 6, mais aucune instruction de script ne produit cette sorte ; ni ses quinze images ni la table de ses personnages ne sont sur le disque.
- Un pavé numérique (fonctions `AD_TenKey*`, sorte de lot 5), disposé comme un clavier de téléphone (1 à 9, *, 0, #), pour faire composer un nombre au joueur, avec ses textes de bonne et de mauvaise réponse : ses instructions (`tenkey_mode`, `tenkey_ok_text_set`, `tenkey_ng_text_set`) n'apparaissent dans aucun script, et ses deux images (`tenkey`, `tenkeybg`) ne sont pas sur le disque.
- Un moteur de sprites et de modèles 3D (`SP_SYSTEM`, `MGP2`, particules, morphing), qui tourne à chaque image mais reste vide : aucun script ne se sert de ses instructions.
- Des images bonus de l'album, montrées à la fin du jeu selon un drapeau ou quand l'album est complet (`AD_CgBonusGetJudg`) : aucun script n'en inscrit (`bonus_cg_flag_entry`, `bonus_cg_spflag_entry`, `bonus_cg_comp_entry`). Seules servent les images bonus des mini-jeux. Le portage reprend le mécanisme, qui reste inutilisé comme dans le jeu.
- À l'écran titre, une question oui / non que rien n'ouvre.
- Dans la chambre, une entrée qui lancerait un prologue à part (`AD_AdvPartStartProc(0)`) : aucun script ne l'inscrit (`prologue_stcm`), et le menu n'a pas d'image pour elle ; le curseur la saute.
- Dans le script 3, toutes les vidéos inscrites une seconde fois, à des chemins de développement (`host0:pss/…`, des fichiers lus sur le PC de développement), dans un passage que le jeu ne lance pas.

### Ce qui diffère de la PS2

- Les attentes de lecture du disque n'existent pas : là où le jeu attend que ses données soient chargées (`AD_DataReadStateGet`) ou qu'une musique se taise, le portage enchaîne. Une vidéo démarre ainsi plus tôt après la page qui la précède, et l'histoire reprend plus tôt après elle.
- Après une vidéo de l'histoire, la musique relancée ne s'entend qu'au bout d'un quart de seconde. Le jeu la relance puis la coupe aussitôt quand la scène suivante est silencieuse : sur PS2, le temps de lecture du disque empêchait de l'entendre, alors que le portage, qui lit instantanément, la faisait revenir un instant. Toutes les autres musiques partent aussitôt.
- Les vidéos sont décodées à l'identique, mais leur couleur, stockée à demi-résolution, est interpolée entre ses échantillons au lieu d'être répétée par blocs de 2 × 2 comme le fait l'IPU de la console : les bords de couleur sont plus nets, sans rien ajouter à l'image. Leur son passe à plein volume, comme sur PS2.
- Là où le jeu écrit ses données système sur la carte mémoire (en quittant des options modifiées), le portage écrit sa sauvegarde, sans l'écran de la carte mémoire. Les drapeaux gardés y vont dès qu'ils changent, les pages lues à chaque attente (une page, un choix, la carte), quand le jeu les copie avec ses données système ; l'avance choisie par R1, en sauvegardant une partie.
- Les parties sauvegardées vont dans la sauvegarde du portage, avec l'écran de la carte mémoire du jeu, ses messages et ses questions (la première fois, il propose de créer les données du jeu, comme sur une carte neuve). Les erreurs d'une carte absente, pleine ou abîmée n'ont pas lieu d'être. Les opérations sont immédiates : leur message reste une demi-seconde.

## Vérifier son fichier

Le CRC32 indique si l'image disque est la bonne version. Vérifiez-le avec 7-Zip (clic droit > CRC SHA > CRC-32) ou avec Python :

```
python Tools/crc32.py <fichier>
```

## Compilation

Prérequis : [Rust](https://rustup.rs) (stable).

### Windows

```
build.bat
```

compile le portage en release et écrit l'archive prête à partager (l'exécutable seul, sans fichier du jeu) : `target\Dokuro-chan.PS2.Windows.zip`.

### Linux

Il faut aussi `pkg-config` et les en-têtes ALSA (`sudo apt install pkg-config libasound2-dev` sous Ubuntu). Sous Linux :

```
./build.sh
```

Depuis Windows, `build_linux.bat` fait la même chose dans WSL (Ubuntu). L'archive est écrite dans `target/Dokuro-chan.PS2.Linux.tar.gz`.

## Organisation

| Dossier | Contenu |
|---|---|
| `src/` | le portage |
| `Tools/` | outils d'étude du jeu : vérification, extraction du disque, décompilation avec Ghidra, export des images et des sons (voir `Tools/README.md`) |
| `docs/` | l'étude du jeu : le format des scripts ([scripts.md](docs/scripts.md)), le moteur et l'état du portage ([moteur.md](docs/moteur.md)) |
| `decompiled/` | le code du jeu décompilé, référence de lecture pour le portage (ignoré par git, produit par `Tools/decompile.bat`) |

### Refaire l'étude

Tout se refait à partir de l'ISO posée à la racine du dépôt (voir `Tools/README.md` pour le détail) :

1. `Tools\decompile.bat <dossier de Ghidra>` : l'exécutable sorti de l'ISO, ses noms de fonctions, le pseudo-C de tout le jeu dans `decompiled/`.
2. `python -I Tools/uni_extract.py <iso> UNION/SCRIPT.UNI Tools/work/script` : les scripts de l'histoire.
3. `python -I Tools/stcm2_table.py Tools/work/SLPM_661.86 Tools/work/symbols.txt > Tools/work/stcm2_opcodes.txt`, puis `python -I Tools/stcm2_dump.py <script>.bin Tools/work/stcm2_opcodes.txt` : les scripts en texte lisible.
4. `python -I Tools/uni_png.py <iso> <secteur> <dossier>` : les images d'un paquet (secteurs donnés par `Tools/iso_list.py`) ; `python -I Tools/sprites_png.py <iso> <dossier>` sort d'un coup toutes les images du jeu.

### Tester

`cargo test` : les tests (ceux qui lisent les vrais scripts ne tournent que si l'ISO est à la racine du dépôt). En debug, des variables d'environnement pilotent le jeu sans fenêtre à regarder : `DOKURO_SNAPSHOT=<images>:<fichier.png>` (une capture, puis le jeu quitte), `DOKURO_EVERY=<n>[@<image>]` (une capture toutes les n images, à partir de cette image), `DOKURO_INPUT=<image>[-<image>]:<bouton>;…` (des appuis, ou un bouton tenu d'une image à l'autre), `DOKURO_STORY=<script>[:<point d'entrée>]` (l'histoire à cet endroit, par exemple `102:sure2`), `DOKURO_TRACE=1` (chaque point d'entrée joué, écrit sur la sortie d'erreur : de quoi reprendre l'histoire juste avant ce qu'on teste), `DOKURO_BOOT=1` (les vidéos du démarrage, qu'un test saute sinon : il commence au titre) et `DOKURO_ZONES=1` (le contour rouge des zones de la souris de l'écran). `cargo test audit -- --ignored --nocapture` suit l'histoire sur les vrais scripts, sans fenêtre (la première réponse à chaque choix, le premier lieu de la carte), et liste les scènes où un zoom, un décalage de la caméra, un tremblement, un plan de fondu ou un personnage hors de l'écran dure dix pages ou plus, avec le point d'entrée et le lot où cela commence, ainsi que les vidéos rencontrées ; `DOKURO_AUDIT=<script>:<point d'entrée>` le fait partir d'ailleurs (une fin, par exemple `121:sure1`).

Sous Windows, `Tools\Tester.bat` règle ces variables depuis un menu, puis lance le jeu, l'audit ou les tests.

En debug, le portage cherche aussi l'image disque à la racine de ce dépôt (`*.iso` est ignoré par git).

## Crédits

*Game ni Natta yo! Dokuro-chan* (2005) est tiré des romans de Masaki Okayu (MediaWorks), édité par Idea Factory et développé par Design Factory. Tous les droits sur le jeu, ses personnages et ses ressources appartiennent à leurs propriétaires. Ce portage est non officiel et sans lien avec eux.

## Licence

Copyright (C) 2026 Nyaldee. Distribué sous licence [GNU General Public License v3.0](LICENSE) — voir le fichier `LICENSE` pour le texte complet.

[Claude](https://claude.com), l'IA d'Anthropic, a contribué au code de ce projet.
