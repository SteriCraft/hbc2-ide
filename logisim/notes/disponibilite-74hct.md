# Disponibilité des puces 74HCT du HBC-2

Relevé du 4 novembre 2026 sur Octopart (stocks agrégés de Mouser, DigiKey, Arrow, Rochester, Verical, TME…). Prix unitaires à la pièce chez Mouser/DigiKey, en USD hors taxes. Les stocks bougent vite : à revérifier au moment de commander.

## Verdict

Les 12 références sont faciles à acheter, en DIP comme en SOIC, chez les distributeurs agréés. Toutes ont au moins une version **active** en DIP chez Texas Instruments, avec des centaines à des milliers de pièces en stock. Compter environ **0,80 à 1,50 $ la puce** en DIP à l'unité, et 0,30 à 0,80 $ en SOIC.

Trois pièges à éviter :

1. **Nexperia ne fait plus de DIP.** Les références Nexperia en `N` (74HCT11N, 74HCT86N…) sont obsolètes depuis 2015 (dernière commande 2015-06-30). Ce qu'on en trouve vient de revendeurs non agréés, souvent à 5 $ ou plus. En DIP, il faut prendre du **TI** (préfixes `CD74HCT…E` ou `SN74HCT…N`). Nexperia reste un bon choix en SOIC (suffixe `D`).
2. **Certaines versions TI sont en fin de vie (NRND)**, alors qu'une autre version TI de la même puce est active :
   - 74HCT02 : `SN74HCT02N` est NRND, prendre **`CD74HCT02E`** (actif, 2 000 en stock chez Mouser).
   - 74HCT157 : `SN74HCT157N` est NRND, prendre **`CD74HCT157E`** (actif).
   - 74HCT86 : `SN74HCT86N` n'a plus aucune offre, prendre **`CD74HCT86E`** (actif).
3. **Le 74HCT283 Nexperia en SOIC (`74HCT283D`) est retiré.** En SOIC, prendre le TI **`CD74HCT283M96`** (actif, ~3 900 chez Mouser). Le fichier chips.md cite encore `74HCT283D` : à corriger.

Le 74HCT27, signalé comme la puce la moins courante, ne pose en fait aucun problème : `CD74HCT27E` est actif, avec ~1 300 pièces chez Mouser et ~20 000 chez Rochester.

## Références recommandées

| Puce | DIP (TI) | Statut | Stock Mouser / DigiKey | Prix ×1 | SOIC conseillé |
|---|---|---|---|---|---|
| 74HCT02 | CD74HCT02E | actif | 2 054 / 1 634 | 0,78 $ | SN74HCT02D (TI) ou 74HCT02D (Nexperia)* |
| 74HCT08 | SN74HCT08N | actif | 9 437 / 3 177 | 0,92 $ | 74HCT08D (Nexperia)* |
| 74HCT11 | CD74HCT11E | actif | 1 405 / 2 800 | 1,12 $ | 74HCT11D (Nexperia)* |
| 74HCT27 | CD74HCT27E | actif | 1 299 / 447 | 1,08 $ | 74HCT27D (Nexperia), en production, ~0,37 $ chez TME |
| 74HCT32 | SN74HCT32N | actif | — / 1 072 | 0,94 $ | 74HCT32D (Nexperia)* |
| 74HCT86 | CD74HCT86E | actif | 2 560 / 1 881 | 1,07 $ | 74HCT86D (Nexperia)* |
| 74HCT138 | SN74HCT138N | actif | 1 265 / 2 692 | 0,88 $ | 74HCT138D (Nexperia)* |
| 74HCT153 | CD74HCT153E | actif | 682 / 642 | 1,22 $ | 74HCT153D (Nexperia), en production, 0,43 $ chez Mouser |
| 74HCT157 | CD74HCT157E | actif | 231 / 598 | 1,08 $ | 74HCT157D (Nexperia)* |
| 74HCT240 | CD74HCT240E | actif | 598 / 582 | 1,48 $ | 74HCT240D (Nexperia), en production, 0,53 $ chez Mouser |
| 74HCT283 | CD74HCT283E | actif | ~55 000 au total (4 distributeurs) ; 22 chez LCSC | ~1,10 $ | **CD74HCT283M96 (TI)**, 0,81 $ — pas Nexperia |
| 74HCT541 | SN74HCT541N | actif | 7 287 / 4 493 | 0,96–1,01 $ | 74HCT541D (Nexperia), en production, 0,44–0,64 $ |

\* Non vérifié une par une, mais ce sont des références courantes de Nexperia en production. À contrôler dans le panier.

À éviter : `SN74HCT02N` (NRND), `SN74HCT157N` (NRND), `SN74HCT86N` (plus d'offre), toutes les références Nexperia en `N` (DIP obsolète), `74HCT283D` (retiré).

## Où acheter depuis la France

- **Mouser et DigiKey** : tout est en stock. Livraison gratuite au-delà d'environ 50 € de commande, droits et TVA inclus, en 2 à 4 jours. Pour un CPU complet, une seule commande chez l'un des deux suffit.
- **TME** (Pologne) : bien fourni en Nexperia SOIC, souvent moins cher, livraison rapide en Europe.
- **LCSC** : prix bas, mais stocks DIP faibles sur certaines références (22 CD74HCT283E le jour du relevé). Plutôt utile si tu fais fabriquer des circuits imprimés chez JLCPCB en SOIC.
- **eBay / AliExpress** : à éviter pour ce projet. Les puces vendues comme « 74HCT » sont parfois des LS ou des HC remarquées, ou des fonds de stock de qualité inconnue. Une puce HC à la place d'une HCT peut mal lire les niveaux d'une sortie TTL, et ce genre de défaut est très long à dépanner.

## Familles : rappel rapide

- **74HCT** (choix du projet) : CMOS rapide, entrées compatibles TTL, très faible consommation, largement produit. Bon choix.
- **74HC** : même vitesse, encore plus courant, mais seuils d'entrée CMOS. À ne pas mélanger avec des sorties TTL ou des EEPROM/RAM à sorties TTL sans vérifier les niveaux.
- **74LS** : TTL bipolaire d'origine, plus gourmand. On en trouve surtout en fonds de stock (Rochester) et sur eBay. Pas de raison de l'utiliser ici.

## Conseil d'achat

Achète 10 à 20 % de puces en plus de ce qu'exige le schéma, surtout pour les références bon marché (portes logiques). Une puce grillée pendant le montage coûte moins cher qu'une deuxième commande avec frais de port.

## Sources

- Octopart : [CD74HCT27E](https://octopart.com/part/texas-instruments/CD74HCT27E), [CD74HCT283E](https://octopart.com/part/texas-instruments/CD74HCT283E), [CD74HCT283M96](https://octopart.com/part/texas-instruments/CD74HCT283M96), [CD74HCT153E](https://octopart.com/part/texas-instruments/CD74HCT153E), [CD74HCT11E](https://octopart.com/part/texas-instruments/CD74HCT11E), [SN74HCT541N](https://octopart.com/part/texas-instruments/SN74HCT541N), [SN74HCT157N](https://octopart.com/part/texas-instruments/SN74HCT157N), [CD74HCT157E](https://octopart.com/part/texas-instruments/CD74HCT157E), [SN74HCT02N](https://octopart.com/part/texas-instruments/SN74HCT02N), [CD74HCT02E](https://octopart.com/part/texas-instruments/CD74HCT02E), [SN74HCT08N](https://octopart.com/part/texas-instruments/SN74HCT08N), [SN74HCT32N](https://octopart.com/part/texas-instruments/SN74HCT32N), [SN74HCT86N](https://octopart.com/part/texas-instruments/SN74HCT86N), [CD74HCT86E](https://octopart.com/part/texas-instruments/CD74HCT86E), [SN74HCT138N](https://octopart.com/part/texas-instruments/SN74HCT138N), [CD74HCT240E](https://octopart.com/part/texas-instruments/CD74HCT240E), [74HCT86N,652](https://octopart.com/part/nexperia/74HCT86N,652), [74HCT27D,653](https://octopart.com/part/nexperia/74HCT27D,653), [74HCT283D,653](https://octopart.com/part/nexperia/74HCT283D,653), [74HCT153D,653](https://octopart.com/part/nexperia/74HCT153D,653), [74HCT541D,653](https://octopart.com/part/nexperia/74HCT541D,653), [74HCT240D,653](https://octopart.com/part/nexperia/74HCT240D,653)
- LCSC : [CD74HCT283E](https://lcsc.com/product-detail/Specialty-Logic_Texas-Instruments-CD74HCT283E_C1523488.html)
