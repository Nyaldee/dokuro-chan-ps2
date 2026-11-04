# Les scripts STCM2

L'histoire du jeu est écrite dans des scripts au format **STCM2** (« STCM2 File Make By Minku 06.0 »), rangés dans `UNION/SCRIPT.UNI`. Ce document décrit leur format et leurs instructions, d'après la machine qui les lit dans l'exécutable (`STCM2_Run`, `GetFunctionVariable`, et les fonctions `STCM2_*` de la table des instructions).

Outils : `Tools/uni_extract.py` sort les scripts du disque, `Tools/stcm2_table.py` sort la table des instructions de l'exécutable, `Tools/stcm2_dump.py` désassemble un script en texte lisible (voir `Tools/README.md`).

## Les fichiers

`SCRIPT.UNI` contient 35 scripts, par numéro d'entrée :

| Entrée | Rôle |
|---|---|
| 0 | `init` : lance le prologue (101), les scripts de départ 1 à 4, celui de la carte (130) et le scénario bonus (127) |
| 1 | les personnages : leurs images (`chara_gp_entry`), corps (`chara_pt_set`) et expressions (`chara_emo_set`) |
| 2 | les décors, l'album (`cglib_entry`), les images gagnées aux mini-jeux, l'image de fin |
| 3 | les vidéos : chaque vidéo de `PSS/` et sa vignette dans l'album |
| 4 | les drapeaux de l'histoire (`flag_make`, `spflag_make`), les bulles du menu, les résultats, les fins |
| 101 à 130 | les chapitres de l'histoire (101 : le prologue ; 127 : le scénario bonus « 12歳の世界！ ») |

## Format d'un fichier

Toutes les valeurs sont des mots de 32 bits, petit-boutistes. Les adresses comptent depuis le début du fichier.

| Place | Contenu |
|---|---|
| 0x00 | « STCM2 File Make By Minku 06.0 », complété de zéros (0x20 octets) |
| 0x20 | adresse de la table des points d'entrée |
| 0x24 | nombre de points d'entrée |
| 0x50 | « GLOBAL_DATA » (12 octets), puis les variables du fichier (des données) |
| … | « CODE_START_ » (12 octets), puis les actions à la suite |
| … | « EXPORT_DATA » (12 octets), puis les points d'entrée |

Un **point d'entrée** fait 0x28 octets : un mot (0), le nom sur 32 octets (« sure1 », « init »…), l'adresse de sa première action.

### Les actions

Une action fait un en-tête de 16 octets, ses paramètres, puis les données qu'ils montrent ; sa longueur totale permet de passer à la suivante.

| Mot | Contenu |
|---|---|
| 0 | 1 : appel d'un sous-programme du fichier ; 0 : instruction |
| 1 | l'adresse du sous-programme (appel), ou le numéro de l'instruction |
| 2 | nombre de paramètres |
| 3 | longueur de l'action, en octets |

Un appel garde l'adresse de retour (pile de 8 niveaux, `piVar10[0xaa]`) ; l'instruction 0 revient de l'appel, ou arrête le fil s'il n'y en a pas.

### Les paramètres

Un paramètre fait trois mots `v0 v1 v2`, que `GetFunctionVariable` transforme en un pointeur et une sorte avant l'instruction :

| `v0` | Sens |
|---|---|
| `0xff000000 + n` | la valeur `n` (sur 24 bits) |
| `0xffffff41`, `0xffffff40` | une adresse (`v1`) : cible d'un saut |
| `0xffffff00 + n` (n < 0x20) | le n-ième argument du sous-programme en cours |
| `0xffffff20 + n` (n < 8) | un registre du fil (0 à 3 entiers, 4 à 7 flottants) |
| autre | l'adresse d'une donnée ; `v1` et `v2` sont deux indices de tableau (`0xff000000 + n`, ou l'adresse d'une donnée qui les donne) |

### Les données

Une donnée fait un en-tête de 16 octets puis sa valeur :

| Mot | Contenu |
|---|---|
| 0 | sorte : 0 entier, 1 flottant |
| 1 | taille en mots |
| 2 | 1 |
| 3 | taille en octets |

Les textes sont des données (de sorte 0) en Shift-JIS, finies par des zéros jusqu'au mot. Les variables du fichier (`GLOBAL_DATA`) sont des données d'un mot, que les instructions lisent et écrivent en place.

## Les instructions

### Faites par la machine (`STCM2_Run`)

| N° | Nom (outil) | Rôle |
|---|---|---|
| 0 | `return` | retour d'appel, ou fin du fil |
| 4 | `yield` | rend la main jusqu'à l'image suivante |
| 6 | `if_int(a, op, b, cible)` | compare deux entiers : continue à l'action suivante si la condition tient, sinon saute à `cible` |
| 7 | `goto(cible)` | saut |
| 8 | `random(dest, n)` | `(rand() + rand() % 3) % n` |
| 9 | `loop(cible, i, n)` | `i += 1` ; saute à `cible` tant que `i < n` |
| 0x0d | `if_float` | comme `if_int`, en flottants (`op` 2 et 3 : `>=` et `<=`) |
| 0x32 | `wait_frames(n)` | attend que le fil ait passé `n` images sur la même action |

Les conditions de `if_int(a, op, b, cible)` :

| `op` | Continue si |
|---|---|
| 0 | `a > b` |
| 1 | `a < b` |
| 2 | `a >= b` |
| 3 | `a <= b` |
| 4 | `a == b` |
| 5 | `a != b` |

Les calculs ont pour numéro `0xffffff00` à `0xffffff14` : `add`, `sub`, `mul`, `div`, `mod`, `shl`, `shr`, `and`, `xor`, `or` (trois paramètres : `dest = a op b`), puis `+=`, `-=`, `*=`, `/=`, `%=`, `<<=`, `>>=`, `&=`, `^=`, `|=` et `=` (deux paramètres). Ils se font en entiers ou en flottants selon la sorte de la donnée modifiée.

### Les instructions du jeu

La table de l'exécutable (`0x35bf60`, 500 entrées) donne une fonction `STCM2_*` à 223 numéros ; la plupart servent aux objets 3D d'autres jeux du même moteur. Les scripts de Dokuro-chan n'en utilisent qu'environ 80 :

| Usage | Instructions |
|---|---|
| Texte | `maintxt_print` (une ligne de plus dans le lot), `batch` (fin du lot : une page), `speaker_change` (le nom), `txtwaitset` (attente imposée), `txtwin_type` |
| Voix et sons | `voice_play`, `bgm_play`, `bgm_stop`, `strse_play` (bruitage en flux, de `SE.VBG`), `strse_waitcut`, `loopse_play`, `loopse_stop` |
| Personnages | `ezchara_stateset`, `ezchara_app`, `ezchara_nextstate`, `ezchara_move`, `ezchara_del`, `trchara_up_all`, `chara_emo_set`, `chara_pt_set`, `chara_entry`, `chara_gp_entry`, `chara_txt_set`, `color_set` |
| Décors et caméra | `bg_entry`, `bg_print_change`, `ezbg_change`, `target_point_set`, `target_count_set`, `target_shake_set`, `target_angle_set`, `target_rgb_set` |
| Fondus et transitions | `blackout_start`, `blackout_end`, `ezeffect_scenechange2` |
| Enchaînement | `thread_change` (autre point d'entrée), `file_change` (autre script), `start_stcm`, `start_mapstcm`, `mapstcm_change`, `init_stcm_set`, `bonus_stcm`, `game_end`, `save_name` (le titre de la sauvegarde) |
| Choix | `menutxt_set` (une réponse), `menu_set` (le drapeau qui reçoit le choix) |
| Drapeaux | `flag_make`, `flag_set`, `flag_change`, `flag_number`, `flag_cmp`, `spflag_*` (drapeaux gardés d'une partie à l'autre) |
| Carte | `map_pos_dataset`, `map_pos_stateset`, `map_pos_commentset`, `map_mark_pos_set`, `map_selmode` |
| Mini-jeux | `mogura_mode`, `run_mode`, `card_mode`, `catch_mode`, `bat_mode`, `minigame_selchara_get`, `minigame_result_get`, `minigame_bonus_cg_set`, `minigame_challenge_*` |
| Fins de journée | `result_mode`, `result_data_set`, `distribute_mode`, `ending_check_flag_set`, `ending_check_bg_set` |
| Album et vidéos | `cglib_entry`, `cglib_movie_entry`, `movie_entry`, `movie_play`, `thum_gp_entry` |
| Divers | `restrict_menu`, `restoff_menu`, `cdmode_check`, `rand` |

Le détail de chaque instruction (paramètres, effet) est dans [moteur.md](moteur.md), avec le moteur qui les joue.

## Un exemple

Le début du prologue (script 101), tel que l'écrit `stcm2_dump.py` :

```
== sure1 ==
    9c  flag_number(dc4='switch', g5c=0)
    dc  if_int(g5c=0, 4, 0, ->138)
   11c  goto(->1f0)
   ...
   1f0  save_name(d20c='プロローグ～草壁桜の日常～')
   238  bgm_play(3, 0)
   260  ezeffect_scenechange2(0, 25, 1, 0)
   2a0  bg_print_change(d2ec='bg03_a', 0, 0, 5, 0)
   304  maintxt_print(d320='僕の名前は草壁桜（くさかべさくら）。')
   358  batch()
```

Une ligne : l'adresse de l'action, l'instruction, ses paramètres (`d…` une donnée et sa valeur, `g…` une variable du fichier, `->…` une cible de saut).
