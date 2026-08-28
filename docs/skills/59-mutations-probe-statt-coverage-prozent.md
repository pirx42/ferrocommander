# Skill: Mutations-Probe statt Coverage-Prozent

**Wann.**
- Ein Feature/Subsystem gilt als „gut getestet", weil die Coverage gruen ist.
- Ein Bug taucht in Code auf, der laut Report abgedeckt war
  („wie kann das sein, da war doch Coverage drauf?").
- Beim Housekeeping-Pass (42 (`42-housekeeping-coverage-doku-drift.md`, nur Chimera)),
  als vierter Durchgang.
- Vor jeder Aussage der Form „das ist abgesichert".

**Regel.** **Coverage misst AUSFUEHRUNG, nicht BEHAUPTUNG.** Eine Zeile zaehlt
als „covered", sobald irgendein Test sie durchlaeuft — voellig unabhaengig
davon, ob danach jemals etwas ueber ihr Ergebnis assertet wird. Ein Feature kann
damit **100 % Coverage haben und 0 % getestet sein**.

Der einzige verlaessliche Test einer Absicherung ist die **Mutations-Probe**:
die Wirkung abschalten und schauen, ob die Suite schreit.

```
Kaputtmachen → volle Suite → bleibt gruen?  ⇒  KEIN Waechter. Test schreiben.
                             faellt etwas?  ⇒  Waechter existiert.
```

**Warum.** Chimera, 2026-07-14: die per-Tick-XP-Vergabe (`energyStep`) war
KOMPLETT ungetestet. Mit hart deaktivierter Vergabe (`if (false)`, also: kein
Item bekommt je wieder XP) liefen **8482 Tests gruen durch**. Kein einziger
Waechter — nach Monaten mit Coverage-Checks und Coverage-Verbesserungen.

Der Grund ist strukturell, kein Versehen: die XP-Zeilen laufen in JEDEM Test
mit, der die Sim tickt (`energyStep` haengt an `stepChassisFrame`). Sie sind
also bestens „abgedeckt" — und tauchen in einer Luecken-Triage
([27](27-coverage-luecken-triage.md)) per Definition NIE auf, denn die listet
nur NICHT-abgedeckte Zeilen. Der Housekeeping-Pass war gegen genau diese Klasse
**blind by design**. Coverage kann diesen Fehler nicht finden; nur die Mutation
kann es.

**How (Schritte).**

1. **Beobachtbare Ausgaben auflisten**, nicht Zeilen. Pro Subsystem: welche
   Zustands-Schreibungen sind das Produkt dieses Codes? (Chimera-Sim-Kern:
   `xp`, `charge`, `heat`, `itemHP`, Schild-Ladung, Fire-Timestamps,
   Siphon-Netto, Zerstoerungs-Waermebombe, …)
2. **Je Ausgabe eine Mutation**, die genau diese Wirkung loescht — nicht
   „irgendeine Zeile veraendern":
   - Zuweisung neutralisieren (`x = alt + delta` → `x = alt`)
   - Funktionsrumpf kappen (`{` → `{ if (true) return;`)
   - Setter no-op (`map.set(id, v)` → `void v;`)
3. **Volle Suite** je Mutation laufen lassen (nicht nur die vermeintlich
   zustaendige Datei — der Waechter darf ueberall sitzen).
4. **Bilanz:** jede Mutation, die gruen durchlaeuft, ist ein Loch. Test
   nachziehen ([26](26-verhaltens-tests.md) /
   [52](52-erhaltungs-invarianten-testen.md)).
5. **Baum wieder sauber:** Mutationen IMMER zurueckrollen (Backup vor dem
   Patch, `finally`-Restore, danach `git diff` pruefen). Ein vergessener
   `if (false)` im Sim-Kern ist ein Desaster.

**Fertiges Werkzeug: `npm run mutation-sweep`** (`scripts/mutation-sweep.sh`) —
deckt die zentralen Zustands-Schreibungen des Sim-Kerns ab. Exit 1, wenn eine
Mutation ueberlebt. Neue Mutation = eine `run_mutation`-Zeile.

**Die wichtigste Regel: GRUENE BASELINE ZUERST.** Bevor die erste Mutation
gesetzt wird, muss die Suite **ungemutet gruen** sein. Sonst liefert jede
Mutation Exit != 0 und wird als „GUARDED" verbucht — der Sweep meldet dann
zufrieden „alles abgesichert" und hat in Wahrheit **nichts gemessen**.

Das ist kein theoretisches Risiko. Am 2026-07-14 rief das Skript `npx vitest
run` **ohne die Projekt-Excludes** aus `package.json` (`--exclude='tests/**'`).
Damit zog vitest die Playwright-e2e-Specs mit in den Lauf, die unter vitest
prinzipiell scheitern („Playwright Test did not expect test() to be called
here") — die Suite war **immer** rot. Ergebnis: 9/9 und 19/19 „GUARDED",
allesamt wertlos, inklusive einer bereits an den User gemeldeten Schlussfolgerung.

Daraus zwei harte Konsequenzen:

1. **Baseline-Gate ins Skript** (ist drin): ungemuteter Lauf, rot → Abbruch.
2. **Der Sweep muss EXAKT die Suite fahren, die das Projekt fuer gruen haelt** —
   also dieselben Flags wie das `test`-Script in `package.json`. Eine andere
   Suite zu messen als die, gegen die committet wird, ist per se bedeutungslos.

Und eine Lehre ueber Kontrollen: der Sweep hatte zwei „Kontroll-Mutationen" an
Stellen, die als geschuetzt galten. Die haben den Fehler **nicht** gefangen —
sie koennen „geschuetzt" nicht von „immer rot" unterscheiden, beide sehen aus
wie Exit != 0. Die einzige Kontrolle, die greift, ist der **Leerlauf** (gar
keine Mutation → muss gruen sein). *Wenn ein Test nicht fehlschlagen KANN, ist
er keine Kontrolle.*

**Betriebs-Fallen (im Skript geloest, nicht wegoptimieren):**

| Falle | Loesung |
|---|---|
| **Suite schon ohne Mutation rot → alles meldet GUARDED, der Sweep misst nichts** | **Baseline-Gate**: ungemuteter Lauf vor der ersten Mutation, rot → Exit 2. Und dieselben Flags wie `npm test`. |
| Volle Suite je Mutation = teuer | `--bail=1` — wir wollen nur wissen OB ein Waechter existiert. Geschuetzte Mutation stirbt beim ERSTEN Failure (~2 min) statt die Suite auszusitzen. |
| Manche Mutationen lassen Sim-Tests in ihr 120-s-Timeout laufen (sie warten auf einen Zustand, der nie eintritt) — ein Lauf zieht sich auf Stunden | kurzer `--testTimeout=8000` |
| Pythons `subprocess`-Timeout killt `npx`, die vitest-Worker halten aber die stdout-Pipe → der Sweep haengt statt weiterzulaufen | `timeout(1)` auf Shell-Ebene: killt die ganze **Prozessgruppe** |
| `pkill -f <muster>` matcht die eigene Shell (Exit 144) | Bracket-Trick `"[m]uster"` — oder gar nicht killen |
| Abbruch mitten im Sweep laesst die Mutation im Code | Restore im `trap EXIT` **und** `git diff`-Check am Ende (Exit 2 wenn dirty) |
| **Sweep im Hintergrund laufen lassen und nebenher weiterarbeiten** | **Nicht tun.** Der Sweep mutiert echte Dateien im Arbeitsbaum. Waehrend er laeuft, ist der Baum zeitweise absichtlich kaputt: `git status` zeigt fremde Aenderungen, ein `git add -A` committet eine MUTATION, und jedes parallele `tsc`/`vitest`/`lint` misst einen manipulierten Stand. 2026-07-15 stand `src/heatManager.ts` genau so da — der Commit kam nur nicht zustande, weil `git status` vor dem Adden gelesen wurde. Entweder warten, oder den Sweep in einem separaten Worktree fahren. |

**Ergebnis des Sweeps (2026-07-14, 28 Mutationen, korrigiertes Harness):**
**11 GUARDED, 17 STUMM.** Und die Verteilung ist die eigentliche Erkenntnis:

| Schicht | Ergebnis |
|---|---|
| **Sim-Kern (Physik):** Energie, Hitze, Ladung, Schaden | **9/9 GUARDED** |
| **Pure Reducer** (`src/run/`, `src/profile/` — beide Kontrollen) | **GUARDED** |
| **React-Hook-Naht** (`useRunFinalize`, `useSectorTasks`, `useRunChallengeTick`, `useRunProgression`, `useCodexEncounters`, `useTimedEvents`, `useDialog`) | **17/17 STUMM** |

Die Physik ist dicht, weil dort jeder fehlende Term eine **Erhaltungs-Invariante**
verletzt ([52](52-erhaltungs-invarianten-testen.md)) — ein Test faellt von selbst.
Die Loecher sitzen woanders, und zwar an einer sehr scharfen Kante:

> **Die puren Module sind getestet. Dass ihr Ergebnis je im State landet, testet niemand.**

Chassis-Freischaltung ist das Musterbeispiel: die Trigger-Tabelle hat einen Test,
die Profil-Mutation hat einen Test — aber dass am Run-Ende irgendwer die eine an
die andere haengt, behauptet kein einziger. Man kann die Zeile loeschen und die
Suite bleibt gruen. Dasselbe gilt fuer Combos, Run-Unlocks, Statistik,
Run-Historie, Codex-Erfassung und das Run-Ende-`completed: true`.

**Zwei Krankheiten, die der Sweep in einen Topf wirft (2026-07-15).** Nach dem
Coverage-Pre-Check des Hook-Naht-Plans:

| Datei | Coverage | Mutation | Diagnose |
|---|---|---|---|
| `hooks/useRunFinalize.ts` | **0 %** | stumm | gar nicht ausgefuehrt — **Coverage haette gereicht** |
| `run/runLifecycle.ts` | **90 %** | stumm | ausgefuehrt, aber nichts behauptet — **nur die Mutation findet das** |

Also: „stumm" heisst nicht automatisch „Coverage hat versagt". Erst der Blick auf
die Coverage-Zahl sagt, WELCHE der beiden Krankheiten vorliegt — und damit, ob
ein Harness fehlt (0 %) oder eine Assertion (90 %).

**Das Messgeraet misst nur, was du hineinschreibst.** Beim Absichern von
`useRunFinalize` fiel auf, dass `unlockComboForChallenge` (Challenge → Loot-Combo)
in KEINER Mutation vorkam — der Sweep meldete die Zeile nie, also galt sie
stillschweigend als in Ordnung. Sie war ungeschuetzt. Ein Sweep beweist etwas
ueber die Zeilen, die er anfasst, und **nichts** ueber die anderen. Beim Anlegen
einer Mutations-Gruppe darum die Wirkungs-Stellen einer Datei durchzaehlen, nicht
die auffaelligste nehmen. (Zusatzfalle: die beiden Combo-Zeilen unterscheiden sich
nur in der Einrueckung — die eine ist ein SUBSTRING der anderen, `sed`/`replace`
trifft die falsche. Zwei-Zeilen-Anker.)

**„Es gibt doch einen Test auf das Feld" sagt nichts ueber den PFAD.**
`useDialog.closedAtMs` hatte einen Test — fuer den Seen-Once-Pfad, wo der Eintrag
schon beim Einreihen als geschlossen angelegt wird. Der Dismiss-Pfad
(`markCurrentClosed`), an dem der Ungelesen-Punkt haengt, lief in keinem. Die
Mutation dort ueberlebte, obwohl das Feld „getestet" aussah.

**Merke — zwei Sonden, nicht eine:**
1. **Buchhaltung** statt Physik (XP, Level, Freischaltungen, Statistik, Waehrung):
   ihr Ausfall verletzt keine Invariante, also schweigt die Suite.
2. **Naht** statt Kern: nicht nur „wirkt die Regel?", sondern „ist die Regel
   ueberhaupt angeschlossen?". Ein gruen getesteter Reducer, den niemand aufruft,
   ist totes Kapital — und Coverage zeigt ihn als abgedeckt.

**Verhaeltnis zu Coverage.** Coverage bleibt nuetzlich — aber NUR als
Negativ-Signal: „hier laeuft gar nichts durch" ist eine harte Aussage. Der
Umkehrschluss ist ungueltig. Also:

| Signal | Aussagekraft |
|---|---|
| Coverage 0 % | Echte Luecke. Zuverlaessig. |
| Coverage 100 % | **Sagt nichts** ueber Absicherung. |
| Mutation ueberlebt | Echtes Loch. Zuverlaessig. |
| Mutation stirbt | Waechter existiert. Zuverlaessig. |

**Anti-Pattern.**
- „Die Datei hat 90 % Coverage, das Feature ist abgesichert" — Nicht-Sequitur.
- Nur die Datei-eigene Test-Suite laufen lassen (`vitest run <file>`) — der
  Waechter koennte woanders sitzen; ein gruener Teillauf beweist nichts.
- Mutation nicht zurueckrollen.
- Die Mutation so waehlen, dass sie einen Typfehler/Crash erzeugt: dann faellt
  die Suite aus dem falschen Grund und du hast nichts gelernt. Die Mutation muss
  **kompilieren und laufen** — nur die WIRKUNG fehlt.

**Verwandt.**
- [52-erhaltungs-invarianten-testen.md](52-erhaltungs-invarianten-testen.md) —
  „Waechter durch Fix-Revert verifizieren" ist derselbe Gedanke, dort fuer den
  EINZELNEN neuen Fix; dieses Skill zieht ihn als Sweep ueber den Bestand.
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md) — die
  Luecken-Triage; strukturell blind fuer abgedeckt-aber-unbehauptet.
- `42-housekeeping-coverage-doku-drift.md` (nur Chimera)
  — Mutations-Pass ist dort der vierte Durchgang.
- [26-verhaltens-tests.md](26-verhaltens-tests.md) — was die gefundenen Loecher
  fuellt.
