# Le moteur d'aventure

Comment le jeu joue l'histoire, d'après le pseudo-C de l'exécutable (`decompiled/adv.c`, `adv_gp.c`, `stcm2.c` ; voir `Tools/decompile.bat`). Les adresses sont celles de la mémoire de la PS2 ; elles servent de repères pour relire le code d'origine.

## Le principe : le script prépare, le moteur joue

`adv_main` (0x1d6640) relance le script en cours (fichier `0x1da7328`, point d'entrée `0x1da732a`) **depuis son point d'entrée**, par `AD_ScrirptProc` : le fil STCM2 s'exécute alors d'un bloc, sans jamais attendre, jusqu'à son `return`. Ce faisant, les instructions ne font rien à l'écran : elles remplissent des **lots** (`batch`), 64 au plus, chacun une page de texte avec tout ce qui l'accompagne (décor, personnages, musique, voix, fondus, caméra…). Puis `adv_main` joue les lots un à un ; quand il n'y en a plus, il relance le script.

Un chapitre est donc découpé en morceaux de moins de 64 pages par un drapeau (souvent `switch`) : le script lit le drapeau au départ (`flag_number`), saute au morceau voulu, et à la fin du morceau augmente le drapeau (`flag_change('switch', 1)`) puis rend la main (`return`). Le dernier morceau passe au chapitre suivant (`file_change(129, 'sure1')`).

Les drapeaux (`flag_*`) sont donc l'état de l'histoire. Une partie sauvegardée garde le point d'entrée joué, les drapeaux à son départ et le lot de la page : le chargement rejoue le point d'entrée pour retrouver ses lots (voir « La sauvegarde et le chargement »).

## Les lots (0x1268 octets, à partir de 0x1da7770)

`cRam01da7319` est le numéro du lot que remplit le script (−1 au départ ; `batch` l'augmente), `cRam01da731a` celui que joue le moteur.

| Décalage | Contenu | Écrit par |
|---|---|---|
| 0x000 | le texte : les lignes, chacune suivie de `#n` | `maintxt_print` |
| 0x190 | la sorte du lot (voir ci-dessous) | `*_mode`, `menu_set`, `movie_play`… |
| 0x192, 0x194 | un nom à afficher (1), le nom | `speaker_change` |
| 0x1b2, 0x1b4 | un message d'information (1), son texte | `infor_win` |
| 0x226, 0x228 | interdictions du lot : 4 menu, 8 texte, 0x10 fenêtre ; levées | `restrict_*`, `restoff_*` |
| 0x22a | la route | `route`, `routeend` (−2) |
| 0x22c | le nom du décor | `bg_print_change`, `ezbg_change` |
| 0x240 à 0x246 | décor : sorte de transition, variante, durée, retard | `bg_print_change` (`ezbg_change` : 0, au hasard 0 à 8, n × 30, 0) |
| 0x24c… | caméra | `camera_set` |
| 0x26c + 0x6e × c + 0x16 × e | la caméra `c`, étape `e` : durée (0x26c), zoom (0x26e), x (0x270), y (0x272), angles (0x274 à 0x278), tremblement (0x27a, 0x27b), couleur (0x27c à 0x280) | `target_*_set` |
| 0x34c + 2 × p | fondu au noir du plan `p` : 0 apparaît, 1 s'efface, 2 éclair | `blackout_*` |
| 0x35a…, 0x361…, 0x368…, 0x376…, 0x37e… | ses valeurs (par plan) | |
| 0x352, 0x35d, 0x364, 0x36e, 0x379, 0x384, 0x3a2, 0x3a4 | changement de scène (fondu propre) | `ezeffect_scenechange2` |
| 0x38c, 0x392…, 0x390 | effets à particules (deux) et leur durée | `effect_play` |
| 0x3aa | durée minimale du lot (le plus long de ce qu'il lance) | décor, transitions |
| 0x3a8, 0x3ac, 0x3ae | attentes dues aux personnages | `ezchara_*` |
| 0x3ae | attente imposée | `txtwaitset` |
| 0x3b3 | le nom d'une vidéo | `movie_play` |
| 0x3c8, 0x3ca | musique : numéro (−1 : arrêt), retard | `bgm_play`, `bgm_stop` |
| 0x3cc + 4 × k | bruitages (50) : numéro (−1 : arrêt), retard | `se_play`, `se_stop` |
| 0x494 + 4 × k | bruitages en boucle : numéro, retard | `loopse_*` |
| 0x49c, 0x4a0 | la voix : numéro, retard | `voice_play` |
| 0x4a4, 0x4a8 | bruitage en flux : numéro, retard | `strse_play` |
| 0x4aa | le flux peut être coupé (1) | `strse_waitcut` |
| 0x4ac à 0x4b4 | l'heure | `time_set`, `time_change` |
| 0x4b6 + 0x248 × k | les personnages (voir ci-dessous) | `ezchara_*`, `trchara_*` |

Un retard en paramètre (`n`) devient `n × 7,5 + 1` images (`n × 7,5 + (durée du changement de scène) / 2 + 15 + 1` pendant un changement de scène).

### La sorte d'un lot (0x190)

| Valeur | Lot |
|---|---|
| 0 | une page de texte, qui attend ○ |
| 1 | pas de texte (le lot est rempli d'autre chose) |
| 2, 3 | un choix (3 : sans attendre) |
| 4 | la carte |
| 5 | le pavé numérique (inutilisé : `tenkey_mode` n'est dans aucun script) |
| 6 | un combat (`AD_Dcb*`, environ 40 Ko de code ; inutilisé : aucune instruction ne produit cette sorte) |
| 7 | le résultat de la journée |
| 8 | la répartition des points |
| 9 à 0x0d | un mini-jeu : filet, attrape, memory, course, batte |
| 0x0e | une vidéo |
| 0x0f | la fin du jeu |

### Les personnages d'un lot (0x248 octets chacun)

Le personnage est trouvé par son nom (`AD_AdvBthCharaNum`). Son mot 0 compte ce qu'on lui a fait : +10 `ezchara_stateset`, +0x14 `ezchara_nextstate`, +100 `ezchara_app` (apparaît), +200 `ezchara_dis`, +300 `ezchara_del` (disparaît), +400 `ezchara_move` ; une seule apparition, disparition ou déplacement par lot, un seul changement d'état.

| Décalage | Contenu |
|---|---|
| 0x0ce | le nom |
| 0x0e2 + 0x14 × k | (5) l'image de chaque morceau |
| 0x146 + 0x14 × k | (5) le motif de chaque morceau |
| 0x1aa + 0x14 × k | (5) l'animation de chaque morceau |
| 0x20e | 0 |
| 0x20f, 0x210 | déplacement (1), disparition (1) |
| 0x211, 0x212, 0x214 | changement d'état en fondu (1 : `nextstate`, 0 : `stateset`) |
| 0x213, 0x215 | animation, couleur |
| 0x216 | numéro (ordre) |
| 0x218, 0x21a | apparition 3, disparition 4 ; sorte (0 apparaît, 1 `dis`, 2 `del`) |
| 0x222 | x d'arrivée |
| 0x224 | x de départ |
| 0x22c | durée du mouvement (n × 7,5, au moins 2) |
| 0x22e | −1 |
| 0x236 + 3 × k | (5) couleur de chaque morceau (r, g, b) |
| 0x246 | retard (n × 7,5) |

Les places : l'écran (640) est partagé en `n` colonnes ; la colonne `k` a son centre en `640 / n / 2 + 640 / n × k`. `ezchara_app(nom, n, k, décalage, durée, retard)` part de ce centre plus `décalage × 30`.

## La fenêtre de texte

`AD_AdvMainWinMake` crée deux fenêtres (`dialog.rs` en porte le cadre) :

| Fenêtre | Style | Priorité | Centre | Taille | Réglages (`AD_WinStateSet`) |
|---|---|---|---|---|---|
| `mainwin` | 0 ou 1 (`txtwin_type`) | 0x46 | 0x140, 0x16d | 0x25d × 0x78 | ouverture 2, 3, 5 ; ombre en palette 8 ; texte à 0x19, 0x14 du coin |
| `speaker` | 2 (rose) | 0x4b | 0x32 + 6 × n, 300 | (12,5 × n + 33,5) × 0x28 | ouverture 2, 4, 4 ; ombre 8 ; texte à 0x14, 6 |

(`n` : la longueur du nom suivi d'une espace, en octets.) Les opacités du cadre et du fond viennent des options (`AD_OptionTxtWinAlphaGet`). Le nom ne change que s'il diffère du nom affiché (`AD_AdvSpeakerChange`). Quand la fenêtre revient (`AD_AdvMainWinOpen` : après ✕, l'historique, un menu), son texte paraît d'un coup (`AD_WinTxtStateSet(0, …)`).

### L'apparition des lettres (`MkFontPrint`, `RunFontSystem`)

Chaque lettre a un compte qui part d'une valeur négative et gagne 1 par image : la première à 0, chacune des suivantes plus bas de la vitesse du texte. Une lettre se montre quand son compte atteint −10 : son opacité monte de 0 à 0x7f et elle glisse de 10 pixels vers la gauche jusqu'à sa place pendant ces dix images. La durée du texte (`MkFontTxtTimeGet`) est l'opposé du plus petit compte ; appuyer sur ○ pendant l'apparition met tous les comptes à 0 (`AD_WinTxtSpeedChange(0)`), sauf pendant les deux dernières images.

Les commandes dans le texte : `#n` (ligne suivante, 29 pixels plus bas, au début de ligne), `#Color[n]` (palette, si la fenêtre n'en impose pas), `#Pos[x,y]` (la suite en x, y, et les lignes suivantes en x), `#speed` (vitesse), `#effect` (0 à 5 : une lettre qui grossit, ondule…), `#rand` (un nombre), `#scale`, `#[…]` (une image en ligne).

### Le signe d'attente

`AD_AdvEnterTran` : l'image `win2` (rectangle de `0x3657c0`) en 0x216, 0x195 + 10 × sin(2 t), prio 0x47, opacité 128 × sin(4 t), avec t qui tourne de 0 à 44 (degrés).

## Le décor (`AD_BGPrintChange`, `AD_BGProc`, `AD_BGTran`)

Deux images de décor alternent (`back0`, `back1`) : la nouvelle se pose sur l'ancienne. Les scripts n'utilisent que le fondu enchaîné (sorte 0) : la nouvelle image monte de l'opacité 0 à 127 en `durée` images, après `retard` images. (Le moteur sait aussi faire apparaître l'image par 64 carreaux, dans neuf ordres : au hasard, par lignes, en spirale… ; aucun script ne s'en sert.)

Un décor fait 512 × 448 de son image ART2 de 512 × 512, étiré sur l'écran de 640 × 448 (`AD_GpCameraTran` : centre 320, 224), à travers la caméra.

Chaque décor a sa caméra (`0x1be926c` + 0x20 × décor : x, y, zoom, angle, sorte et niveau du tremblement…). `AD_BGPrintChange` donne au nouveau décor une caméra en (0, 0) à 100 % et sans tremblement (`camera_set`, 0x12d, pourrait la régler autrement : aucun script ne s'en sert) ; les étapes de caméra du lot s'y appliquent, ainsi qu'aux personnages. Celle du décor qui s'en va reste où elle en était jusqu'à la fin du fondu enchaîné. Un zoom n'a donc pas à être défait avant un changement de décor : les scripts comptent dessus (après les vidéos du début, le zoom à 230 du lot précédent n'est jamais remis à 100). La même remise à zéro vaut pour l'image bonus d'un mini-jeu et le décor qui revient ensuite (`AD_BGPrintChangeBk`, qui passe 0, 0, 100).

Pendant un changement de scène (`ezeffect_scenechange2`), le décor du lot change d'un coup, au milieu de l'éclair (retard = durée de l'éclair).

## Les fondus (`AD_BlackOut*`)

Sept plans, de priorités 0x1c, 0x39, 0x3a, 0x9e, 0xda, 0xe4, 0xfa ; les scripts se servent surtout du plan 2 (sous la fenêtre de texte) et du plan 3 (les éclairs des changements de scène). Un fondu a une couleur (`color_set` : 0 noir, 1 blanc, 2 rouge, 3 bleu, 4 violet, 5 jaune, 6 vert, 7 orange), une opacité, une durée et une forme :

| `a` | Forme |
|---|---|
| 0 | tout l'écran, linéaire sur la durée |
| 1 | 40 bandes verticales de 16 pixels |
| 2 | 56 bandes horizontales de 8 pixels |

Avec des bandes, chacune attend `(durée / 2) × k / n` images puis passe à l'opacité visée en `durée / 2` images ; `b` donne l'ordre des bandes (`AD_BlackOutMakeEffectProc`) : 0 dans l'ordre, 1 à rebours, 2 et 3 depuis les bords ou le milieu, 4 une sur deux.

`blackout_start` noircit et reste noir ; `blackout_end` repart de l'opacité du dernier départ ; un éclair (`blackout_flash`, changement de scène) apparaît puis s'efface (`1 + 2 × répétitions` passages).

## Le portage : état

Le portage suit les boucles du jeu : `src/game.rs` fait tourner l'une ou l'autre comme l'aiguillage `PTR_adv_title` de l'exécutable — l'écran titre (`adv_title`, `src/title.rs`), le menu de la chambre (`adv_mainmenu`, `src/mainmenu.rs`) ou l'histoire (`adv_main`, `src/adv.rs`) ; une partie de mini-jeu (`AD_MiniGame*`, `src/minigame.rs`) sert aux deux. Les services communs (manette, fondus, sons) sont dans `src/sys.rs`, ce qui se garde d'une partie à l'autre dans `src/record.rs`.

| Partie | Fichier | État |
|---|---|---|
| Machine des scripts (STCM2_Run) | `src/stcm2.rs` | faite, testée sur les vrais scripts |
| Remplissage des lots, drapeaux, inscriptions | `src/story.rs` | fait (les ~80 instructions des scripts) |
| Décor (fondu enchaîné), caméra et son tremblement, vibration, personnages, fondus en bandes | `src/scene.rs`, `src/shake.rs`, `src/rumble.rs` | fait (les instructions d'animation des personnages ne sont dans aucun script) |
| Enchaînement des lots, page, voix, attente de ○, choix | `src/adv.rs` | fait ; fenêtres cachées par ✕, menu du jeu (□), vitesse du texte et opacités des options, avance rapide, lecture automatique, pages lues (`src/readmode.rs`, `src/bread.rs`) ; les passages d'un lot à l'autre (`AD_ModeChangeProc1` à 5) pour les pages, la carte et le résultat de la journée |
| Historique | `src/backlog.rs`, `src/adv.rs` | fait, avec ses icônes et le retour à une page (`AD_RewindLoad`) |
| Mini-jeux dans l'histoire, image bonus | `src/adv.rs`, `src/minigame.rs` | faits |
| Chambre (menu principal), menu des mini-jeux, challenge, scénario bonus, fins vues, album | `src/mainmenu.rs`, `src/cglib.rs` | faits, vidéos de l'album comprises |
| Fin du jeu (`game_end`, état 0x16), images de l'album vues et image bonus | `src/adv.rs`, `src/album.rs` | faites ; la sauvegarde système sur la carte mémoire est celle du portage |
| Options (depuis la chambre et le menu du jeu) | `src/options.rs` | faites ; l'écran de la carte mémoire (le portage écrit sa sauvegarde) mis à part |
| Drapeaux spéciaux, mini-jeux joués, pages lues et réglages gardés | `src/record.rs` | fait |
| Écran titre | `src/title.rs` | fait, avec la vidéo d'ouverture après une minute d'attente |
| Carte des journées | `src/map.rs`, `src/adv.rs` | faite, avec son arrivée (`AD_MapStEffectMake`) et l'effet du lieu choisi (`AD_MapOkEffectMake`) |
| Résultat de la journée | `src/result.rs`, `src/adv.rs` | fait |
| Répartition des points (`distribute_mode`, script 128) | `src/adv.rs`, `src/result.rs` | faite |
| Sauvegarde et chargement (menu du jeu, つづきから) | `src/savedata.rs`, `src/mcs.rs`, `src/adv.rs` | faits, depuis une page, un choix ou la carte ; les erreurs de la carte mémoire n'ont pas lieu d'être |
| Vidéos PSS (MPEG-2) | `src/movie.rs`, bibliothèque `ps2-video` | faites : au titre, dans l'histoire (état 8), dans l'album |

En debug, `DOKURO_STORY=<script>[:<point d'entrée>]` lance l'histoire à cet endroit (par exemple `DOKURO_STORY=102:sure2`), avec les captures de `DOKURO_SNAPSHOT`.

## Le menu de la chambre (`adv_mainmenu`, état 1)

Le fond `mainmenu` (SYSTEM.UNI 16, décor de 640 × 448), la musique 3. Huit entrées (`AD_MainmenuMainTran`, image `parts`, rectangles en `0x35d520`), l'une sous l'autre à partir de y = 126, tous les 0x28 pixels, centrées en x = 0x177 + w / 2 ; la choisie agrandie de 1,05, prio 0xdc. L'entrée 2 n'existe pas (le curseur la saute) : dans le code, elle lancerait le prologue (`AD_AdvPartStartProc(0)`), qu'aucun script n'inscrit.

| Entrée | Rectangle | Effet |
|---|---|---|
| 0 | 0, 0, 142, 31 | nouvelle partie (`AD_AdvPartStartProc(2)`) |
| 1 | 150, 0, 141, 30 | charger (état 2, `AD_LoadMcProc`, voir plus bas) |
| 3 | 0, 40, 114, 30 | album (`AD_CgLibMain`) |
| 4 | 123, 40, 143, 31 | mini-jeux (états 3 à 6, déjà portés) |
| 5 | 274, 39, 143, 32 | options (état 8, voir plus bas) |
| 6 | 0, 79, 174, 33 | les fins vues (état 7, `AD_MainmenuEndcheckTran`) |
| 7 | 235, 79, 58, 32 | retour au titre (question « 0x364280 », 0x148 × 0x24) |

Le curseur `kasol` en x = 0x15e, y = 0x7e + 0x28 × k (k − 1 au-delà de l'entrée 2), glisse en 5 images ; ↑ ↓ (répétées après 0x14 images, toutes les 5) le déplacent, en boucle sur 8.

Un personnage tient compagnie (`AD_MainmenuCharaSet`) : il est tiré du compte de la musique (n % 5), avec une de ses deux bulles (n & 1). Son portrait `menu_chara` (SYSTEM.UNI 7, rectangles en `0x35d5b0` : 8,28,180,170 ; 201,23,190,175 ; 7,232,146,188 ; 168,232,133,188 ; 315,220,127,201) est posé en bas, centré en x = 0xbf, y = 0x198 − h ; la bulle `menu_hukidasi` (219 × 121 depuis 0, 0) en 0x45, 0x69. Le texte de la bulle (fenêtre `charatxt`, style 1, prio 0xde, centre 0xbc, 0x9e, 0xcd × 0x5a, sans fond ; texte à 3, 5, en palette 6, `#Scale[0.9]`, apparition immédiate) : les trois lignes de `chara_txt_set(perso, bulle, ligne)` du script 4.

## Le menu des mini-jeux dans le jeu complet (`adv_mainmenu`, état 3)

Sept entrées (`AD_MainmenuGameTran`), l'une sous l'autre (pas de h / 2 × 2 + 5), centrées en x = 0x78 + w / 2 : les cinq jeux, le challenge, le retour (237, 276, 59, 32). Un jeu jamais joué dans l'histoire (`AD_MiniGamePlayCheck` < 0) s'affiche « ??? » (303, 278, 83, 30) et ne se lance pas ; le challenge aussi, tant que son drapeau d'ouverture (`minigame_challenge_openflag_set`, un drapeau gardé) est à 0. ✕ ou le retour ramènent à la chambre (état 0, sous-état 0x14).

Le menu du challenge (état 4, `AD_MainmenuChallengTran`) a huit entrées : les cinq jeux, l'explication (`0x35d5a8`), le scénario bonus, le retour. Le scénario bonus (le script 127, « 12歳の世界！ », inscrit par `bonus_stcm` dans `init`) s'affiche « ??? » (`0x35d598`) tant que le drapeau spécial que nomme `bonus_scenario_openflag` est à 0 (`AD_AdvBonusScenarioPlayCheck`) ; ce drapeau est « challenge », celui que met à 1 le challenge réussi. Ouvert (303, 78, 199, 32 : `0x35d5a0`), ○ le lance : le noir en 10 images, puis `AD_AdvPartStartProc(1)` ; sa fin ramène à la chambre.

## Les données gardées d'une partie à l'autre (`AD_AdvSystemSaveProc`)

Les 15 drapeaux spéciaux (`spflag_*` : fins vues, album complet, challenge ouvert et réussi…), ce que chaque mini-jeu a été joué (`AD_MiniGamePlayRecord`, `0x1adb3d2`), les images vues de l'album (`AD_CgLibCgRecord`, à chaque changement de décor), ce qui a déjà été lu (`AD_BRead*`), les options. Le jeu les copie dans la sauvegarde d'une partie à chaque attente (une page par `AD_BackLogSubSaveProc` → `AD_RewindSave` → `AD_AdvSaveProc`, un choix, la carte…), et les écrit sur la carte mémoire avec une partie sauvegardée ou en quittant des options modifiées ; il ne les relit de la carte qu'une fois, au premier chargement. Le portage écrit les drapeaux dès qu'ils changent, les pages lues telles que le jeu les a copiées à la dernière attente.

## La fin du jeu (`game_end`, état 0x16 d'`adv_main`)

`game_end` (0x123) fait du lot la fin. (Aucun script n'inscrit d'image bonus de l'album, `bonus_cg_*_entry` : la recherche ci-dessous ne trouve jamais rien dans le jeu, comme celle de l'album complet.) `AD_ModeChangeProc1` pose le noir (0xf images) ; l'état 0x16 attend 0x14 images, supprime la fenêtre de texte et les personnages, puis cherche l'image bonus de l'album (`AD_CgBonusGetJudg(0)` puis `(1)` : celle de `bonus_cg_flag_entry` ou `bonus_cg_spflag_entry`, la première posée, si elle n'a pas encore été vue et que son drapeau, ordinaire ou spécial, a la valeur voulue). Trouvée, elle remplace le décor, est notée vue, et le noir s'efface en 0x14 images ; ○ (après 0x14 images) passe, et le jeu cherche de nouveau. Puis, 0x14 images plus tard (une seule sans image), la sauvegarde système (`AD_SystemMcProc` : le portage écrit la sienne) et le retour : au titre pour une partie commencée par はじめから ou chargée (`AD_AdvPartStartProc(2 ou 4)`, `0x1da7594` à 0), à la chambre pour le scénario bonus (`AD_AdvPartStartProc(1)`, `0x1da7594` à 1). Le prologue (`AD_AdvPartStartProc(0)`, lui aussi ramené à la chambre) n'est jamais inscrit : aucun script n'a `prologue_stcm`.

## L'album : ce qui a été vu (`AD_CgLib*`, `src/album.rs`)

`cglib_entry` et `cglib_movie_entry` (page, place, nom, planche et rectangle de la vignette) rangent un décor ou une vidéo dans l'album (sept pages de 64 places, `0x1ba7400`) et lui donnent une fiche pas encore vue (`AD_CgLibCgEntry` : 0x82 fiches au plus en `0x1ba9000` : vue ou non, image ou vidéo, numéro du décor dans `bg_entry` ou de la vidéo dans la table des vidéos). Chaque changement de décor (`AD_BGPrintChange` → `AD_CgLibCgRecord`) et chaque vidéo (`AD_MVPlay` : au titre, dans l'histoire, dans l'album) marquent la fiche vue. La table des vidéos du jeu tient celles de l'exécutable (« op », « if », « df », « mate ») et celles des scripts (`movie_entry`) ; l'album range « op » (page 6, place 0, son nom écrit comme un nombre : 28783). Le portage numérote d'abord celles des scripts, puis celles de l'exécutable (les numéros ne servent qu'à l'album ; ainsi ceux de ses sauvegardes ne bougent pas). Les fiches vont avec les données système ; relues, elles remplacent celles des scripts (le portage y ajoute les places qui manqueraient, pas encore vues). Toutes les fins vues, l'écran des fins note leur image (`AD_EndcheckGpgLibCgRecord`).

## L'écran de l'album (`AD_CgLibMain`, `AD_CgLibKasolProc`, `AD_CgLibTran` ; `src/cglib.rs`)

L'entrée 3 de la chambre, depuis le noir. Si l'image bonus de la troisième sorte (`bonus_cg_comp_entry`) est gagnée — il ne reste qu'une place de l'album pas vue —, elle paraît d'abord (notée vue, le noir s'efface en 0x14 images), attend ○ (après 0x14 images), puis le noir tombe en 0x14 images. (Le jeu inverse ici son attente — il décompte tant que le compte est sous 1, et passe sinon — et l'album paraît sans attendre le noir : bug corrigé.) L'album paraît alors : son fond (le décor `cglib`, `SYSTEM.UNI` 15, déclaré par le jeu et non par les scripts), le noir qui s'efface en 0x1e images, la fenêtre `cg` (style 2, prio 0xa0, centre 0x1e0, 0x2d, 0xd2 × 0x25, opacités 0x78, texte « #Speed[0]達成度　%3d％ » à 0x10, 4 du coin : le pourcentage des places vues de la page, images et vidéos, chacune d'après sa sorte : `AD_CgLibCgCheck2(numéro, sorte de la place)`), le cadre du curseur sur le premier onglet. Les touches attendent 0x1e images.

| Élément | Image | Place (centre) |
|---|---|---|
| onglets des sept pages (`cgicon`, `SYSTEM.UNI` 20) | 0, 32p + 1, 120 × 30 | 80, 60 + 40p ; opacité 0x7f pour la page ouverte, 0x5a sinon |
| « 戻る » | 0, 225, 120 × 30 | 80, 400 |
| feuilles de la page (16 vignettes chacune, jusqu'à quatre) | 130, 32f + 3, 45 × 26 | 240 + 100f, 410 |
| vignettes (`cglib_entry` : planche `thum_gp_entry` et rectangle) | — | coin 0xa5 + 0x73c, 0x50 + 0x4b r, 0x5a × 0x3f ; noires si pas vues |

Le cadre (`Cursor_Make` de sorte 5, `Cursor_Tran5`) entoure ce que vise le curseur : une vignette (90 × 63), un onglet (120 × 30), une feuille (45 × 30, à 410) ou « 戻る » (120 × 30, à 400) ; il y glisse en 6 images, adouci ((sin(t × 180 − 90) + 1) / 2), et les touches attendent 4 images. Il est fait de traits (`GP_Paint_Line`) : un halo de demi-largeur 3 (prio 0xdb, opacité a), le bord de demi-largeur 1 en haut et en bas, 2 sur les côtés (0xdc, 2a), et des traits clairs de demi-largeur 4 (0xdd) qui parcourent le haut et le bas en 120 images (60 pour s'allonger, 60 pour s'effacer par leur départ), les côtés une demi-période plus tard. Sa couleur dérive d'une unité par image vers une couleur tirée au hasard parmi sept (`EtColorSet` 1 à 7 : blanc, rouge, vert, bleu, jaune, violet, cyan), puis une autre.

Les touches : L1 et R1 changent de feuille, L2 et R2 de page, ✕ referme. Sur les vignettes, les flèches vont de l'une à l'autre (après 10 images, toutes les 6) et sortent vers les onglets (à hauteur de la rangée) ou les feuilles ; ○ (lu sans attente, `0x154998c`) ouvre une image vue : le plan 5 au noir en 0xf images, l'image en grand à la 19ᵉ image (`AD_BGPrintChange`), le plan 5 s'éclaircit à la 20ᵉ ; ○ ou ✕ la referment (le plan 5 au noir en 0xf images, puis l'album revient). Une vidéo vue se joue en grand (`AD_MVPlay`). Sur les onglets, ○ ouvre la page ; sur les feuilles, ○ ouvre la feuille. ✕ ou « 戻る » : le noir en 10 images, le cadre et la fenêtre s'en vont, puis la chambre revient. À la souris, le portage mène le curseur à ce qui est survolé, et un clic vaut ○.

## Les fins vues (`AD_MainmenuEndcheckTran`, état 7 d'`adv_mainmenu`)

L'entrée 6 de la chambre, sans noir. `ending_check_flag_set(n, drapeau)` donne le drapeau spécial de chaque fin (six), `ending_check_bg_set(décor)` l'image de leurs vignettes (`end_gp` : 3 × 2 cases de 0xaa × 0xe0). Sur le fond de la chambre : la bulle « エンディング » (`menu_hukidasi` 1, 0x1a4, 0x17a × 0x5c), le cadre des vignettes (`menu_endifng` 0xe, 0xd, 0x14a × 0xea, en 0x140 − 0xa5, 0x62) et celui des noms (0xb, 0x10f, 0x1ea × 0x65, étiré à 600 de large en 0x14, 0x151) ; pour chaque fin vue sa vignette, réduite à 0x6a × 0x70 (en 0xa0 + 0x6a × colonne, 0x66 + 0x70 × rangée ; les fins 0 à 5 aux cases (1, 0), (2, 1), (0, 1), (0, 0), (2, 0), (1, 1)), et son nom (`0x35d5e0` + 8n), « ??? » (`0x35d610`) sinon, centrés en x = 0xaa + 0x129 × (n / 3), y = 0x165 + 32 × (n % 3) ; enfin « 達成率 » et la part des fins vues (vues × 100 / 6, en chiffres de `menu_endifng`). ○ ou ✕ ramènent à la chambre en 10 images.

## La carte des journées (`map.c`, état 9 d'`adv_main`)

Avant chaque carte, le script de la carte (`start_mapstcm`, script 130) est lancé : il pose les lieux du jour (`map_pos_dataset` : script, point d'entrée, valeur donnée à « switch » ; `map_pos_stateset` : 0 caché, 1 grisé, 2 choisissable ; `map_pos_commentset`) et les têtes des personnages (`map_mark_pos_set(personnage, lieu)`).

| Élément | Image (SYSTEM.UNI) | Place |
|---|---|---|
| fond, qui défile d'un pixel par image (boucle de 639) | `map_back` (32), 512 × 448 | prio 0x5f |
| vignettes des lieux, en couleur (2) ou grisées (1) | `map_prts1` (30), `map_prts2` (31) : 209 × 158 en (0,0), (210,0), (0,159), (210,159) | centres (180,138), (460,138), (180,315), (460,315) ; prio 100 ; le lieu choisi à 1,1 |
| cadres : choisi (rouge), choisissables (bleus) | `map_prts1` : (222,318,217,167), (0,318,216,165) | prio 0x65 ; le choisi à 1,1 |
| têtes des personnages | `map_prts2`, rangée y = 392 | depuis le coin bas droit de la vignette, vers la gauche, à 1,1 |
| bandeau du haut « 移動先選択 » (`0x3644b0`) | fenêtre style 4 | centre (0x140, 0x1e), 0xf0 × 0x22, opacité 0x6e |
| commentaire du lieu | fenêtre style 4 | centre (0x140, 0x1a4), 0x1e5 × 0x22 |

La carte arrive sans noir (`AD_ModeChangeProc3`, cas 4 : `AD_MapWinMake(0x1e)` et `AD_GPTranModeSet(3)`, qui lance `AD_MapStEffectMake` et rend 0x3e) : pendant 0x3e images, la scène reste dessinée et des effets passagers (sorte 0, effacés à leur fin) font paraître la carte par-dessus.

| Effet | Départ | Images-clés |
|---|---|---|
| fond, deux copies (centres 0x140 et 0x140 − 0x280, y 0xe0), prio 0x5f | opacité 0, vitesse 1 vers la droite (mode 0) | 0x14 images à 0, 0x14 vers 0x7f, 0x14 à 0x7f |
| vignette k (lieux 1 ou 2), prio 100 | opacité 0, retard 5k + 0x14 | 10 images vers 0x80 et l'échelle 1,3 ; 10 vers 1 (1,1 pour le lieu choisi) ; 0x14 − 5k vers la couleur 100 (le lieu choisi reste à 1,1) |
| cadre k (lieux 2), prio 0x65 | opacité 0, échelle 1,8 pour le choisi, retard 5k + 0x1e | 10 images vers 0x80 et la couleur 0xff (1,1 pour le choisi) ; 0x14 − 5k vers la couleur 0x80 |

Tout s'efface à la 60ᵉ image ; à la 0x3e, la carte se dessine (fond, cadres, vignettes, têtes) et l'état 9 commence. Le fond de l'effet, parti de 0x140, est alors là où commence le défilement de la carte (0x3c). Une partie chargée sur la carte la retrouve d'emblée (`AD_ModeChangeLoadProc` : `AD_GPTranModeSet2(3)`, sans effet). Les effets en mode 0 avancent à leur vitesse (+0x54) dans leur direction (+0x58, en degrés), freinée chaque image par leur image-clé (+0xc ; la première une fois l'effet tenu), au moment de les dessiner (`AD_EffectTran`).

Les flèches vont de lieu voisin en lieu voisin (`AD_MapPosCursorMoveSet`) jusqu'à un lieu choisissable. ○ choisit : le script et le point d'entrée du lieu deviennent ceux à suivre, « switch » prend la valeur du lieu, le son 2, les personnages s'en vont, et l'effet du lieu choisi (`AD_MapOkEffectMake`, sorte 0) : sa vignette clignote par-dessus elle-même (prio 0x65, échelle 1,1, opacité 0 → 0x50 → 0 → 0x50 → 0 en quatre fois 5 images, couleur 0xff,0xdc,0xdc puis 0xdc) ; trois cadres rouges (`map_prts1` 0xde, 0x13e, 0xd9 × 0xa7, prio 0x66, retards 0, 10, 20) passent de 1,1 et 0x50 à 1,4 et 0 en 0x14 images, en blanchissant (couleur 0xff). 0x1e images plus tard le noir tombe en 10 images. Il reste jusqu'au lot suivant qui a une page, un choix ou la carte (voir « Les passages d'un lot à l'autre »).

## Le résultat de la journée (`AD_Result*`, état 0x0c d'`adv_main`)

Le fond `resu_back` (SYSTEM.UNI 55), puis six cases de 190 × 199 (x = 0x1c + 0xc4 × colonne, y = 0x17 + 0xce × rangée) : la première fille, Sakura (une image seule), puis les quatre autres. Pour chaque fille, `result_data_set(n, drapeau, maximum, niveau 1, niveau 2)` (script 4) donne son niveau d'après la valeur du drapeau (−1 si elle est négative, puis 0, 1 ou 2 aux seuils) et sa jauge (valeur × 100 / maximum, de 0 à 100). Le niveau choisit l'image de la case (`resu_c0` à `resu_c3`, SYSTEM.UNI 50 à 53, quatre quarts chacune) ; aux niveaux 0 et −1 un voile s'y ajoute ; puis la plaque du nom, le fond de la jauge (des cœurs) et son remplissage (`resu_parts`, 54). L'écran arrive après le noir (0xf images, `AD_ModeChangeProc1`), qui s'efface en 10 sur lui ; ○ pose la question « 好感度確認を終了します よろしいですか？ » (`AD_CheckWinMake`, `0x3653c0`, 0xfa × 0x37, いいえ d'abord) ; はい : le noir tombe en 10 images (0xc images d'attente) et reste jusqu'au lot suivant qui a une page.

## La répartition des points (`AD_Distribute*`, état 0x0d d'`adv_main`)

`distribute_mode(n)` (0x1e4) fait du lot une répartition de n points. `AD_ModeChangeProc1` pose le noir (0xf images) et place le curseur sur la première case ; l'écran se dessine 10 images plus tard (`AD_DistributeTran`) : le fond `resu_back`, le cadre du curseur (`resu_parts` 0, 0x108, 190 × 199, prio 0xd4, en x = 0x1c + 0xc4 × colonne, y = 0x16 + 0xce × rangée) et les six cases du résultat, toutes voilées et sans jauge (`AD_ResultColumnTran` au niveau −1). Puis le noir s'efface en 10 images.

Les flèches vont de case en case (après 0x14 images, toutes les 7), en boucle sur trois colonnes et deux rangées ; la case de Sakura (colonne 1, rangée 0) est sautée : vers la colonne 2 depuis la 0, vers la 0 depuis la 2, vers la rangée 1 sinon. ○ choisit la fille (son 2) : sa case clignote (`AD_DistributeColumnEffMake` : son image du niveau 0, prio 0xd4, opacité 100, 0, 100, 0 en quatre fois 5 images) et sa question s'ouvre (`0x365400`, `0x365450`, `0x3654a0`, `0x3654f0`, `0x365540` : « ドクロちゃんに好感度を割り振ります よろしいですか？ »…, 0xd2 × 0x51, いいえ d'abord). いいえ revient au choix ; はい : le noir en 0x14 images, puis `AD_DistributeEnd` ajoute les n points au drapeau de la fille (celui de `result_data_set`) et l'écran devient gris sombre (0x14) sous le noir, jusqu'au lot suivant. Pas d'historique ni de menu pendant la répartition ; l'avance rapide continue. À la souris, le portage choisit la case survolée, et un clic sur une case vaut ○.

## L'écran titre (`adv_title`)

Le décor `title` (SYSTEM.UNI 10) sort du noir en 0x14 images ; 0x28 images plus tard la musique 1 commence. Par-dessus, de l'image `t_rogo` (SYSTEM.UNI 11) : le logo (0, 13, 339 × 246, centré en 320, 160), la mention du bas (3, 280, 378 × 37, en 320, 410) et l'invite « スタートボタンを押してね » (0, 325, 316 × 28, en 320, 340), dont l'opacité monte de 10 par image jusqu'à 0x80 et pulse (× (sin 2t + 1) / 2, t de 0 à 89).

START mène à la chambre (le noir en 10 images) ; le personnage de la chambre est tiré du compte des images passées au titre (`AD_MainmenuCharaSet`). Après 0xe10 images (une minute) sans START, la musique s'arrête, le noir tombe en 0x1e images et la vidéo « op » passe, puis le titre revient. Le titre sait aussi poser une question oui / non (sous-état 10 de l'état 2) que rien n'ouvre.

## Les vidéos (`AD_MVPlay`, `MovieTheater` ; `src/movie.rs`)

Seize fichiers PSS dans `PSS/` : MPEG-2 640 × 448 progressif à 29,97 images par seconde, affiché en 4:3, avec un son PCM 16 bits stéréo à 48 kHz (en-tête `SShd`, blocs de 0x200 octets par voie). Une vidéo a un nom et un fichier (`AD_MVMake`, trente places en `0x1be96c0`) : « op », « if », « df » et « mate » (OP, IF_LOGO, DF, IFMATE) sont inscrites par l'exécutable, les autres par les scripts (`movie_entry`). Le portage les décode avec la bibliothèque `ps2-video` (Rust pur, identique à FFmpeg au bit près sur les seize vidéos).

`AD_MVPlay` note la vidéo vue dans l'album, coupe la voix (`AD_VoiceCancel`) et joue la vidéo jusqu'au bout dans sa propre boucle (`MovieTheater`) : elle seule s'affiche. START l'arrête, passé 30 tours de boucle (le compteur d'appui du bit 11 de la manette, `0x15499a4`, vaut 1). Le portage compte 30 images de jeu ; sur PC, Échap et le clic l'arrêtent aussi. Le son de la vidéo sert d'horloge aux images ; il passe à plein volume (`changeMasterVolume(0x3fff)`), sans les volumes des options.

- Au démarrage (`main`, état 0x1e, après la lecture des données système) : « if », « df », « mate » puis « op » à la suite, chacune arrêtable par START ; puis le titre. L'ouverture est ainsi notée vue dans l'album dès le premier lancement. (En test, `DOKURO_BOOT` les garde ; sinon le banc de test commence au titre.)
- Au titre (état 6) : après une minute sans START, la musique s'arrête et le noir tombe en 0x1e images ; « op » passe ; puis la musique 1 repart et le noir s'efface en 0xf images.
- Dans l'histoire (lot de sorte 0x0e) : `AD_ModeChangeProc1` (cas 0x0e) met le plan 6 au noir ; l'état 8 attend que la musique se soit tue, passe la vidéo, puis le noir s'efface en 10 images, la musique en cours repart du début (`AD_BGMRestart`) et 10 images plus tard le lot suivant commence. Le portage met la musique en pause plutôt que d'attendre qu'elle se taise.
- Dans l'album : ○ sur une vidéo vue arrête la musique (`AD_BGMPause`), assombrit le plan 5 en 0xf images et attend 0x14 images ; la vidéo passe ; la musique repart du début et l'album revient aussitôt (le plan 5 s'éclaircit en 0xf images).

Les vidéos de l'histoire (relevées par le test `audit`) : an16 (cinq fois), an12, an04, an03, an11, an09, an07, an02, an15, an13 ; ed01 passe à la fin de chacune des six fins (scripts 121 à 126), que l'audit ne parcourt pas. Le script 3 inscrit aussi chaque vidéo à un chemin de développement (`host0:pss/…`), dans un point d'entrée que le jeu ne lance pas.

## Les touches (`AD_InputKey`)

Le jeu lit des touches logiques, traduites en boutons de la manette (△ 0, □ 1, ✕ 2, ○ 3, L1 4, R1 5, L2 6, R2 7, START 8) :

| Touche | Bouton | Dans l'histoire (`AD_AdvSubProc`, page qui attend) |
|---|---|---|
| 0x1f | ○ (ou L2 dans l'histoire) | valider, page suivante |
| 0x1e | ✕ | retour |
| 0x20 | ✕ | cacher les fenêtres ; un nouvel appui (ou une autre touche) les rend |
| 0x21 | ○ | (valider, dans les menus et les options) |
| 0x26 | □ (sauf interdiction 4, `restrict_menu`) | le menu du jeu (`AD_AdvMenuOpen`) |
| 0x27 | L1 (sauf interdiction 8, `restrict_txt`) | l'historique (`AD_BackLogModeStart`), s'il y a déjà des pages |
| 0x28 | R1 (○ dans l'histoire) | |
| 0x22 | START (sauf interdiction 2) | la lecture automatique (`AD_ReadModeProc`) |
| 0x23 | △ (sauf interdiction 1) | l'avance rapide |
| 0x24 | ✕, □ ou L1 | arrêter l'avance ou la lecture automatique |
| 0x25 | R1 (sauf interdictions 1 et 2) | passer de l'avance des pages lues à l'avance forcée |

Les interdictions 1 et 2 ne sont jamais posées : les scripts n'ont que `restrict_menu`, `restrict_txt` et `restrict_win` (4, 8, 0x10). Toutes sauf 0x22 à 0x25 sont muettes tant que le compte à rebours `0x1da756a` est positif ; il baisse d'une image chaque fois qu'une touche a été lue ainsi.

Le portage y fait correspondre les usages d'un visual novel sur PC : clic gauche, Entrée ou la molette vers le bas pour ○ ; clic droit pour ✕ (cacher les fenêtres) ; la molette vers le haut pour L1 (l'historique) ; Échap pour □ (le menu) pendant les pages, pour ✕ ailleurs. Dans l'historique, la molette fait défiler et, vers le bas depuis la page la plus récente, le referme. Dans l'histoire, Ctrl tenu avance rapidement (il vaut △ tant qu'il est tenu, et l'avance cesse quand on le lâche) et A vaut START (la lecture automatique) ; ailleurs, A reste L2. Le jeu lui-même suit l'original et se joue à la manette ; le clavier et la souris ont en plus les commandes habituelles d'un visual novel sur PC.

### La souris (`src/pointer.rs`)

L'original n'a pas de souris : le portage la fait passer par les touches du jeu, sans toucher à ses automates. Chaque écran déclare ses zones, des rectangles tirés des mêmes constantes que son dessin (la place d'une entrée, d'une carte, d'un buisson…), chacune avec ce qu'elle désigne ; `pointer::pick` rend la zone survolée et la zone cliquée. Survoler une zone y mène le curseur, comme les flèches ; un clic sur une zone vaut ○ sur ce qu'elle désigne ; un clic à côté ne fait rien. Là où le jeu attend n'importe quelle touche (le titre, une page qui attend, les fins, une vidéo), un clic n'importe où suffit. Les questions (はい / いいえ) sont traitées par `Windows::answer`, pour toutes les fenêtres qui en ouvrent.

Pendant une partie de mini-jeu, la souris ne sert qu'à viser (le buisson du filet, la carte du memory ; Sakura suit sa position dans le jeu des affaires) : le clic y vaut ○ partout, comme la touche, et le jeu décide de ce qu'il fait.

En debug, `DOKURO_ZONES=1` dessine le contour rouge des zones de l'écran : de quoi vérifier qu'elles tombent sur ce qui est dessiné.

## L'historique (`AD_BackLog*`)

À chaque page de texte (`AD_ModeChangeProc2`), le jeu range dans un anneau de 0x96 pages : le nom suivi de `#n` puis le texte affiché (préfixe `#Speed[5]…` compris), la taille de sa fenêtre (0x253 de large ; 0x8c de haut avec un nom, 0x69 sans), la voix, le style de la fenêtre de texte.

Sur une page qui attend, L1 l'ouvre (s'il y a déjà des pages) : les fenêtres de texte se ferment, le plan 4 s'assombrit à 0x40 en 0xf images, et les fenêtres de l'historique (`save%d`, prio 0xe1, cadre transparent, fond à 100, texte à 0x19, 0xc du coin) s'ouvrent en 10 images, centrées en x = 0x15e : la page en cours en y = 0xe0, deux plus anciennes au-dessus et deux plus récentes au-dessous, 10 pixels entre deux. ↑ (ou L1 tenu) remonte, ↓ (ou R1) redescend, la page glissant en 1 à 4 images ; ○ fait réentendre sa voix ; ✕ ou L1 referment ; □ ouvre le menu du jeu (sous-état 4). Tant que la voix d'une page joue, seuls ○, L1 et ✕ répondent, pour l'arrêter.

Deux icônes de `win2` (prio 0xe3), dont les rectangles sont lus par le registre gp (`gp` vaut 0x36d6f0, posé au démarrage : `0x365780` à `0x365798`) : la voix (126, 0, 64 × 53, centre 38, 204), quand la page au centre en a une et ne glisse pas, qui bat pendant qu'elle joue (sa taille × 90 / (100 + 10 sin a), a avançant de 5° par image) ; le retour (126, 53, 64 × 53, centre 38, 254), sur une page plus ancienne que la dernière qui a un point de retour. Leur opacité monte ou descend de 12 par image (0 à 0x7f).

Chaque page de texte devient un point de retour (`AD_BackLogSizeSet` avec 1) : à son attente, ou à celle de son choix, le jeu copie la partie comme pour une sauvegarde (`AD_BackLogSubSaveProc` → `AD_RewindSave`, 0x96 places de 0xe48 octets en `0x1cfd840`). △ (sans compte à rebours) sur une telle page : « この場面へ巻き戻します よろしいですか？ » (`0x365220`, 0xfa × 0x37) sur l'écran assombri (plan 5, 0x40 en 0xf images) ; いいえ revient à l'historique ; はい : le plan 5 s'éclaircit, le noir tombe en 0xf images, l'historique se ferme, puis les bruitages s'arrêtent, « 巻き戻し » (`0x365268`, une fenêtre de 0x96 × 0x32) entre dans l'historique, et la partie de cette page reprend comme une partie chargée, l'historique gardé (`AD_AdvPartStartProc(5)`).

## Le choix (`AD_AdvSelectWinMake`, état 7)

Les réponses (`menutxt_set`, jusqu'à dix) vont dans les fenêtres `select0` à `select9` : style 3 pour la choisie (opacités 100), 4 pour les autres (0x80), prio 0xa0, ouverture en 10 images, texte à 0x14, 7 du coin, sans ombre ; centre x 0x140, y = 0xe8 − 0x1f × n + n² / 2 + (n + 0x2c) × k pour la réponse k de n ; largeur 12,5 × octets + 39,5 (la plus longue réponse), hauteur 0x29. Le curseur `select` (kasol, prio 0xa0) paraît en 15 images en x = 0x11d − 6 × octets, à la hauteur de la réponse choisie. Les touches attendent 0x14 images.

↑ ↓ (répétées après 0x14 images, toutes les 7) changent de réponse (`AD_AdvSelectWinSelChange`) : la nouvelle passe au style 3 (100), l'ancienne au style 4 (0x7f), le son 1, le curseur glisse en 6 images. ○ répond : la voix s'arrête, la réponse va dans le drapeau du choix, le choix entre dans l'historique (`AD_AdvSelectTxtSet` : « → » devant la réponse choisie, « 　 » devant les autres, une fenêtre de style 3 de (octets + 2) × 13,75 + 15 sur 0x1e × n + 0x14), le curseur s'en va en 5 images, le son 2, les fenêtres se ferment en 5. Les réponses déjà choisies (dans cette partie ou une précédente) s'écrivent en palette 2 (`AD_BReadCheck`, voir « Ce qui a déjà été lu »).

## Ce qui s'ouvre pendant une attente (`AD_AdvSubProc`)

Une page qui attend (état 6), un choix dont le curseur ne bouge pas (7) et la carte (9) passent la main à `AD_AdvSubProc` : L1 ouvre l'historique (sous-état 1), ✕ cache les fenêtres (2, s'il y a une page affichée), □ le menu du jeu (4) ; fenêtres cachées, ✕ les rend, L1 et □ font de même. En ouvrant un sous-état, le jeu cache la fenêtre de texte et celle du nom (10 images), les réponses (10), le curseur du choix (15) et les bandeaux de la carte (15) ; le menu du jeu laisse la page et les réponses visibles mais cache le curseur et la carte. Le sous-état 6 rouvre ce qui était caché, puis le jeu revient à ce qu'il attendait. Pendant un sous-état, le jeu ne lit plus les touches de la carte ni du choix (`c27c31`).

## Les passages d'un lot à l'autre (`AD_ModeChangeProc1` à 5, état 2)

Pour chaque lot, dans l'ordre : `AD_ModeChangeProc1` (la place de la page, `AD_AdvBthBeforeProc` ; les fondus que demande la sorte du lot), `AD_ModeChangeProc2` (le texte vidé, la fenêtre du nom, la page est-elle lue), `AD_ModeChangeProc5` (les fondus qu'elle ne demande plus s'effacent ; hors des pages, la fenêtre de texte se ferme), `AD_ModeChangeProc3` (l'arrêt de l'avance ; l'affichage de la sorte : `AD_GPTranModeSet`), puis `AD_ModeChangeProc4` (l'état suivant). Leur durée est 0xf images, 0xb − vitesse en avance rapide (celle de `AD_ModeChangeProc1` d'après la page précédente, la page n'étant vérifiée qu'ensuite) ; l'état 2 attend la plus longue.

Les fondus sont ceux des plans (`AD_ModeChangeStaticProc1` et 2, sept opacités) : rien pour les pages, les choix, la carte et les mini-jeux ; le plan 6 au noir (0x7f) pour le résultat de la journée, la répartition des points, une vidéo et la fin. Un lot vide (sorte 1) ne touche à rien. Ainsi le noir posé à la sortie de la carte ou du résultat reste jusqu'au prochain lot qui a une page, un choix ou la carte ; celui de la sortie d'un mini-jeu s'efface de lui-même (10 images). L'état 2 attend la durée d'`AD_ModeChangeProc5` s'il a fermé ou effacé quelque chose (sinon 0), portée à ce que rend `AD_ModeChangeProc3` : 0x3e pour la carte, 10 pour un mini-jeu, 0 pour le résultat (qui peut donc commencer avant que son noir soit complet). Dans l'histoire, un mini-jeu arrive sans noir : `AD_MiniGameStartStEffectMake` fait venir le fond du choix du personnage (`mini_selbg`, centre 0x140, 0xcc, prio 0xab, sorte 0) de l'échelle 2 et transparent à l'échelle 1 et opaque en 10 images, par-dessus la scène ; à la 10ᵉ, le choix se dessine (`AD_GPTranModeSet(0xd)`). La suite (la musique, le choix, le noir de 0x14 images, le tutoriel, le jeu) est celle du menu des mini-jeux, qui, lui, part du noir (`AD_GPTranModeSet2`, sans effet). Le portage suit tous ces passages.

### Les bruitages en boucle (`AD_LoopSE*`, `loopse_play`, `loopse_stop`)

Deux places (`gpffff8aa8` + 4 × place) : la note de la banque (−1 : aucune) et le retard avant qu'elle parte ou s'arrête. Un lot les demande (`AD_LoopSEPlay`, retard d'au moins 1 ; `AD_LoopSEStop`) ; `AD_LoopSEProc` les lance au bout du retard (`Snd_CommandLOOPMSIN` : la note en boucle dans sa place, ou son arrêt), `AD_LoopSEAtOnceProc` tout de suite quand le texte d'une page paraît. Les options et la carte mémoire ouvertes depuis le menu du jeu les coupent en gardant leur numéro (`AD_LoopSEPause`) et les relancent au retour (`AD_LoopSERePlay`) ; le retour dans la chambre, le chargement d'une partie et le retour à une page de l'historique les coupent (`Snd_CommandLOOPMSIN(3, −1)`). Une partie sauvegardée garde les deux numéros, que le chargement relance à la première image (`AD_AdvLoadProc`). Les scripts n'en jouent que deux, les notes 4 et 5, dans la place 0. Les numéros ne sont remis à zéro qu'au démarrage (`AD_SoundInit`) : bug corrigé, voir le README.

### Ce que le passage réécrit (`AD_AdvBthAfterProc`, +0x3a2 du lot ; `Pass` dans `src/scene.rs`)

Le lot porte une sorte de passage : 0 d'ordinaire, 1 après `ezeffect_scenechange` (avec, en +0x3a4, la durée de son éclair). `AD_AdvBthAfterProc` la remplace d'après la sorte du lot précédent : d'un lot de texte (sortes 0 à 3) vers une autre sorte, 2 ; entre deux sortes autres que le texte et différentes, 3. Puis il réécrit ce que le script a posé :

- le décor : 1, 2 et 3 le changent d'un coup (durée 1), après l'éclair (1), après la durée du passage, `sRam01da756a` (2), ou aussitôt (3) ;
- les éclairs des plans (fondus de sorte 2) n'ont pas lieu pour 2 et 3 ;
- la musique : un lot de vidéo ne fait que la noter (`AD_BGMPlayRecord`) ; sinon son retard devient 0 (3), la durée du passage (2), ou s'allonge de l'éclair (1). `AD_BGMPlay` et `AD_BGMStop` ne font que la demander : `AD_BGMProc` la lance au bout de son retard, et `AD_AllSndAtOnceProc` (quand le texte d'une page paraît) lance tout de suite la musique et les bruitages encore en attente. `Snd_Strm_BgmPlay` elle-même ne fait que déposer une demande : le flux ne part qu'une fois lu sur le disque, et un arrêt avant l'annule. Après une vidéo, `AD_BGMRestart` relance ainsi la musique que le lot suivant arrête onze images plus tard, sans qu'on l'entende ; le portage, dont les musiques partent aussitôt, garde pour cette seule relance un délai fixe de 15 images (`RESTART_LATENCY`) ;
- les bruitages et les sons en boucle : leur retard s'allonge de la durée du passage (2) ou de l'éclair (1) ;
- l'attente du décor (+0x3aa), qu'attendent les apparitions de personnages : 0 (1 et 3) ou la durée du passage (2) ;
- les personnages, pour 1 et 2 (d'après le chiffre des centaines de leur compte, puis des dizaines ; X la durée de l'éclair ou du passage) : une apparition, une disparition ou un effacement dure X images sans retard ; un déplacement se fait d'un coup au bout de X images ; un changement d'image seul attend X images. 3 ne les touche pas.

Avant cela, `AD_AdvBthBeforeProc` met à 0 les attentes des apparitions et des disparitions (+0x3a8, +0x3ac) d'un lot où le script a posé un changement de scène.

Ainsi, un lot de vidéo qui suit une page change son décor d'un coup sous le noir du passage. Le portage ne faisait pas cette réécriture (bug du portage, corrigé) : le décor d'un lot de vidéo en fondu enchaîné de 90 images (101:sure1, la résurrection de Sakura) commençait son fondu avant la vidéo et le reprenait après, et l'ancien décor reparaissait.

## L'avance rapide et la lecture automatique (`AD_ReadMode*`, `src/readmode.rs`)

Le mode (`0x1da7534`) : 0 aucun, 1 avance rapide, 2 avance qui s'arrête, 3 lecture automatique, 4 lecture qui s'arrête, 5 le signe montre l'avance que R1 vient de choisir. À la fin de chaque image d'`adv_main`, `AD_ReadModeProc`, sans sous-état et hors des états 0, 8 (mini-jeu), 0x0c (résultat) et 0x16 (fin) :

- △ : de 0, 2, 4 ou 5 à 1 ; de 1 à 2. Sinon ✕, □ ou L1 (touche 0x24) : de 1 à 2.
- START : de 0, 2, 4 ou 5 à 3 ; de 3 à 4, et sur une page qui attend (état 6), ○ attend 10 images (`0x1da756a`). Sinon 0x24 : de 3 à 4.
- R1 : l'avance passe des pages lues (既読, réglage 0) à la forcée (強制, 1) ou l'inverse ; depuis 0, 4 ou 5, le mode devient 5 et le signe est opaque (0x7f).
- L'opacité du signe (`0x1da7536`) monte de 0x14 par image en modes 1 et 3 (jusqu'à 0x7f), sinon descend de 0x14 (de 10 en mode 5) ; à 0, le mode revient à 0. Ouvrir un sous-état la met à 0 (`AD_ReadModeTranCut`).

`AD_ReadModeCheck` rend le mode, sauf en avance des pages lues, où il rend 1 si la page est déjà lue (`0x1da73a0`) et 0 sinon : une page pas lue se lit normalement. Les vitesses viennent des options (`AD_OptionSkipSpeedGet` : celle de l'avance en cours, ou celle de la lecture automatique, de 0 à 10). Quand le mode vaut 1 :

| Moment | Effet |
|---|---|
| Passages d'un lot à l'autre (état 2) | 0xb − vitesse images au lieu de 0xf |
| Transitions du lot (état 3) | vitesse + 5 images par image (aussi après ○, avec la même vitesse) |
| Fermeture de la fenêtre du nom sans nom (`AD_ModeChangeProc2`) | 0xc − vitesse images au lieu de 10 (d'après la page précédente : la page n'est vérifiée qu'ensuite) |
| Ouverture de la fenêtre du nom (état 3) | (0xc − vitesse) / 2 images au lieu de 10 |
| Voix | pas jouée |
| Texte (état 4) | d'un coup ; ○ n'accélère le texte que hors de tout mode |
| Attente de la page (état 5) | (0xb − vitesse) / 2 images, puis la page suivante sans ○ |

En mode 1, même sur une page pas lue, le texte paraît plus vite (`AD_ReadTxtSpeedProc` : la vitesse de la page fois (10 − vitesse de l'avance) / 10). En mode 3, la page attend (10 − vitesse) × 5 + 10 images, plus (12 − vitesse) par glyphe de la fenêtre (`AD_AutoTimeGet`, `AD_WinTxtNumGet`), puis la fin de sa voix ; L2 la passe tout de suite. Ces attentes sont le compte à rebours `0x1da756a` : ○ et les touches d'`AD_AdvSubProc` (L1, ✕, □) restent muettes jusqu'à sa fin.

`AD_ReadModeStop(cas)` arrête l'avance (mode 1) ou la lecture automatique (tout autre mode) si le bit du cas est mis dans leurs options (`AD_OptionSkipStopCheck`) : à un lot de texte pas lu (cas 1 ; l'avance forcée n'y regarde pas), à un choix (0), au résultat de la journée (2, « 好感度ポイント »), à une vidéo (3) ; à la fin du jeu, au début d'une partie et au chargement, toujours (−1).

Le signe (`AD_ReadModeTran`, prio 0xa0, centre 0x221, 0x28) : « 既読スキップ » (`win2` 4, 0x32, 0x77 × 0x15), « 強制スキップ » (4, 0x1a) ou « オート » (4, 0), d'opacité celle du mode × sin(t), t avançant de 2° par image (de 1° en lecture automatique) sur 180°. Huit flèches (`win2` 4, 0x4c, 0x17 × 0x1f, prio 0x9f) tournent autour, à 10 + r pixels, r rejoignant 4 × vitesse d'un pixel par image ; la flèche k (0 à 7) est à l'angle a + 20k, à l'échelle 1 − (8 − k) / 10 et à l'opacité × (0,6 − (8 − k) / 20) ; a avance de vitesse + 1 degrés par image (vitesse / 2 + 1 en lecture automatique). Pour la lecture automatique, l'argument d'`AD_OptionSkipSpeedGet` est perdu par le décompilateur : le portage prend la vitesse de la lecture automatique.

## Ce qui a déjà été lu (`AD_BRead*`, `src/bread.rs`)

À chaque passage du script (`AD_AdvBthFormat`), le jeu note sa place : le numéro du script (`0x1da731c`), le nombre qui suit les quatre premiers caractères du nom du point d'entrée (`strtol`, `0x1da731e` ; le nom est copié par-dessus le précédent, dont la fin reste derrière un nom plus court), la valeur de « switch » (`0x1da7320`), un compte des pages à −1 (`0x1da7322`), pas de route (`0x1da7324` à −1). À chaque lot (`AD_AdvBthBeforeProc`), `route_set(n)` commence la route n (le compte des pages avance, le compte dans la route repart de −1), `route_end` la termine ; puis le compte des pages avance hors route, le compte dans la route (`0x1da7326`) en route.

Les fiches (3000 au plus, 8 octets chacune à `0x1cf04c0` : la suivante, le script, un mot) forment une liste rangée par script. Le mot : ((((sorte × 64 + numéro) × 64 + switch) × 64 + page) × 16 + route) × 256 + valeur, les octets signés ajoutés tels quels (sans masque : un champ qui déborde déborde sur le suivant). La sorte 0 (une page hors route) cherche la fiche par sorte, numéro et switch (le mot décalé de 18 bits), et y garde la plus loin des pages lues ; la sorte 1 (une page en route) cherche aussi par page et route (décalage de 8) et garde dans la valeur le compte dans la route le plus loin lu ; la sorte 2 (un choix) cherche comme la 1 avec la route à 0 et garde un bit par réponse choisie.

- `AD_BReadCheck` (au changement de lot, pour les pages et les choix ; pour chaque réponse d'un choix) : lue si la page est avant (ou à) la plus loin lue, ou si le bit de la réponse est mis. Une liste pleine rend une erreur, que le jeu prend pour « lue ».
- `AD_BReadWrite` : une page à l'attente (état 5, avant ○), une réponse quand elle est choisie.
- `AD_BReadSearch` crée la fiche qui manque, à la fin de celles de son script ; insérée devant les fiches d'un autre script, elle prend la place de la première d'entre elles, déplacée au bout du tableau. Le jeu garde une fiche « en main » par sorte (`0x1cf6280`) : la recherche ne relit pas la liste tant que la clé ne change pas. Une fiche neuve y vaut « rien lu » (page −1, valeur −1 ou 0), mais son mot dit « page 0 lue » ; c'est ce mot que relit le lancement suivant si la fiche n'a pas été écrite entre-temps (une page de choix au début d'un passage, sauvegardée pendant le choix), sans effet visible puisque la page de choix n'est pas vérifiée à l'arrêt de l'avance.
- Le numéro de la page tient sur 6 bits : au-delà de 63 pages dans un même passage du script, il déborde sur la valeur de « switch » et la fiche n'est plus retrouvée. Pleine (3000 fiches), la liste ne grandit plus et tout passe pour lu.

Le portage garde les fiches dans l'ordre de la liste (`read` dans la sauvegarde : le script sur un octet, le mot sur quatre, en hexadécimal) et reproduit la fiche en main.

Le jeu fige la partie à sauvegarder à l'entrée de chaque attente (`AD_BackLogSubSaveProc`, `AD_AdvSaveProc`) ; le portage la fige à l'ouverture du menu, ce qui revient au même puisque rien ne change pendant l'attente. Une partie sauvegardée garde ce qu'elle attendait : la reprise rouvre la page, ou la page et le choix (curseur sur la première réponse), ou la carte (son script, `AD_MapStart`, `AD_MapWinMake(0x1e)`).
## Le menu du jeu (`AD_AdvMenuOpen`, `AD_AdvMenuProc`, sous-état 4 d'`AD_AdvSubProc`)

□ sur une page qui attend : la voix s'arrête, le plan 4 s'assombrit (0x40 en 0xf images), la fenêtre « menu » s'ouvre en 10 images (style 1, prio 0xdc, centre 0x140, 0xdc, 0xb4 × 0xaa, opacités 0x80, texte à 0xc, 0xc sans ombre ; ces deux arguments d'`AD_WinMake`, perdus par la décompilation, sont lus avec `Tools/mips_calls.py`). Son texte, à `0x363fc0` : « セーブ / ロード / オプション / 桜くんの部屋 / 戻る ». Le curseur (`kasol`) paraît en 0xf images en x = 0xd0, y = 0x9d + 0x1e × k, et glisse en 6 images ; ↑ ↓ choisissent (en boucle sur cinq), ○ valide, □ ou ✕ referment (l'écran s'éclaircit en 0xf images).

| Entrée | Effet |
|---|---|
| セーブ | l'écran de sauvegarde (sous-état 7, voir plus bas) |
| ロード | l'écran de chargement (sous-état 8) |
| オプション | les options (sous-état 5, voir plus bas) |
| 桜くんの部屋 | la question `0x365280` (0xfa × 0x37, plan 5 assombri à 0x40) ; はい : le noir, puis la chambre |
| 戻る | le menu se ferme |

## Le tremblement de la caméra et la vibration (`src/shake.rs`, `src/rumble.rs`)

`target_shake_set(caméra, étape, sorte, niveau)` donne un tremblement à une étape de la caméra. La caméra garde une sorte et un niveau en cours (`0x1be9284`, `0x1be9285`) ; pendant une étape qui en donne d'autres, le tremblement passe de l'ancien au nouveau (compte / durée), et le nouveau s'installe à la fin de l'étape (`AD_CameraProc`). Un compte avance d'une image (`0x1be926e`, modulo 4000).

La sorte se lit axes × 4 + période : axes 0 X, 1 Y, 2 Z (le zoom), 3 XY, 4 XZ, 5 YZ, 6 XYZ ; période 1 en 40 images, 2 en 20, 3 en 12, 4 au hasard ; 0 immobile. Avec `c` le compte, `P` la période et `n` le niveau + 1 :

| Axe | Valeur | Au hasard |
|---|---|---|
| X (`AD_BGShakeX`) | n × (\|c % P − P/2\| / (P/10) − 3), de −3n à 2n | n × (rand % 5 − 2) |
| Y (`AD_BGShakeY`) | de même, le compte décalé de 5, 4, 3 (P = 40, 20, 12) | de même |
| Z (`AD_BGShakeZ`) | n × (\|(c + d) % P − P/2\| / (P/2) × 2 − 1), en centièmes du zoom ; XZ décalé (d = 5, 4, 3), XYZ en 20, 12 et 8 images | n × ((rand % 9) / 3 − 1) |

Pendant un passage, la nouvelle valeur au hasard de X et Y vaut (n × (rand % 6)) − 3, celle de Z (rand % 9) / 6 − 1 ; pour la sorte 0x1b (XYZ, 8 images), le jeu oublie le facteur du passage sur Z (bug corrigé par le portage, voir le README). Le dessin ajoute x et y (entiers) à la caméra, z / 100 au zoom.

La vibration suit : à chaque image, une sorte 0 donne la force 0 (`AD_PadVib(0)`), sinon, avec sx, sy entiers et sz avant la division par 100, (|sx| + |sy| + |sz × 2,5|) × 2 + 1 si un seul axe bouge, (|sx| + |sy| + |sz × 2|) × 3 / 2 sinon. `AD_InputProc` l'envoie à l'image suivante (`PadVibration(0, 2, −1, force)` : le petit moteur au-delà de 4, le gros à 3 × force, au plus 255), l'arrête quand la force retombe à 0, et la coupe une image après 0x708 images (30 s) de vibration continue. L'option « 振動 » coupe toute force positive ; l'écran des options verrouille la vibration (`AD_PadVibLock`) et la fait essayer (`PadVibration(0, 2, 4, 0x96)`) 7 images après un changement et en quittant l'entrée. La durée 4 de l'essai est comptée en images (la routine de la manette, qui la décompte, n'est pas dans les fonctions nommées).

Le portage envoie le gros moteur au moteur basse fréquence de la manette (force × 257), le petit au moteur haute fréquence (en marche : au maximum) : XInput sous Windows, un effet FF_RUMBLE du périphérique evdev de la manette sous Linux. C'est une conversion directe, sans courbe ni atténuation pour imiter la DualShock 2 : 255, le maximum du jeu, devient 0xffff, le maximum que l'API permet de demander. Ce qu'en fait la manette dépend d'elle et de son pilote (une manette XInput standard tourne alors à fond ; Steam Input ou DS4Windows appliquent leurs propres réglages), et ses moteurs ne réagissent pas comme ceux de la DualShock 2 : les faibles forces (niveaux 3 et 5, 20 à 30 %) peuvent y être plus discrètes.
## La sauvegarde et le chargement (`save.c`, `mcs.c` ; `src/savedata.rs`, `src/mcs.rs`)

### Ce qu'une partie sauvegardée contient

`AD_AdvSaveProc` remplit 0xe44 octets en `0x1cf65b0` ; les 0x118 premiers servent d'en-tête au fichier de la carte. Ce qui compte pour reprendre :

| Donnée | Où | Dans le portage |
|---|---|---|
| le titre : `save_name(texte)` l'écrit en « %s#n » (`0x1da7334`), « なし » au départ (`AD_GameInit`) | en-tête, octet 0 | `title` |
| l'heure de la sauvegarde (`mcsGetTime`, en BCD) | en-tête, 0x110 ; « vraie donnée » en 0x108 | `time` |
| le point d'entrée joué, les drapeaux de l'histoire et `switch` à son départ (`AD_AdvSaveEachProc`, à chaque passage du script) | `0x1da7390`, `0x1cf67d2` | `entry`, `flags_at_entry` |
| l'état d'`adv_main` (dont le lot de la page, `0x1da731a`) | `0x1cf66c8` | `batch` |
| les drapeaux de l'histoire à la page | `0x1cf677e` | `flags` |
| la musique, les deux bruitages en boucle, le nom affiché | `0x1cf6758`… | `music`, `speaker` |
| le décor, la caméra, les personnages (table `0x1c24c00`), les sept plans de fondu, les images posées | | `scene` (plans 0 à 3) |
| la carte (son script, ses lieux), le combat | `0x1cf72fa`, `0x1cf73f0` | `map_script` |
| les données système (réglages, drapeaux spéciaux, textes lus, mini-jeux) | `0x1cf7518` | gardées à part (`src/record.rs`) |

### Reprendre (`AD_AdvLoadProc`, sous-état 9 d'`AD_AdvSubProc`)

`AD_AdvPartStartProc(4)` remet le jeu à zéro et met `adv_main` dans l'état 0x15, sous-état 9 : le noir d'un coup, les personnages effacés ; les drapeaux et `switch` du départ du point d'entrée ; le point d'entrée est **rejoué** (`AD_ScrirptProc`), ce qui refait tous ses lots ; puis `AD_AdvLoadAfterProc` : les drapeaux de la page, le lot de la page, son texte d'un coup (`AD_WinTxtSpeedChange(0)`) dans `mainwin` qui s'ouvre, le nom (8 images), la page seule dans l'historique (avec sa voix), le décor, la caméra ; la musique s'arrête et la musique gardée repart ; enfin le sous-état 6 (les fenêtres reviennent, 15 images) et le noir s'efface en 10 images. Aucun script n'appelle `rand` : le rejeu redonne les mêmes lots.

### Les entrées

| Depuis | Avant l'écran | Après |
|---|---|---|
| menu du jeu, セーブ (sous-état 7) | le menu se ferme (5 images), le noir (plan 6) en 10, les fenêtres se cachent ; 11 images plus tard le noir s'efface d'un coup sous celui de l'écran | la page : l'écran sans noir, le plan 4 s'éclaircit en 15 images, les fenêtres reviennent (texte d'un coup), 15 images avant ○ |
| menu du jeu, ロード (sous-état 8) | de même | chargé : la reprise ; abandon : la page, comme ci-dessus |
| chambre, つづきから (état 2) | le noir (entrées 0 à 3 de la chambre) | chargé : la reprise ; abandon : la chambre revient depuis le noir |

### L'écran (`MemorycardPart`, `mcsMain`)

`MemorycardPart` enchaîne `mcsInit`, `mcsLoadObject` (la planche TIM2 MEMORY.UNI 10, `Tools/tim2_png.py`), `mcsFadeIn` (un noir de 0x80 à 0, 8 par image), `mcsMain`, `mcsFadeOut`, `mcsExit` ; il rend 1 (fait) ou −1 (abandon). Le fond est `save_bg` (SYSTEM.UNI 14), sur tout l'écran ; tout est à la priorité 0xf5.

Les messages viennent de la table `0x1dfbe0` (`Tools/mcs_messages.py`) : leur texte, leur sorte (1 : question はい / いいえ, qui part toujours sur いいえ ; 2 : attendre ○ ; 3 : une opération en cours), et pour ○ et ✕ un code et l'état suivant de l'automate `mcsm*`. Le chemin d'une carte présente et initialisée :

1. `mcsmCheckMc` (« メモリーカード（ＰＳ２）をチェックしています », 0x17). Sans données du jeu : en sauvegarde, la question 0x35 (はい : les données sont créées avec douze fichiers vides, 0x19, puis 0x28 attend ○ ; いいえ : 0x45 « セーブをしないで戻ります。よろしいですか？ ») ; en chargement, 0x33 (はい : sortie ; いいえ : la carte est regardée de nouveau).
2. `mcsmLoadHeaderList` : les en-têtes un à un (0x17), puis le choix (`SetDefaultCursor` : le fichier 1).
3. `mcsmSelectFile` : 0x36 « セーブするファイルを選択してください。 » ou 0x37. ↑ ↓ d'un fichier (1 ↔ 12), ← → d'une colonne (± 6) ; `mcsGetKey` répète après 0x14 images, puis toutes les 8. ○ : 0x3d (nouvelle), 0x3e (écraser), 0x3f (charger ; un fichier vide ne répond pas en chargement). ✕ : 0x39 ou 0x3a (quitter).
4. Sauvegarde : 0x1a pendant l'écriture, puis 0x29 attend ○ et l'écran se ferme. Chargement : 0x1c, puis l'écran se ferme.

Les opérations de la carte durent dans le jeu le temps de la carte ; le portage, qui les fait aussitôt, garde leur message 0x1e images.

La mise en page (`mcsDraw`, sprites de la planche : source → place) :

| Sprite | Source | Place |
|---|---|---|
| cadre du message | 0xe, 400, 0x1e7 × 0x6b | 18, 325 |
| cadre de l'en-tête | 9, 0xa5, 0x11c × 0x42 | 23, 249 |
| « SAVE » / « LOAD » | 0x16a, 6, 0x5f × 0x20 / 0x16a, 0x33, 0x65 × 0x1f | 500, 12 |
| cadre d'un fichier k (0 à 11) | 8, 8 (en chargement, vide : 8, 0x30), 0x120 × 0x1e | 0x14 + 0x131 × (k / 6), 0x32 + 0x21 × (k % 6) |
| numéro k | 0x16a + 0x2d × (k / 6), 0xd0 + 0x1f × (k % 6), 0x2d × 0x1f | 0x19 + 0x130 × (k / 6), 0x32 + 0x21 × (k % 6) |
| curseur (crochets) | 0xb, 0x124, 300 × 0x27 | 0xf + 0x131 × (k / 6), 0x2d + 0x21 × (k % 6) |
| ○ 決定, ✕ 戻る | cadres 0xac, 0xf3, 0x76 × 0x1c ; 0x169, 100 / 0xac, 0x5d × 0x1f | 340 et 352, 270 ; 487 et 499, 270 |
| はい, いいえ | cadres comme ci-dessus ; 0xc, 0x161, 0x35 × 0x1a / 0x54, 0x161, 0x4b × 0x1c ; crochets 0xb, 0x124 et 0x12a, 0x123, 0xf × 0x2a | cadres 512, 335 et 375 ; textes 545 / 535 ; crochets 505 et 625, y 330 ou 368 |

Les textes (`MkWindowSet`, « #Scale[0.76]#Speed[0]#Color[0] » devant) : le message en 37, 330 ; l'en-tête en 39, 257 (« 保存日時　%d/%02d/%02d » puis « #n　　   　　　%d:%02d ») ; le titre du fichier k en 0x3c + 0x131 × (k / 6), 0x36 + 0x21 × (k % 6) (« NON DATA » s'il est vide). L'écran n'a pas de sons.
## Les options (`AD_Option*`, `src/options.rs`)

### Les réglages

Quinze octets en `0x1c28360` ; `AD_OptionInit` donne leurs valeurs de départ, gardées par le portage dans sa sauvegarde (clé `options`).

| Octet | Réglage | Plage | Départ |
|---|---|---|---|
| 0x60, 0x61, 0x62 | volumes : musique, bruitages, voix | 0 à 0x7f | 0x3c, 100, 0x78 |
| 0x63 | mode d'avance rapide (0 : les pages lues ; 1 : tout) | 0, 1 | 0 |
| 0x64 | couper les effets (`AD_OptionEffCutGet`) | 0, 1 | 0 |
| 0x65 | vibration | 0, 1 | 1 |
| 0x66 | vitesse du texte | 0 à 10 | 5 |
| 0x67, 0x68 | opacités du cadre et du fond de la fenêtre de texte | 0 à 0x7f | 100, 100 |
| 0x69 et 0x6a, 0x6b et 0x6c, 0x6d et 0x6e | les trois avances (forcée, des pages lues, automatique) : vitesse (0 à 10) et conditions d'arrêt (bits 1 un choix, 2 une page pas lue, 4 un point d'affection) | | 5 et 0xf, 5 et 0xf, 5 et 0xd |

Où ils servent :

- Les volumes (`AD_OptionSndVolProc`, dès le titre) : la musique (`Snd_Strm_BgmVolumeSet`), les voix et bruitages en flux (`Snd_Strm_SeVolumeSet`), les bruitages de la banque (`Snd_SetMsinVolume`). Linéaires : `v × 0x3fff / 0x7f` (Snd_Strm_VolSetCh). La musique joue donc à 0x3c / 0x7f de son volume tant qu'on n'y touche pas.
- La vitesse du texte (`AD_ReadTxtSpeedProc`) : `(10 − vitesse) / 15` image entre deux lettres, fois le `#Speed[5]` de chaque page ; en avance rapide, fois encore `(10 − vitesse de l'avance) / 10`.
- Les opacités (`AD_OptionTxtWinAlphaGet`) : à l'ouverture de `mainwin` et de `speaker` (`AD_AdvMainWinMake`, `AD_AdvMainWinOpen`, `AD_AdvSpeakerChange`).
- L'avance rapide (`AD_OptionSkipSpeedGet`, `AD_OptionSkipStopCheck`) : la vitesse et les arrêts de l'avance forcée si le mode vaut 1, de celle des pages lues sinon ; l'avance forcée ne regarde pas le bit 2.
- « Couper les effets » n'a pas d'entrée à l'écran : ↑ ↓ sautent l'entrée 1 de la fenêtre de jeu, et son texte (« 愛キャッチ… ») n'est pas affiché.

### D'où on y vient

| Depuis | Avant | Après |
|---|---|---|
| La chambre, entrée 5 (`adv_mainmenu`, état 8) | le curseur et la bulle du personnage s'en vont ; l'écran répond après 10 images | 10 images ; la musique s'arrête (`AD_BGMPause`). Rien de changé : volumes, 10 images, la musique repart. Un réglage changé : le noir (plan 6, 10 images), la sauvegarde système (`AD_AdvSystemSaveProc`, écran de la carte mémoire), la musique repart, le noir s'efface. Puis l'état 0, sous-état 0x14 : la bulle revient, le curseur sur l'entrée 0. |
| Le menu du jeu, オプション (`AD_AdvSubProc`, sous-état 5) | la fenêtre du menu se ferme (5 images), le plan 4 s'assombrit à 0x7f (15 images), la fenêtre de texte se cache, la voix s'arrête ; l'écran répond après 15 images | 10 images, la musique s'arrête ; un réglage changé : le noir et la sauvegarde système, puis le noir s'efface. Volumes, 10 images, la musique repart, et le menu du jeu se rouvre (sous-état 4 : plan 4 à 0x40, fenêtre de texte, curseur sur セーブ). |

L'écran titre a aussi un état d'options (`adv_title`, état 4), que rien n'atteint.

### L'automate (`AD_OptionProc`)

Trois niveaux (`0x1c283a0`) ; une entrée choisie (`0x1c283ac`), une valeur en cours (`0x1c283a8`), un sous-état (`0x1c283a4`), et ces quatre valeurs telles que les a vues `AD_OptionCursorProc`, qui ne tourne que si l'une a changé. Les touches : ↑ ↓ (`AD_InputDirect` : à l'appui, puis après 8 images toutes les 5) ; ← → de même, ou toutes les images pour les volumes et les opacités ; ○ et ✕ sont les touches logiques 0x21 et 0x20, verrouillées par le compte à rebours `0x1da756a` (mis à 0xf après chaque changement d'écran). Sons : 1 au déplacement, 2 à la validation, 0 au retour.

**Niveau 0, le menu des options**, quatre entrées en boucle :

| Entrée | ○ |
|---|---|
| 0 | la fenêtre de jeu (niveau 1) |
| 1 | le son (niveau 2) |
| 2 | la question `0x364500` (0xd2 × 0x37) ; はい : les réglages de départ, la musique s'arrête, volumes, la musique repart |
| 3 | sortir : ← → entre 決定 (0, garder) et キャンセル (1 : les réglages d'avant l'ouverture, `0x1c28380`, reviennent) |

✕ met le curseur sur キャンセル.

**Niveau 1, la fenêtre de jeu**, sept entrées en boucle (la 1 sautée) :

| Entrée | Sous-état | Commande |
|---|---|---|
| 0 スキップモード | 10 (curseur `op_skipm`) | ← → entre 既読 et 強制 ; ○ ou ✕ revient |
| 2 振動 | 0x1e (`op_vib`) | ← → bascule (le jeu fait vibrer la manette 7 images plus tard) |
| 3 テキストウィンドウ | 0x28 (`op_win`) | ↑ ↓ entre 枠 et 背景, ← → de 0 à 0x7f |
| 4 文字速度 | — | ← → de 0 à 10 sur l'entrée ; ○ n'y fait rien |
| 5 スキップ・オートプレイ設定 | 0x32 (`op_skip`) | ↑ ↓ entre les trois avances et 戻る ; ○ ouvre une avance (sous-état 0x3c, 0x46, 0x50 ; curseur `op_skip2`) : ↑ ↓ entre vitesse et cases, ← → règlent la vitesse ou choisissent la case, ○ coche |
| 6 戻る | — | retour au niveau 0 (✕ aussi) |

Les curseurs du mode et de la vibration restent, estompés (opacité 0x3c, `Cursor_Hidden`), sur la valeur réglée. En quittant la fenêtre de l'avance rapide, le jeu met l'entrée 4 puis la ramène dans les quatre entrées de la fenêtre qu'il quitte : le curseur revient sur l'entrée 0, avec le son du déplacement après celui du retour (bug corrigé par le portage, qui le laisse sur l'entrée 4, voir le README).

**Niveau 2, le son**, quatre entrées en boucle : BGM, SE, VOICE (← → de 0 à 0x7f ; les bruitages prennent leur volume et le font entendre tous les 4 pas), 戻る. La sortie arrête la musique, applique les volumes, attend 10 images et la relance ; le curseur revient sur « サウンド設定 ».

### La mise en page

Les fenêtres (`AD_WinMake` puis `AD_WinStateSet(…, 2, 1, 8, −1, x, y)` : ouverture en fondu, sans ombre, texte en x, y du coin ; ouverture en 10 images, opacités 0x80 ; fermeture en 10 images) :

| Fenêtre | Style | Priorité | Centre | Taille | Texte | Phrases |
|---|---|---|---|---|---|---|
| `option` | 1 | 0xe6 | 0xb9, 0x7d | 0x10e × 200 | 0x23, 0xf | `0x364d80`, `d90`, `da0`, `dc0` |
| `op_info` | 2 | 0xe6 | 0x140, 400 | 600 × 0x23 | 0x12, 5 | l'explication de l'entrée |
| `op_game` | 1 | 0xe8 | 0x140, 200 | 0x1ae × 0x145 | 0x23, 0xf | `0x364c50` à `0x364d60` |
| `op_skip` | 1 | 0xea | 0x186, 0xd2 | 0x1c2 × 0x140 | 0x28, 0xf | `AD_OptionSkipTxtSet`, à l'échelle 0,9 |
| `op_sound` | 1 | 0xe8 | 0x14a, 0x78 | 0x172 × 0x96 | 0x23, 0xf | `0x364aa0` à `0x364b00` |

Les cases de l'avance rapide sont écrites `#Color[0]` (cochées) ou `#Color[6]` (grises). Les phrases s'impriment avec `Tools/exe_text.py`.

Les curseurs (`kasol`, `Cursor_Make` en 15 images ; `Cursor_Move` en 5) :

| Curseur | Priorité | Places |
|---|---|---|
| `op_main` | 0xe7 | x 0x46 ; y 0x34, 0x52, 0x99, 0xc3 (キャンセル : x 0xa8) |
| `op_game` | 0xe9 | x 0x7d ; y 0x43, 0x66, 0x89, 0xac, 0x10b, 0x12e, 0x150 |
| `op_skipm` | 0xe9 | x 0x164 + 0x5a × valeur, y 0x41 |
| `op_vib` | 0xe9 | x 0x159 + 0x46 × valeur (0 : ON), y 0x88 |
| `op_win` | 0xe9 | x 0x96, y 200 + 0x1e × entrée |
| `op_skip` | 0xeb | x 0xbc, y 0x4c + 0x5a × entrée |
| `op_skip2` | 0xec | vitesse : 0xd4, 0x68 + 0x5a × k ; cases : x 0x124, 0x177, 0x1ca (avance forcée : 0x124 + 0x53 × valeur), y 0x81 + 0x5a × k |
| `op_sound` | 0xe9 | x 0xa5 ; y 0x46 + 0x1e × entrée, 0xaa pour 戻る |

Les jauges (`AD_OptionTran`, `AD_AdvWinBar2`), dessinées selon le mode de `AD_GPTranSubModeSet` (2 la fenêtre de jeu, 3 l'avance rapide, 4 le son ; un mode plus haut attend 10 images, le temps que sa fenêtre s'ouvre) : le cadre `kasol` (4, 0x22, 0xe2 × 0x10) à x, y, w × h, puis la barre (9, 0x37, 0xd8 × 6) en x + 4, y + 5, `(w − 10) × pourcentage / 100` × (h − 11), à la priorité de la fenêtre (+1 pour la barre).

| Jauges | x | y | w × h | Valeur |
|---|---|---|---|---|
| son | 300 | 0x3d, 0x5b, 0x79 | 0xa5 × 0x16 | volume × 100 / 0x7f |
| avance rapide | 0x170 | 0x5c, 0xb6, 0x110 | 0xb4 × 0x14 | vitesse × 10 |
| fenêtre de jeu | 0x136 | 0xbb, 0xd9 (opacités), 0xfd (vitesse) | 0xa4 × 0x18 | opacité × 100 / 0x7f, vitesse × 10 |

Quelques arguments manquent au pseudo-C (passés sur la pile ou laissés dans un registre) : la durée de `Cursor_Make` (15, lue avec `Tools/mips_calls.py`), celle de `Cursor_Hidden(op_main, 0x3c)` et le dernier argument de certains `AD_InputDirect` (le portage prend 10 images et « à l'appui »).

## Les mini-jeux abandonnés (`decompiled/battle.c`, `decompiled/tenkey.c`)

Deux jeux sont restés dans l'exécutable sans que rien ne les lance (les sortes de lot 5 et 6). Il n'en reste que le code : aucune de leurs images n'est dans les paquets du disque (tous parcourus avec `Tools/uni_list.py`), et le portage ne les reprend pas.

### Le combat (`AD_Dcb*`, sorte 6)

- `AD_DcbStart` montre ses images, déjà inscrites sous leur nom : `dcb` (la fenêtre), `dcbhp`, `dcbstart`, `dcbcuteff`, `dcbcuteff2`, `dcbcuteffbg`, `dcbgun`, `dcbcutmae`, `dcbkurai`, `dcbgauge_bg` ; ailleurs servent `dcbicon`, `dcbicon2`, `dcbeff`, `dcb_win`, `dcb_win2`, `dcb_lose`, `dcb_lose2`. Une image s'inscrit d'ordinaire par une instruction de script, qui donne son paquet et son numéro : aucun script ne le fait pour celles-ci.
- Le choix (`AD_DcbSelctProc`, `AD_DcbSelctTran`) : ← → parmi les personnages (leur nombre en `0x1ba9324`, leurs numéros en `0x1ba9320`), ○ valide (`AD_DcbSelctEndProc`). Chaque personnage a une fiche de 0x19c octets en `0x1bae5a0` : ses points de vie (le premier mot, réduits d'un dixième par niveau, `0x1ba9351`), trois boutons d'action et un spécial (`AD_DcbButtonSet`, `AD_DcbSpButtonSet`), sa planche et sa place pour le choix. Aucun code connu ne remplit ces fiches.
- Le combat se joue sur le décor de l'histoire, avec ses personnages (`AD_TrChara*` : expressions, poses, couleur) ; la caméra vise des points du décor (`AD_DcbCameraProc`, `AD_BGTargetSet`).
- Le cœur (`AD_DcbCircleProc`, `AD_DcbTargetProc`, `AD_DcbHitJudgProc`) : un repère tourne sur un cercle (angles de 0 à 0x168, soit 360°) où sont posées jusqu'à 1000 cibles (fiches de 0x14 octets en `0x1ba94e0` : son cercle en +2, son angle en +4, son état en +0xe ; l'octet +0 est comparé à `0x1ba9325`, probablement le bouton attendu). `AD_DcbHitJudgProc` regarde les cibles entre l'angle précédent du repère et le nouveau : une cible ainsi passée est notée, et la bonne réponse la touche (le bruitage 0xb), la verrouille (`AD_DcbLockMoveProc`) et rapporte des points (`AD_DcbPointGet`, `AD_DcbHitMake`).
- Autour : les points de vie (`AD_DcbHpProc`, `AD_DcbHpTran`), la jauge (`AD_DcbGaugeTran`), des cut-ins plein écran (`AD_DcbCutinPlay`), des renforts (`AD_DcbSup*`), des tours (`AD_DcbTurn*`), un compte à rebours (`AD_DcbCountProc`), les fins (`AD_DcbGdEndEffMake`, `AD_DcbBdEndEffMake`).
- `AD_DcbEnd` met un drapeau du script (`0x1ba932a`) à 1 (gagné) ou 0 (perdu), que l'histoire aurait lu.

### Le pavé numérique (`AD_TenKey*`, sorte 5)

- Les instructions de script `tenkey_mode` (0x1a4), `tenkey_ok_text_set` (0x1a5) et `tenkey_ng_text_set` (0x1a6) le règlent et donnent ses textes de bonne et de mauvaise réponse ; aucun script ne s'en sert.
- Ses touches sont celles d'un téléphone, dans l'ordre de l'exécutable (`0x363798`) : 1 à 9, *, 0, # ; le joueur y compose un nombre au curseur (`AD_TenKeyCursorProc`, `AD_TenkeyInputProc`), peut effacer (`AD_TenKeyDelEffectMake`), et le nombre s'affiche au fur et à mesure (`AD_TenkeyDisplayTran`).
- Ses images, `tenkey` (les touches) et `tenkeybg` (le fond), ne sont pas sur le disque.
