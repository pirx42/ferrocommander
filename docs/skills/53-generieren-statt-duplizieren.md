# Skill: Generierte Datei statt hartkodierter Liste/Wert (gegen Drift)

**Wann.** Man steht vor einer Liste oder einem Wert, der sich aus einer
**Source-of-Truth ableiten lässt** — vorhandene Dateien im Build-Output,
`package.json`, installierte Versionen, eine bestehende Konstanten-Quelle —
und ist drauf und dran, ihn an einer **zweiten** Stelle von Hand zu pflegen.
Oder: ein Bug entpuppt sich als „die hartkodierte Kopie ist veraltet/unvollständig".

**Regel.** Ableitbare Daten werden zur **Build-Zeit generiert**, nicht dupliziert.
Ein Skript (`scripts/generate-*.ts`) liest die Quelle und schreibt ein
**eingechecktes** File unter `src/generated/`; der `npm run build` ruft es als
ersten Schritt auf (immer frisch). Konsumenten importieren das generierte File.

**Warum.** Jede handgepflegte Kopie driftet von ihrer Quelle weg — und der Drift
ist unsichtbar, bis jemand den Unterschied bemerkt:
- Eine neue Datei/Version kommt dazu, die Kopie wird nicht nachgezogen.
- Die Kopie probiert Dinge an, die es nicht (mehr) gibt → Fehler/404-Rauschen.

Konkret aus dieser Codebase (2026-06-30, je nach User-Report):
- **Run-Liste:** `NewRunScreen` fetchte eine feste `ALL_RUN_IDS`-Liste durch →
  404-Blind-Proben für im Public-Build gestrippte Runs. Fix: Build-Manifest
  `runs/index.json` (`generate-run-manifest.ts`) → nur Vorhandenes wird geladen.
- **Credits-Lizenzliste:** hartkodiert, 6/10 Versionen veraltet, Playwright +
  Fonts fehlten. Fix: `generate-licenses.ts` liest `package.json` × `node_modules`
  → Versionen + Lizenztexte bleiben automatisch korrekt.

**Wie anwenden.**

1. **Quelle identifizieren.** Woraus ist die Liste/der Wert ableitbar? (Verzeichnis-
   Listing, `package.json`, eine andere Konstante.)
2. **Pure Kernlogik nach `src/`** (testbar ohne fs), fs-/CLI-Wrapper ins Skript
   (`scripts/`). Muster: `src/run/runManifest.ts` (pure) + `scripts/generate-run-manifest.ts`.
3. **In den Build einhängen:** `npm run build` ruft das Skript auf (vor `tsc`/`vite`,
   je nach Abhängigkeit). Manuell via eigenes `generate:*`-npm-Script.
4. **Eingecheckt halten** (damit `vite dev` ohne Build läuft) — Muster
   `propertyNormFactors.ts`. Der Build regeneriert es (idempotent prüfen:
   `git diff` nach dem Build leer).
5. **Fallback im Konsumenten**, falls das generierte Artefakt fehlen kann (Dev ohne
   Build / alter Deploy) → graceful auf das alte Verhalten zurückfallen.

**Abgrenzung / Ausnahmen.**
- **Bewusst statische Werte** bleiben hand-gepflegt, wenn der User das so will
  (z. B. das README-Test-Count-Badge, User-Wunsch 2026-06-23) — das ist eine
  Entscheidung, kein Drift-Bug.
- Generieren lohnt nur, wenn die Quelle wirklich die Wahrheit ist und sich ändert.
  Eine 3-Zeilen-Liste, die sich nie ändert, braucht keinen Generator.

**Folge-Falle.** Was generiert/gestrippt wird, darf die App nicht an anderer Stelle
**hart referenzieren** (siehe die Run-Liste oben). Public-Build-Variante: siehe
[good-development-practices.md](../good-development-practices.md) 7.4.

**Folge-Falle 2 — Regenerieren ist ein LETZTER Schritt (2026-08-07, msg 16753).**
Generierte Content-Dokus (`docs:deployment`, `docs:loot`) mitten in einem
mehrphasigen Content-Umbau zu regenerieren erzeugt einen eingecheckten
Zwischenstand, der wie „aktuell" aussieht — nach späteren Phasen (hier:
deception-Rückbau + bolt-Fixes NACH der Phase-D-Regenerierung) ist er still
falsch. Regel: alle `docs:*`-Generatoren laufen einmal ganz am ENDE, nach der
letzten Content-Änderung; der Owner-Fund war, dass deployment-content.md noch
entfernte Pools als deployt listete. Zweiter Fund im selben Zug: der
Generator-Replace muss nach dem Lauf gegen die Quelle verifiziert werden
(`git diff` zeigt die erwartete Änderung?) — der docs:loot-Marker-Replace
matchte wegen Regex-Sonderzeichen nie und ließ den Block still veralten.

**Folge-Falle 3 — der Generator saugt maschinen-lokale Artefakte auf
(Housekeeping-Fund 2026-08-21).** Wer ein VERZEICHNIS ausliest, nimmt auch mit,
was dort nicht hingehoert. `generate-deployment-doc` zaehlte einen gitignorten
Optimizer-Snapshot (`static/runs/*.best-bot.json`, geschrieben von
`npm run optimize`) als Run — beim naechsten Regenerieren waere ein Artefakt vom
EIGENEN Rechner in einer committeten Doku gelandet. Dieselbe Datei bot der
Run-Picker als spielbaren Run an. Zwei Konsequenzen fuer dieses Skill:

- Den Diff einer regenerierten Doku IMMER lesen, bevor er committet wird. Ein
  Generator-Output ist nicht automatisch reproduzierbar — er ist nur so
  reproduzierbar wie sein Eingabe-Verzeichnis sauber ist.
- Nicht-Inhalte an der QUELLE ausfiltern und das Praedikat mit allen Konsumenten
  teilen (hier `runManifest.isRunFile`, genutzt von Manifest UND Doku-Generator)
  — nicht in jedem Generator eine eigene Filter-Kopie.

**Verwandt.**
- [16-keine-magic-values.md](16-keine-magic-values.md) / [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md)
- [44-keine-redundanzen.md](44-keine-redundanzen.md) (DRY — generieren ist DRY für Daten)
- `35-reproducible-deploy.md` (nur Chimera) (der Generator ist Teil des Build-Ablaufs)

**Quelle.** Learnings KW 2026-06-23…30: dreimal kostete eine hartkodierte Liste/Wert
einen User-Report (Run-404s, veraltete Credits-Versionen, fehlende Lizenztexte).
Good-dev-practices 3.3c.
