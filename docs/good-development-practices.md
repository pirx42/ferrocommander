# Good Development Practices

> **Herkunft:** uebernommen aus dem Chimera-Projekt (`../chimera/docs/good-development-practices.md`,
> Stand 2026-08-28). Projekt-spezifische Beispiele darin (npm-/vitest-/Playwright-Kommandos,
> RL-Training, Deploy auf Port 3012, Telegram-Kanal) sind als Illustration zu lesen —
> die Rust-/Cargo-Aequivalente fuer dieses Projekt stehen im Root-[CLAUDE.md](../CLAUDE.md)
> und in [skills/25-gruene-suite-vor-commit.md](skills/25-gruene-suite-vor-commit.md).
> Die Prinzipien (Teil A), der Workflow (Teil B) und der Vier-Phasen-Zyklus (Teil C)
> gelten unveraendert.

Arbeitsregeln, die sich in diesem Projekt bewaehrt haben. Das Dokument
richtet sich an Menschen **und** AI-Agenten, die am Code mitarbeiten.
Es ist absichtlich praeskriptiv formuliert: wer abweicht, sollte einen
bewussten Grund haben.

Vier Teile:

- **Teil A — Prinzipien (thematisch):** Regeln pro Bereich (Code,
  Tests, Commits, …) mit Begruendung und Beispiel.
- **Teil B — Workflow (chronologisch):** ein End-to-End-Ablauf von der
  Aufgabenannahme bis zum Push, in dem die Prinzipien zur Anwendung
  kommen.
- **Teil C — Vier-Phasen-Zyklus (praeskriptive Empfehlung):** ein
  Mehr-Tage-Rhythmus mit klar abgegrenzten Phasen (Feature → Tests →
  Doku → Refactor) und Zeitrahmen.
- **Teil D — Beobachteter Arbeitsrhythmus aus der Commit-Historie:**
  empirische Baseline aus der echten Commit-Historie des Chimera-Projekts,
  an der sich Teil C orientiert.

**Arbeitsteilung mit [docs/skills/](skills/CLAUDE.md):** die Skills sind
das operative Regelwerk — bei inhaltlicher Ueberlappung ist die
Skill-Datei die kanonische Stelle (SSOT) und dieses Dokument verweist
dorthin. Das GDP traegt die Landkarte (Teil A als thematischer Ueberblick),
die Ablaeufe (Teil B/C), die Empirie (Teil D) und Volltext nur fuer
Regeln ohne eigenes Skill.

---

## Teil A — Prinzipien

### 1. Kommunikation mit dem Auftraggeber

#### 1.1 Frage nach, wenn der Auftrag mehrdeutig ist

**Regel.** Bei mehrdeutigen oder option-haltigen Aufgaben frage **vor**
der Umsetzung nach. Biete benannte Optionen (A/B/C) mit Tradeoffs an,
nicht nur eine Liste offener Fragen.

**Warum.** Jede Sekunde Klarstellung vor der Arbeit spart Stunden
Rueckbau. Optionen benannt zu machen ist hoeflicher und leichter zu
beantworten als ein offenes „Wie soll das genau sein?".

**Beispiel.** Wenn eine Feature-Beschreibung wie „adde ein Skip-Button"
kommt, klaere:
- Wie viele Penalties bei Skip? (1 zufaellig / alle / keine)
- Auf welchen Sektoren aktiv / ausgegraut?
- Confirm-Dialog oder Ein-Klick?

Danach auf die Antwort warten, nicht spekulativ anfangen.

#### 1.1a Nichts erfinden — verifizieren oder nachfragen (seit 2026-07-18)

**Regel.** Aussagen ueber Spielverhalten, Code oder Anforderungen werden
NIE aus Annahme geschrieben: erst gegen die Implementierung verifizieren
(Code/gepflegte Doku); bleibt es unklar, IMMER nachfragen statt raten
(Owner msg 15477/15478). „Plausibel" ist kein Beleg.

**Warum.** Handbuch-Vorfall 2026-07-18: ~9 Annahme-Fehler (Kabel-Button,
„Munition", Infiltration verkehrt herum, …) — jede Korrektur-Runde kostet
mehr als die Verifikation. Details in
[docs/skills/65-nichts-erfinden-verifizieren-oder-fragen.md](skills/65-nichts-erfinden-verifizieren-oder-fragen.md).

---

#### 1.2 Halte Rueckmeldungen knapp und konkret

**Regel.** Antworten in der Chat-/Terminal-Schnittstelle sind standardmaessig
kurz. Details nur wenn sie direkt Entscheidungsrelevant sind. End-of-Turn-
Summary passt in 1–2 Saetze.

**Warum.** Lange Zusammenfassungen am Ende jeder Iteration kosten
Zeit, sind kaum lesbar und wiederholen Informationen, die im Diff
stehen.

**Beispiel.** „Deployed (abc1234). 1580 Tests gruen." reicht. Kein
Absatz ueber jeden geaenderten Selektor.

#### 1.3 Meta-Fragen direkt beantworten

**Regel.** Auf Fragen wie „gepusht?" / „schon committed?" / „in welcher
Datei?" liefere zuerst die Antwort, dann ggf. Kontext.

**Warum.** Wenn jemand fragt „gepusht?", hilft „ja, Commit abc1234"
sofort — ein Absatz Vorrede ist irrelevant.

#### 1.4 Bestaetigung fuer destructive / shared-state Aktionen

**Regel.** Operationen mit nicht-lokaler Wirkung brauchen explizites
Einverstaendnis: `git push` **von Produktions-Code**, force-push, dependency
downgrades, DB-Migrationen, Dienste neu starten, Nachrichten an Dritte,
API-Calls mit Kosten. Einmalige Zustimmung gilt nur fuer den konkreten Scope.

**Push-Freigabe (Stand Branch-Workflow 2026-07-18, §6.7 / Skill 64):**
JEDER Push auf dev oder main braucht eine Freigabe — der Abnahme-Merge
eines Plans zaehlt als Freigabe. Nur Topic-Branch-Pushes sind frei (kein
CI-/Deploy-Effekt). `git push --force` IMMER fragen. Historie: die
fruehere Ausnahme „reine Test-/Doku-Aenderungen direkt pushen" (User-Spec
msg 11745, 2026-06-09) ist damit ABGELOEST (siehe §6.7).

**Warum.** Ein versehentliches `git push --force` auf main kann
Arbeit anderer zerstoeren. Ein Restart im falschen Moment kann Nutzer
mitten im Level abwuergen. Die Kosten fuer das Nachfragen sind klein,
die Kosten fuer einen Fehler oft gross.

**Beispiel.** Selbst wenn das CLAUDE.md sagt „Fuehre git commits
selbststaendig durch", ist ein Push auf dev/main dennoch ein separater
Schritt mit eigener Freigabe — weil er Sichtbarkeit nach aussen hat.

#### 1.5 Multi-Phasen-Plan: alle Phasen durchziehen, nur bei Entscheidungen unterbrechen

**Regel.** Sobald ein Plan vom Auftraggeber freigegeben ist, alle Phasen
am Stueck durcharbeiten. Unterbrochen wird ausschliesslich fuer echte
Entscheidungen (Tradeoffs, Schema-Brueche, neue Anforderungen). Nach
jeder Phase pushen, knapp Bescheid geben, weitermachen.

**Warum.** Sich nach jedem Sub-Schritt zu vergewissern produziert
unnoetige Wartezeiten und macht es schwer, einen Plan in einem Stueck
mental zu halten. Wenn der Plan vorher abgestimmt war, ist Wegarbeiten
das Default.

**Anti-Pattern.** „Phase 1 fertig — soll ich Phase 2 anfangen?" wenn der
Plan Phase 1+2+3+4 enthielt und es keine neuen Erkenntnisse gibt.

#### 1.6 Updates bei langen Aufgaben (~20-min-Kadenz + Meilensteine)

**10-Minuten-Kontrollpunkt (seit 2026-07-18, msg 15522).** Laengere Laufe
werden nach spaetestens 10 Minuten INHALTLICH geprueft (Scope/Parameter/
Durchsatz/Restdauer) — nicht nur Fortschritt gemeldet. CPU-Last beweist
nur „rechnet", nicht „das Richtige" (simulateAll-Vorfall: 1,9 h falscher
Suite-Scope). Details Skill [05](skills/05-updates-bei-langen-tasks.md).

**Regel.** Bei Aufgaben, die laenger als ~5 Minuten reine Arbeitszeit
brauchen, etwa alle **~20 Minuten** kurz Bescheid geben — was passiert
gerade, was ist der naechste Schritt — **plus** ein Status-Ping nach
jedem abgeschlossenen Meilenstein (Plan-Teilaufgabe/Phase/Sub-Agent).
Kadenz vom User am 2026-07-09 von ~5 min auf ~20 min hochgesetzt
(Memory `feedback_long_task_updates`); nicht mehr im starren 5-Minuten-
Takt. Bei Sub-Agent-Spawns gilt das Stagger-Update fuer den auf Antwort
wartenden Hauptthread.

**Warum.** Stille ueber mehr als ein paar Minuten erzeugt Unsicherheit
beim Auftraggeber — ist es noch dran, haengt es, ist es vergessen?
Kurze Sichtbarkeit kostet wenig und beruhigt viel.

**Hintergrund-Jobs beobachtbar machen** (2026-07-16): Output in eine
Log-DATEI (`> job.log 2>&1`), nie nur durch `| tail`; vor jeder
Haenger-Diagnose die Lauf-Konfiguration/Defaults pruefen (100 % CPU
beweist nur „rechnet", nicht „terminiert"). Details Skill
[05](skills/05-updates-bei-langen-tasks.md).

#### 1.7 Pausen explizit ansagen

**Regel.** Wenn die Arbeit einen Halte-Punkt hat (warten auf User-
Entscheidung, fertig ohne weitere Tasks, Sub-Agent laeuft im Background
und Hauptthread idle), das **explizit** melden. Nie stillschweigend
verstummen.

**Warum.** Aus Sicht des Auftraggebers ist „kein neuer Output" identisch
mit „crashed", „vergessen" oder „wartet auf mich" — er muss
selbst nachfragen, um den Unterschied zu sehen. Eine Zeile „warte auf
Entscheidung zu X" oder „alle Tasks durch — bereit fuer naechste
Runde" loest die Mehrdeutigkeit.

#### 1.8 Nacht-/Offline-Autonomie

**Regel.** Wenn der Auftraggeber explizit ankuendigt offline zu gehen
(„ich gehe schlafen", „bin morgen wieder da", aehnlich), arbeite bis
zur angegebenen Wiederkehr-Zeit (typisch ~7:00) autonom an
entscheidungsfreien Tasks. Nicht idle warten. Tasks die Rueckfragen
brauchen werden NICHT angefangen — die warten bis morgens.

**Warum.** Die wachen Stunden des Auftraggebers sind die einzige Zeit,
in der Entscheidungen geklaert werden koennen. Decisions-required-Tasks
in der Nacht zu blockieren waere Verschwendung dieser Zeit. Niedrig-Risiko-
Tasks (Doku, Test-Nachzug, klare Bugfixes, Translation, Audit-Folgen)
sind dagegen ideal — der Auftraggeber findet morgens Fortschritt vor.

**Verhalten.**
- Vor Beginn: vorhandene Memory-Eintraege (Pflichtregeln, Konventionen)
  laden und befolgen — nicht die Gelegenheit nehmen, Hausregeln zu
  brechen.
- Risiko-Filter: keine Schema-Brueche, keine Refactors die viele Konsumenten
  beruehren, keine destruktiven Git-Operationen ohne ausdrueckliche
  Vorab-Zustimmung. Bei aufkommender Entscheidung: vermerken (Memory /
  Plan-Doku) und liegen lassen.
- Stoppen wenn alle entscheidungsfreien Tasks durch sind — nicht
  Risiko-Tasks angehen, nur um die Zeit zu fuellen. Ein kurzer
  End-of-Night-Status reicht.

#### 1.9 Auf demselben Kanal antworten

**Regel.** Antwort + alle Rueckfragen gehen ueber den **Kanal zurueck,
ueber den die Nachricht eingegangen ist**. Telegram-Eingang → Telegram-
Reply. Terminal-Eingang → Terminal-Output. Nie stillschweigend wechseln,
auch nicht „weil das gerade einfacher ist".

**Warum.** Der Sender liest nur den Kanal, ueber den er geschrieben hat —
seine Push-Notifications und seine Aufmerksamkeit sind dort. Wenn die
Antwort woanders landet, sieht er sie nicht. Insbesondere: Telegram-
Eingang mit Terminal-Reply ist fuer den Sender wie kein Reply.

**Verhalten.**
- Telegram-Eingang → `reply`-Tool mit `chat_id` aus dem `<channel>`-Tag;
  `react` fuer Bestaetigung, `edit_message` fuer Live-Status, **neue**
  `reply` am Ende einer langen Task (Edits triggern keine Push-
  Notification).
- Terminal-Eingang → direkter Text-Output, keine Telegram-Tools.
- Keine `AskUserQuestion`-Dialoge im Terminal, wenn die Anfrage per
  Telegram kam.

#### 1.10 Vor aufwaendigem Selbst-Bauen kurz fragen

**Regel.** Bevor ein Artefakt **muehsam von Hand zusammengebaut** wird,
das der User interaktiv (im Spiel, im Tool, aus seinem Kopf) schneller
und zuverlaessiger erzeugen koennte, erst kurz fragen: „Kannst du X
schneller vorbereiten/exportieren?". Dann entscheidet der User, ob er
etwas vorbereitet oder ob ich es selbst baue. Nicht stillschweigend
lostuefteln.

**Warum.** User-Spec msg 11713 (2026-06-08). Ausloeser: ein
Infiltrations-Save fuer e2e-Tests sollte synthetisch erzeugt werden
(Bot-Sim-Dump + Hand-Edit der JSON + mehrere Fehlproben fuer
`activeBossBody` + Energie-Netz + Infiltrator-Charge). Der User hat
denselben Aufbau im Spiel in Sekunden gebaut und als Save geschickt —
exakt korrekt. Eine Frage vorab haette den Umweg gespart.

**Verhalten.**
- Erkennen: „Das baue ich jetzt aufwaendig von Hand" — besonders bei
  Spielstaenden, Fixtures, Setups, die im Tool interaktiv entstehen.
- Kurze, konkrete Frage ueber den aktiven Kanal stellen + sagen, wo ich
  das Ergebnis ablege.
- Auf Antwort warten; bei „bau du" den Eigen-Weg gehen.
- **Nicht** fragen bei trivialen oder rein code-seitigen Artefakten,
  oder wenn der User offline ist (Nacht-Autonomie → guenstigsten
  Eigen-Weg waehlen, Ergebnis spaeter zur Review stellen).

Siehe Skill [51](skills/51-vor-aufwand-fragen.md).

---

#### 1.11 Vorschau-Bilder fuer visuelles Feedback erstellen

**Regel.** Erzeugt eine Aufgabe ein **visuelles Ergebnis** (Icon/Emblem,
Karten-Grafik, Layout, Farbpalette, Rendering-Variante), zuerst ein
**Vorschau-Bild aller Varianten nebeneinander** rendern, selbst per `Read`
pruefen und dann dem User schicken — statt ihn im Spiel danach suchen zu
lassen. Das Galerie-Script **ins Repo committen**, weil man es bei jedem
Tweak wieder braucht.

**Warum.** User-Spec msg 13771 (2026-07-02, Upgrade-Medaillen): „diese
vorschau ist super gut. bitte merke dir bei solchen aufgaben immer
vorschauen zu erstellen fuer feedback. das geht viel schneller als im spiel
selbst." Im Spiel treten nicht alle Events/Upgrades/Items pro Run auf — man
saehe die Varianten nie zusammen und muesste bis zum Boss spielen. Ein
Seite-an-Seite-Board kostet einen Render-Zyklus und macht subjektive
Design-Fragen (Emblem, Kontrast, Farbe) sofort entscheidbar. So baut man
nicht N Motive „blind" fertig, bevor der User eins sieht.

**Verhalten.**
- Galerie-Entry mit der **echten Komponente** fuer ALLE Varianten (kein
  Nachbau → keine Divergenz); Instanzen ueber vorhandene Factories
  (`createModule(def)` fuer Items, Effekt-Literale fuer Events/Upgrades).
- Headless screenshotten: kurzlebiger Vite-Dev-Server + Playwright
  `fullPage`. In Chimera fertig als **`npm run gallery`**
  (`scripts/gallery.mjs` → `docs/visual-gallery.png`, `src/devPreview/`).
- Erst selbst ansehen (`Read` aufs PNG), dann ueber den aktiven Kanal
  schicken ([1.9](#19-auf-demselben-kanal-antworten)) mit knapper Legende.
- Wegwerf-Einmal-Previews wieder loeschen; das reproduzierbare Board bleibt.
- Ergaenzt — ersetzt NICHT — den finalen In-Game-Pilot bei
  Interaktions-/Feel-Fragen ([7.6](#76-pilot-deploy-vor-massen-rollout--bei-kritischen--sichtbaren-änderungen-seit-2026-07-01) / Skill 54).

Siehe Skill [55](skills/55-vorschau-bilder-fuer-visuelles-feedback.md).

---

### 2. Planung und Scope

#### 2.1 Scope vor Implementierung festzurren

**Regel.** Vor der ersten Codezeile: schreibe dir selbst auf (oder sag
dem Auftraggeber), was zu dieser Aufgabe gehoert. Wenn bei der Umsetzung
Scope-Erweiterung aufkommt („das sollten wir auch gleich saeubern"),
kein Ja vor Rueckfrage.

**Warum.** Scope-Creep erzeugt grosse, schwer reviewbare Diffs,
verwaessert den Fix mit unverwandten Aenderungen und macht git-blame
nutzlos.

**Beispiel.** Aufgabe: „Fix Bug X". Waehrend du den Bug liest, faellt
dir ein unuebersichtliches Modul auf. Nicht mit-refactorn — entweder
anschliessend als separate PR oder erst nach Rueckfrage.

#### 2.2 Mehrphasige Aenderungen in separate Commits

**Regel.** Nicht-triviale Refactors in Phasen teilen — jede Phase ein
Commit, getestet + build-green einzeln. Keine Batch-Commits mit mehreren
unabhaengigen Aenderungen.

**Warum.** Revert-Granularitaet + Review-Lesbarkeit. Ein bisect findet
den Fehler leichter in 10 kleinen Commits als in 1 grossen. Getrennte
Phasen zwingen dich zu ueberlegen, ob die Zwischenschritte wirklich
lauffaehig sind.

**Beispiel.** Ein groesseres Refactor „Data Model X abloesen": Phase 1 =
neue Typen + Loader + JSON-Migration. Phase 2 = Runtime umstellen.
Phase 3 = UI nachziehen. Jede Phase fuer sich gruen.

#### 2.3 Major-Dependency-Upgrades isoliert

**Regel.** Keine Major-Version-Bumps einer Dependency im Rahmen eines
anderen Tasks oder im `npm update`-Sweep. Jeder Major ist sein eigener
Task mit eigenem Regressionsfenster.

**Warum.** Major-Bumps bringen Breaking Changes, die Debugging-Zeit
kosten. Mit anderem Task vermischt wird der Fehler unauffindbar.

**Beispiel.** TypeScript 5 → 6 gehoert in einen eigenen Tag mit
`strict`-Check, nicht in einen Bug-Fix-Commit.

#### 2.4 Korrektere Variante statt Quick-and-Dirty

**Regel.** Bei Refactor-Scope-Entscheidungen den **substantiellen Pfad**
waehlen, nicht das Scaffolding-Minimum. Wenn zwei Varianten zur Auswahl
stehen — eine „macht es richtig", die andere „setzt nur das Gerippe",
ist die richtige Variante das Default, sofern Aufwand und Risiko
vertretbar sind.

**Quick-Fixes brauchen explizite User-Freigabe** (User-Spec msg 10982,
2026-05-31): wenn (b) gerechtfertigt scheint, fragt der Assistent
zurueck — er trifft die Entscheidung nicht stillschweigend. Konkrete
Form: „hier waere ein Quick-Fix moeglich der X umgeht — soll ich, oder
soll ich die richtige Variante umsetzen?" und auf Freigabe warten.
Ausnahme: die unten genannten Faelle (Spike, Hot-Fix, reine Migration)
darf der Assistent ohne neue Freigabe als Quick-Fix umsetzen — der
User-Auftrag muss aber erkennbar einen dieser Faelle anvisieren.

**Warum.** Scaffolding-Loesungen haben die Eigenschaft, dauerhaft zu
bleiben. „Quick-and-Dirty jetzt, sauber spaeter" wird in der Praxis
selten nachgezogen, weil der naechste Druck aus einer anderen Richtung
kommt. Ein bisschen mehr Zeit jetzt erspart einen Refactor in 6 Monaten
plus die Begleiterscheinungen (mehrere Stellen muessen umgezogen werden,
Tests anders, Konsumenten geaendert). Die Freigabe-Pflicht zwingt den
Assistenten, die echte Kosten/Nutzen-Rechnung offen zu legen statt sie
wegzukapseln — und schliesst die Pattern „erst Quick-Fix einbauen,
dann nachschieben muessen" aus, das in dieser Codebase schon mehrfach
zu Korrektur-Iterationen gefuehrt hat (zuletzt pulseMod-Display-Layer-
Quick-Fix vor msg 10982).

**Wann _doch_ Minimum.** Bei tatsaechlich exploriertem Spike-Code, der
explizit als Throwaway markiert ist und auch wieder geloescht wird (nicht
in main gemergt). Oder bei Hot-Fix unter Zeitdruck — dann mit
ausdruecklicher Folge-Story.

**Beispiel.** Bei einer Property-Bag-Erweiterung sind zwei Varianten
moeglich: (a) neues Feld direkt in `VALUE_KEYS` aufnehmen und im
Resolver mit allen Regeln integrieren, oder (b) Sonderbehandlung im
Caller. (a) ist substantiell, (b) ist Scaffolding. (a) waehlen, wenn
nicht ein expliziter Grund (b) rechtfertigt.

#### 2.5 Neues Item-Kind: Pflicht- und Optional-Touch-Set

**Wann.** Auftrag verlangt ein neues Item im `ITEM_CATALOG` — egal ob
Modifier-Sub-Modul oder Main-Item (Producer/Consumer/Storage).

**Regel.** Touch-Set nach Klasse abarbeiten. Niemals Catalog-Eintrag ohne
i18n-Labels committen (Memory `feedback_chimera_i18n_rule`).

**Pflicht (gemeinsam, alle Items):**
1. `src/types/base.ts` — `ItemKind`-Union erweitern.
2. `src/itemCatalog.ts` — `ItemDef`-Eintrag mit Pflichtfeldern (`kind`, `icon`, `subSlotCount`, `category`, `color`, `botRole`, `maxCables`). **Kein `label`-Feld** — der user-facing Name lebt seit 2026-06-05 ausschliesslich in i18n (`items.<kind>`, via `getItemLabel`).
3. `src/i18n/de_DE.ts` + `src/i18n/en_US.ts` — `items.<kind>` = der Item-Name (Single-Source) + optional `help`.
4. `docs/items.md` — narrativer Eintrag + danach `npm run docs:items` fuer den AUTO-GENERATED-Block (Plan 2026-06-02-propertybag-modularisation Teil B).
5. `npm run generate:norms` — bei JEDER Katalog-Property-Aenderung (auch an bestehenden Items, z.B. neue Kategorie), sonst reisst `propertyNormFactors.test.ts` im CI (Vorfall 2026-07-11: nova → damage-Cat → intrinsischer Trait wechselte).
6. `src/simulation/rlObservation.ts` — neuen Kind ans ENDE von `KIND_ORDER` appenden (append-only!) + Laengen-Waechter in `rlObservation.test.ts`. Der Waechter laeuft NUR in der Sim-Suite, nicht in `npm test` — nach dem Append explizit `npx vitest run --config vitest.simulation.config.ts src/simulation/rlObservation.test.ts` fahren (zweimal verpasst: nova/firewall 07-12, catalyst/harvester 07-17).

**Modifier-spezifisch:**
- `subSlotCount: 0` (oder 1 wenn er selbst Sub-Mods nimmt), `botRole: "modifier"`, kein Energy-Profil noetig.
- `modifierMultiplierEffect` und/oder `modifierValueEffect` mit `output`/`cost`/`stealth`/`damage`/`health`/`experience`/`spatial`/`shield`/`diffusion` als Zahl oder `"invert"`.
- Falls Sondermathematik (kontext-/zeit-abhaengig): `src/itemHooks/<kind>.ts` als Hook (heatExtractor-Pattern).
- Falls Container den Kind-Beitrag N-fach anwendet: `hooks: { childApplyCount: 2 }` (duplicator-Pattern, transparente Expansion).

**Main-Item-spezifisch:**
- `botRole: "container"` (Consumer) oder `"producer"`, eigenes Energy-Profil in `src/energyProfiles.ts`.
- `categories: { output: [...], cost: [...], stealth: [...], health: [...], experience: [...] }` — listet die VALUE_KEYS pro Cat (`output: ["production"]` fuer Generator, `cost: ["demand", "heat"]` fuer Engine).
- `drawDecoration` in `src/renderer/itemDecorations.ts` (fast immer noetig).
- `defaultMaxHP` falls Item HP/Zerstoerung haben soll (Standard 100 via DEFAULT_MAX_HP-Fallback).
- `damage`/`damageRate`/`visualEffect: "laser"` fuer Bolt-Spawner.
- `rootOnly: true` fuer Items die nicht in Sub-Slots passen (infiltrator).
- `defaultThreshold` + `src/probeTypes.ts`-Eintrag fuer Diagnose-Items (auto-derived DIAGNOSTIC_KINDS).
- Save-Format-Bump in `src/persistence.ts` NUR wenn neues PropertyBag-Feld noetig (selten).

**Auto-Derived Konstanten** (KEINE manuelle Pflege noetig): `VALID_LOCK_KINDS`, `DIAGNOSTIC_KINDS`, `DEFAULT_THRESHOLDS`, `CONTAINER_KINDS`, `PRODUCER_KINDS`, `MODIFIER_KINDS` werden aus dem Catalog generiert. Korrektes `botRole`/`category`/`lockable` setzen reicht.

**Workflow.**
1. Catalog + Types + i18n in einem Commit.
2. Decoration + Energy-Profil im gleichen Commit.
3. Hooks/Codex falls noetig.
4. Tests in `src/__tests__/`.
5. `npm run docs:items` fuer SSOT-Regeneration.
6. Build + Deploy + Spieltest.

**Aufwand.** Historische Beobachtungswerte: pure Modifier 3 Files ~1 h;
Main-Item ohne Sondermathematik 5-6 Files ~2-3 h; Main-Item mit eigener
Sub-Mechanik 8-12 Files, gehoert in einen Plan-Doc. Fuer Plan-Schaetzungen
gilt die gelebte Faktor-Regel aus Skill
[45-aufwand-schaetzung-kalibrieren.md](skills/45-aufwand-schaetzung-kalibrieren.md)
(Refactor ×0.10, Feature ×0.25) — nicht diese Absolut-Zahlen fortschreiben.

**Warum.** Items-Hinzufuegen ist ein wiederkehrender Aufgabentyp mit hoher
Drift-Anfaelligkeit (i18n vergessen → UI-Fallback, energyProfile vergessen
→ Item produziert nix, categories-Map vergessen → overclocker wirkt nicht,
botRole falsch → Bot-Heuristik verwirrt). Checkliste vermeidet stille
Fehler die erst beim Spieltest auffallen. Volldetails in
[docs/skills/46-neues-item-hinzufuegen.md](skills/46-neues-item-hinzufuegen.md).

#### 2.6 Letzte Plan-Phase = Refaktorierungs-Audit mit Korrektur

**Wann.** Jeder Mehr-Phasen-Plan in `docs/plans/`.

**Regel.** Die abschliessende Phase jedes Plans ist ein **Refaktorierungs-
Audit + Korrektur** ueber die **gesamte** im Plan umgesetzte Implementierung —
geprueft gegen Architektur und Redundanz, gefundene Verbesserungen werden in
derselben Phase **umgesetzt** (nicht nur notiert). Wird beim Planschreiben
bereits als letzte Phase mitgeplant; ein Plan gilt nicht als „Umgesetzt",
bevor diese Phase durch ist.

**Audit-Achsen.** Redundanz/DRY (gleiche Logik ueber mehrere Plan-Commits →
geteilter Helper/Typ/Konstante), Architektur (Sonderfaelle die ein gemeinsames
Muster verdecken, Handler-Map statt Switch, pure Reducer, zu grosse
Funktionen), Konsistenz (Naming, Layer-Verortung, Schnittstellen-Form),
tote Reste (Shims, ungenutzte Exporte, Scaffolding).

**Scope.** Nur was DIESER Plan angefasst hat — nicht die ganze Codebasis
(dafuer der Subagent-Audit aus
[docs/skills/47-architektur-audit-mit-subagents.md](skills/47-architektur-audit-mit-subagents.md)).

**Warum.** Ueber mehrere Phasen entstehen lokal sinnvolle, in Summe aber
suboptimale Strukturen (duplizierte Logik, inline-Unions die zentral
gehoeren, verdeckte Muster). Im Gesamt-Blick am Plan-Ende — Suite gruen,
Verhalten verifiziert — ist Konsolidierung am billigsten und sichersten.
Volldetails in
[docs/skills/49-plan-abschluss-refaktorierungs-audit.md](skills/49-plan-abschluss-refaktorierungs-audit.md).

#### 2.7 Spielregel-Semantik: eine lernbare Regel, keine Sonderfaelle (seit 2026-07-05)

**Regel.** Spielmechanik-Semantik-Fragen (was BEDEUTET ein Wert/Modifier
auf einem Item?) werden ERST per Telegram diskutiert, DANN implementiert —
und die Loesung muss eine GLOBALE, fuer Spieler lernbare Regel sein, keine
Per-Item-Sonderfaelle. Litmus: *Kann ein Spieler die Regel aus einem Satz
lernen und auf alle Items anwenden?* (User-Spec msg 14116.)

**Warum.** Ein Item, das eine deklarierte Mechanik stillschweigend anders
interpretiert, bricht das mentale Modell der Spieler. Beispiel: fan(inverter)
sollte saugen — die Sonderfall-Loesung (`inverterDirectionCategories`-Flag)
wurde revertiert; richtig war die globale Vorzeichen-Regel (Inverter macht
Werte echt negativ, jedes Geraet interpretiert das Vorzeichen physikalisch).
Volldetails in
[docs/skills/56-spielregel-semantik-ohne-sonderfaelle.md](skills/56-spielregel-semantik-ohne-sonderfaelle.md).

---

#### 2.8 Katalog-/Config-Audits: Block-Extraktion statt Einzeilen-Grep (seit 2026-07-11)

**Regel.** Aussagen ueber ALLE Eintraege einer Katalog-/Config-Datei
(Klassifikationen, "welche Items haben X?") nie per Einzeilen-Regex
gewinnen — Eintraege koennen mehrzeilig sein und fallen still raus.
Block-Extraktion (Anker bis naechster Anker) oder gleich die Struktur
laden; bei Wirkungs-Audits zusaetzlich `propertyBag/reader.ts`
(Kontext-Effekte), `itemHooks/` und `energyProfiles.ts` pruefen.
Extraktion mit 2-3 bekannten Faellen gegenpruefen. Vorfall: falsche
Q3b-Restliste im XP-Level-Plan (msg 14802). Details in
[docs/skills/58-block-extraktion-statt-einzeilen-grep.md](skills/58-block-extraktion-statt-einzeilen-grep.md).

---

#### 2.9 User-gegebene Objekte: ganz uebernehmen oder frisch ersetzen (seit 2026-06-12)

**Regel.** Gibt der User konkrete Items/Builds/JSON-Vorlagen zum Einbetten
oder Transformieren, dann entweder den KOMPLETTEN Property-Satz durchreichen
oder eine frische Instanz desselben Typs bauen — nie selektiv Properties
strippen (nur eindeutig runtime-lokale Felder wie `currentHP` duerfen weg).
Explizit leere Sub-Slots `{ "module": null }` in Vorlagen 1:1 erhalten —
Auto-Pad greift nur fuer Container-Kinds.

**Warum.** Partielles Uebernehmen erzeugt einen Frankenstein-Zustand
(User-Spec msg 12100): 2026-06-12 hat `xp`-Strippen die vom User ueber
Item-Level austarierte Energie-Balance gebrochen; 2026-05-08 fehlte ein
explizit leerer Generator-Slot in der Library. Details in
[docs/skills/62-user-objekte-ganz-oder-frisch.md](skills/62-user-objekte-ganz-oder-frisch.md).

### 3. Code-Qualitaet

#### 3.1 Editiere bestehende Dateien, erzeuge keine neuen leichtfertig

**Regel.** Wenn eine Aenderung in eine bestehende Datei passt, geh dort
hinein. Neue Dateien nur bei neuen konzeptuellen Einheiten.

**Warum.** Datei-Proliferation erschwert Navigation. Die erste Frage
sollte lauten: „Wo ist der natuerliche Platz?", nicht „Wo erzeuge ich
eine neue Datei?".

#### 3.2 Trenne Pure Reducer von Stateful Hooks

**Regel.** Business-Logik in reinen Funktionen (deterministisch, keine
Seiteneffekte) — IO / State / Timing in Hooks oder Thin Wrappers. Hook
delegiert an Reducer, nicht umgekehrt.

**Warum.** Pure Funktionen sind trivial testbar und komponierbar. Hooks
lassen sich schwer testen; wenn Hooks Logik enthalten, haengt die Logik
an React-Rendering.

**Beispiel.** Reducer `advanceToNextSection(state, runDef, gameTimeMs)`
hat keine Seiteneffekte — ein Hook ruft ihn via `setGameState(gs =>
advanceToNextSection(gs, …))` auf.

#### 3.2a Grosse Hooks: Tick-Sub-Funktionen als pure Module extrahieren

**Regel.** Sobald ein Hook mehrere hundert LOC ueberschreitet und in der
zentralen Tick-Funktion 5+ inline-Sub-Funktionen liegen (`applyXXX()`),
ziehe diese in `src/<bereich>/<concern>Tick.ts` als pure Funktion
(`runXXXTick(input)`) raus. Der Hook bleibt Orchestrator: Setup-Effekte,
HeatManager-/State-Refs, Reihenfolge der Aufrufe. Jede extrahierte
Funktion bekommt **alle Deps** (HM-Refs, Charge-Maps, Schild-Maps,
Callbacks) als Input-Objekt — keine versteckten Closure-Captures.

**Warum.** Inline-Sub-Funktionen erben den gesamten Closure-State der
tick()-Funktion und sind dadurch nicht isoliert testbar; das Hauptziel
des Hooks (was wird wann gerufen) verschwindet hinter der Sub-Logik.
Pure-Modul-Extraktion macht jeden Tick-Pfad einzeln testbar, reduziert
die Hook-LOC drastisch und erzwingt explizite Dependency-Listen, die
versteckte Kopplungen sichtbar machen.

**Beispiel.** `useHeatSimulation` war 982 LOC mit 7 inline-Sub-Funktionen
(applyEmergencyShutdown, applyExtremeTemperatureDamage,
applyFieldLockDisplacement, applyThermoInjections, applyRepairPaste,
applyShooterBolts, applyLaserCutters). Nach Extraktion in
`src/run/*Tick.ts` ist der Hook 637 LOC; jede Sub-Funktion hat ein
typisiertes Input-Interface, schliesst keine Refs mehr ein und ist
isoliert testbar. tickMitigateCombined wandert als Callback durch die
Input-Objekte — beim Refactor wurde explizit, welche Pfade Schild-
Mitigation brauchen.

**Reihenfolge.** Eine Extraktion pro Commit, jeweils mit gruenen Tests.
Nicht batch — der Reducer-Verkehr (Reihenfolge der Tick-Schritte ist
semantisch wichtig) wird sonst undurchsichtig. Bei jeder Extraktion
unbenutzte Imports und nun toter Code (Konstanten, Closures) im Hook
mit aufraeumen.

#### 3.3 Keine Magic Values

**Regel.** Zahlen / Strings mit Bedeutung als Konstanten mit Namen,
nicht inline. Schwellenwerte, Timeouts, Einheiten — alles benannt.

**Warum.** Benannte Konstanten sind Dokumentation. `const
TASK_START_DELAY_MS = 5000` erklaert sich selbst; `5000` in einem
Timer-Aufruf nicht.

#### 3.3a Konstanten zentralisieren, sobald sie thematisch verwandt sind

**Regel.** Wenn drei oder mehr Konstanten zum gleichen Subsystem gehoeren
(Heat-Sim, Energy-Pool, Cable-Physics, Tutorial-Run-Timing) und ueber
mehrere Dateien wandern, sammle sie in einer dedizierten
`<bereich>Constants.ts`-Datei. Feature-spezifische UI-/Timing-Konstanten
(`BEAM_FLASH_COOLDOWN_MS`, `FIELD_LOCK_SNAP_DURATION_MS`) bleiben in
ihren Feature-Modulen — sie sind keine Sim-Parameter und gehoeren zur
jeweiligen Logik.

**Warum.** Versprenkelte Konstanten driften: zwei Stellen halten dasselbe
Konzept mit minimal unterschiedlichen Werten, oder die Doku zitiert
einen Wert, der so nirgends mehr existiert. Ein zentraler Sammelpunkt
macht Game-Mechanik-Tuning lokal und vereinfacht Doku-Verweise.

**Beispiel.** `src/heatConstants.ts` haelt `HEATMAP_DIFFUSION_RATE`,
`BORDER_COOLING_EXTRA`, `ITEM_HEAT_DURATION_MS`, `CABLE_HEAT_FULL_FLOW`,
`ZONE_HEAT_DAMAGE_THRESHOLD`. Vorher waren diese in `heatPhysics.ts`,
`useHeatSimulation.ts`, `thermoInjectionTick.ts` und
`extremeTemperatureTick.ts` verteilt; `docs/heat-system.md` referenziert
sie als Spielmechanik-Parameter — nur sinnvoll mit einem Standort.

#### 3.3b Balance-/Content-Werte datengetrieben, nicht hartverdrahtet (seit 2026-06-24)

**Regel.** Ist ein Wert oder Verhalten etwas, das **Autoren/Spieler pro Content
tunen** sollen (Zonen-Boni, Boss-/Run-Parameter), gehört er ins **Content-Schema
(JSON)**, nicht in eine Code-Konstante — auch keine zentrale `…Constants.ts`. Der
Solver liest den Wert aus den Daten; Code hält nur die Mechanik, nicht den
gewünschten Wert.

**Warum.** Eine Code-Konstante zwingt für jede Balance-Änderung einen Build +
Deploy und macht pro-Chassis/-Boss-Varianz unmöglich. Beispiel (msg 12940): der
Torso-Produktions-Bonus war `TORSO_PRODUCTION_BONUS = 1.5` hart im Solver. Umbau
auf `ZoneBonusEntry.productionMultiplier` / `BossZoneDef.productionMultiplier` →
pro Zone/Chassis/Boss frei einstellbar, der Solver bekommt eine `zoneProductionMul`-
Map. Der frühere Arm-„Bonus" (×1,5 Verbrauch) war faktisch eine Strafe und wurde
ersatzlos entfernt — hartverdrahtete „Boni" entziehen sich der Sichtbarkeit + dem
Tuning. Abgrenzung zu 3.3a: reine Sim-/UI-Timing-Konstanten bleiben im Code;
gemeint sind **Authoring-/Balance-Größen**.

#### 3.3c Abgeleitete Listen/Werte generieren statt duplizieren (gegen Drift, seit 2026-06-30)

**Regel.** Eine Liste oder ein Wert, der sich aus einer **Source-of-Truth ableiten
lässt** (vorhandene Dateien im Build-Output, `package.json`, installierte Versionen),
wird zur **Build-Zeit generiert**, nicht von Hand an einer zweiten Stelle gepflegt.
Generiertes File eingecheckt (damit Dev ohne Build funktioniert) UND im `npm run build`
neu erzeugt (immer frisch). Muster: `src/generated/propertyNormFactors.ts`,
`scripts/generate-*.ts`.

**Warum.** Jede handgepflegte Kopie driftet von ihrer Quelle weg. Beispiele
(2026-06-30, je nach einem User-Report):
- **Run-Liste:** `NewRunScreen` probierte eine hartkodierte `ALL_RUN_IDS`-Liste per
  `fetch` durch → 404-Blind-Proben für gestrippte Runs im Public-Build. Fix: ein
  Build-Manifest (`runs/index.json`, `generate-run-manifest.ts`) listet die
  tatsächlich ausgelieferten Runs → es wird nur geladen, was existiert.
- **Credits-Lizenzliste:** die hartkodierte Tool-/Versionsliste war zu 6/10 veraltet
  + unvollständig (Playwright/Fonts fehlten). Fix: `generate-licenses.ts` liest
  `package.json` × `node_modules` → Versionen, Lizenzen und Lizenztexte können nicht
  mehr veralten.

**Abgrenzung.** Bewusst statische Werte (z. B. das README-Test-Count-Badge, User-Wunsch
2026-06-23) bleiben hand-gepflegt — das ist eine Entscheidung, kein Drift-Bug. Gemeint
sind **ableitbare** Daten, deren Hand-Kopie nur eine Fehlerquelle ist. Folge-Falle: was
generiert/gestrippt wird, darf die App nicht an anderer Stelle hart referenzieren
(siehe 7.4). Skill: [53-generieren-statt-duplizieren.md](skills/53-generieren-statt-duplizieren.md).

#### 3.4 Kommentare erklaeren das „Warum", nicht das „Was"

**Regel.** Kein Kommentar, der die Code-Zeile paraphrasiert. Kommentare
fuer verborgene Invarianten, Workarounds, Design-Entscheidungen, die
dem Leser ohne Kontext nicht klar waeren.

**Warum.** Der Code sagt was er tut; der Kommentar soll sagen, warum.
„Increments counter" ist Laerm; „Retry-Zaehler — dient dem Jitter im
Reconnect-Flow (Issue #412)" ist Kontext.

#### 3.5 Vertraue Framework-Garantien, validiere nur an Systemgrenzen

**Regel.** Keine defensive Programmierung fuer Szenarien, die das
Framework ausschliesst. Validation nur an Aussengrenzen (User-Input,
externe APIs, File-Parser).

**Warum.** Ueberdefensiver Code verbirgt die tatsaechliche
Geschaeftslogik unter Null-Checks und Try/Catches. Wenn der Compiler
oder das Framework garantiert, dass X nicht passiert, verschwende keine
Zeile darauf.

**Beispiel.** In einem TypeScript-Typ `{ foo: string }` keinen `if
(typeof obj.foo !== "string")`-Check einbauen.

#### 3.6 Keine Backward-Compatibility-Shims beim Refactoring

**Regel.** Wenn du etwas ersetzt, entferne das Alte vollstaendig. Kein
`_unused` leftover, kein `// removed`-Kommentar, kein Re-Export der
alten API „fuer den Fall, dass". Altcode rottet.

**Warum.** Dead Code ist schlimmer als kein Code: er wirkt wie
lebender, schickt Leser in falsche Richtungen.

**Ausnahme.** Oeffentliche APIs und persistierte Daten mit externen
Consumern brauchen oft Migrationen / Deprecation-Pfade. Interner Code
nicht.

#### 3.6a Keine Redundanzen / kein doppelter Code (seit 2026-05-31)

**Regel.** Gleiche Logik kommt **einmal** vor — was sich an mehreren
Stellen wiederholt, wandert in einen geteilten Helper, eine Funktion oder
eine Konstante (zentral, siehe [3.3a](#33a-konstanten-zentralisieren-sobald-sie-thematisch-verwandt-sind)).
Auch 3-5 Zeilen lohnen die Extraktion, BEVOR die zweite Kopie committet
wird. Grenze: gleiche **Bedeutung**, nicht gleiche Optik — semantisch
verschiedene Aehnlichkeiten nicht prematur abstrahieren.

**Warum.** Duplikate divergieren: das Burst-Window-Muster
`(now - fireTime) < THRESHOLD` lebte an drei Stellen (heatPhysics,
instanceStatRows, engine); beim Umzug der Zeitbasis auf gameTimeMs wurde
nur eine umgestellt — die anderen kollabierten still, der Bug fiel erst
dem User auf (msg 10991). Anwendungs-Checkliste, Smell-Test
(„muss ich bei einer Aenderung eine zweite Stelle anfassen?") und
Ausnahmen: Skill [44-keine-redundanzen.md](skills/44-keine-redundanzen.md).
Quelle: User-Spec msg 11002 (2026-05-31).

#### 3.7 Discriminated Unions: Handler-Map statt verteilter `kind`-Switches

**Regel.** Sobald eine diskriminierte Union (`kind`/`type`/`action`) an mehr
als zwei Call-Sites auf den Discriminator verzweigt, wird das Verhalten in
eine **Handler-Map** ausgelagert: ein Handler-File pro Variante, zentrale
`Record<Kind, Handler>`-Map mit TS-erzwungener Vollstaendigkeit, Caller
dispatchen ueber `HANDLERS[e.kind].method?.(...)`. Das Pattern ist die
Default-Wahl — neue Discriminated Unions werden direkt so angelegt, nicht
erst nach dem dritten Switch refactored.

**Warum.** Verteilte `kind`-Switches muessen bei jeder neuen Variante alle
gefunden und ergaenzt werden — ein Vergessen wird oft erst zur Laufzeit
sichtbar; die Map macht es zum Compile-Error und buendelt das Wissen pro
Variante in einem File. Drei Chimera-Refactors (NPC_HANDLERS, TASK_HANDLERS,
EFFECT_HANDLERS) haben so ~140 verstreute Switch-Stellen aufgeloest.

Bauanleitung (vier Konventionen), Gegenanzeigen (Parser/Loader,
Type-Narrowing-Guards, einzelne lokale Lookups), Hilfsregeln und
Mini-Skelett: Skill
[21-handler-map-pattern.md](skills/21-handler-map-pattern.md).
Live-Beispiele: `src/run/effectHandlers/`, `src/run/taskHandlers/`,
`src/run/npcHandlers/`.

#### 3.8 i18n-Pflicht fuer user-facing Texte (seit 2026-05-22)

**Regel.** Jeder neue user-facing Text muss i18n-faehig sein. Zwei Pfade:
im TS/TSX-Code definierter Text ueber `t()` aus `useTranslation()` (bzw.
`getStaticTranslation` ausserhalb React) mit Keys in `src/i18n/en_US.ts`
UND `de_DE.ts`; in JSON definierter Text als `LocalizedString`
(`{ de_DE, en_US }`; Loader `parseLocalizedFromAny`, Display
`resolveLocalized`). Verboten: hardcoded UI-Literale, DE-Fallback per `??`,
fehlende Locale-Variante.

**Warum.** Jeder durchgerutschte String ist ein Bug im EN-Locale, den der
User bisher per Screenshot fangen musste („[object Object]", TaskHUD
„ODER", „Hitzeschaden") — vor dem Commit pruefen statt nachziehen.
Volldetails (JSON-Dateiliste, Ja/Nein-Beispiele, Migrator- +
Translation-Skripte): Skill [22-i18n-pflicht.md](skills/22-i18n-pflicht.md);
Architektur: `docs/architecture.md` → „Internationalization".

#### 3.9 Design-Invarianten kennen und respektieren — nicht als Bug fixen

Reports der Form „X wirkt nicht auf Y" / „der Wert pulst/skaliert nicht wie
erwartet" erst gegen die **bewussten System-Invarianten** halten, bevor man
an der Engine schraubt. Was isoliert wie ein Fehler aussieht, ist oft
gewollte Semantik.

Bekannte Chimera-Invarianten (Stand 2026-06-13):

- **Boss-Event-Effekte ohne Dauer (`durationMs`/`recoveryMs` = null) sind
  PERMANENT — gewollte Engine-Semantik, kein Bug.** Auf einem Loop-Boss
  (`loopTimeMs`) feuert das Event jeden Zyklus neu und die Instanzen STAPELN
  (kein Replace). Ein akkumulierendes Hazard (fieldLock sperrt pro Loop weitere
  Felder, zoneCompression kollabiert die Zone) ist eine VERGESSENE Dauer im
  Boss-JSON, kein Engine-Fehler. Warnexempel 2026-06-13: ich baute auf Verdacht
  einen „Replace-on-Refire"-Engine-Fix (fieldLock, dann fälschlich auf
  zoneCompression verallgemeinert) — vom User komplett revertiert, der echte Fix
  war `durationMs:6000` im JSON. Zusatzfehler: das Lebensdauer-Feld ist je Kind
  verschieden (fieldLock `durationMs`, **zoneCompression `recoveryMs`**) — mein
  Scan prüfte nur `durationMs` und las Compressions falsch als permanent. Lehre:
  erst Design-Intent + Daten (fehlendes Feld?) prüfen, nicht die Engine; und das
  RICHTIGE Feld pro Kind nachschlagen.
- **Modifier wirken nur Child→Parent, nie auf Siblings.** Ein Sub-Modul
  modifiziert seinen Parent, nicht seine Geschwister im selben Container.
  `mux(pulseMod, repairPaste)` lässt die Reparatur NICHT pulsen (Siblings) —
  die pulsende Komposition ist `repairPaste(pulseMod)` (Nesting). Technisch:
  Two-Pass-Aggregation in `src/propertyBag/resolve.ts` + Owner-Scope (nur der
  Kategorie-Besitzer bekommt Child-Multiplier auf seine Werte). User-
  Entscheidung msg 11370: „childs wirken nur auf parents aber nicht auf
  siblings" — die Engine-Aenderung wurde explizit abgelehnt.
- **`inverterMod` ist kategorie-selektiv und wird vom ersten Vorfahren mit
  passendem Kategorie-Faktor ODER `categories`-Deklaration konsumiert.** Er
  kippt die Werte/Multiplier genau EINER Kategorie. Container OHNE Faktor/
  Deklaration in der Ziel-Kategorie sind transparent — der Inverter propagiert
  durch sie nach oben. **Ein Item mit `categories`-Deklaration ist eine
  Absorptions-Grenze (Owner-Scope, `phaseC` `hostOwnsCategories`):** es
  absorbiert ALLE verbleibenden Inverter-Counts → der Inverter erreicht den
  Parent nicht mehr. Verifiziert: `engine(duplicator(inverterMod))` → speed 0
  (durch den cat-losen duplicator propagiert), aber `engine(sensor(inverterMod))`
  → speed 50 (sensor hat `categories` → absorbiert). Folge fuer Item-Bauer: soll
  ein Inverter eine Host-VALUE kippen (cooler/heatSink: heatCooling→heat), muss
  er DIREKTES Kind des Hosts sein; eine `categories`-Map auf einem Zwischen-Item
  macht es bewusst zur Inverter-Grenze. Detail-Skill:
  [46-neues-item-hinzufuegen.md](skills/46-neues-item-hinzufuegen.md) →
  „Inverter-Propagation".
- **Drag darf nie die Sim aendern oder blockieren** (Sim-Output identisch zu
  nicht-gedragged).

Vorgehen: Pfad empirisch nachstellen (Wegwerf-Dump-Test, der
`resolveTree(...)` / `buildInstanceStatRows(...)` fürs gemeldete Setup
loggt), gegen die Invariante halten, erst dann Bug-vs-Intended entscheiden.
Wenn Intended → kein Fix, sondern Diagnose + korrekte Komposition empfehlen;
eine echte Engine-Aenderung als bewusste Design-Frage mit Blast-Radius
benennen (siehe Skill
[48-design-invarianten-respektieren.md](skills/48-design-invarianten-respektieren.md)).

---

#### 3.10 Browser-geteilte Module: kein `require` — `process.getBuiltinModule` (seit 2026-07-11)

**Regel.** Module, die Browser UND Node/Sim teilen, duerfen fuer synchrone
Node-APIs (`node:fs` etc.) kein `require` nutzen — unter ESM/tsx liefert das
still `null` und der Sim-Pfad verliert die Funktionalitaet, waehrend vitest
(CJS-interop) es verdeckt: **Unit-gruen ≠ Sim-gruen**. Stattdessen den
geteilten Helfer (`getNodeSyncFs` via `process.getBuiltinModule`) verwenden
und den Sim-Pfad (`npm run simulate`-Smoke) separat verifizieren. Siehe
Memory `project_chimera_esm_require_getbuiltinmodule`.

---

### 4. Testing

#### 4.1 Tests begleiten Features und Fixes

**Regel.** Ein neuer Feature-Commit bringt Tests fuer die neue Logik.
Ein Bug-Fix bringt einen Regression-Test, der ohne den Fix fehlschlaegt.

**Warum.** Ohne Tests weisst du nur „funktioniert gerade", nicht
„bleibt korrekt". Bug-Regressions-Tests dokumentieren zudem, was das
gemeldete Problem war.

#### 4.2 Erklaere + frage, bevor du bestehende Tests veraenderst

**Regel.** Tests werden nicht einfach „angepasst, damit sie gruen sind".
Ein existierender Test hat einen Grund. Wenn eine Aenderung ihn zum
Scheitern bringt: (a) zuerst ueberlegen, ob die Aenderung wirklich das
dokumentierte Verhalten brechen soll. (b) Wenn ja, die Aenderung erklaeren
und — bei echten Verhaltensregeln — mit dem Auftraggeber abstimmen.

**Warum.** Tests sind kodifizierte Anforderungen. Einen Test zu
aendern, um einen Build zu reparieren, loescht oft genau die Invariante,
die der Test schuetzt.

#### 4.3 Greene Suite vor jedem Commit

**Regel.** `tsc --noEmit`, die komplette Test-Suite, **die e2e-Tests
(`npm run e2e`, Playwright — laufen NICHT in `npm test` mit!)**,
**`npm run lint`** und der Production-Build laufen gruen, bevor ein
Commit gemacht wird. Wenn einer rot ist: fixen, nicht committen.
(e2e ergaenzt seit 2026-06-07, User-Spec msg 11609; lint ergaenzt seit
2026-07-06 nach CI-Fail msg 14129 — ein einziger eslint-ERROR, auch in
Test-Dateien, laesst den GitHub-Workflow fehlschlagen; Bestands-Warnings
sind ok. Siehe Memory `feedback_e2e_before_commit`.)

**Warum.** Rote Commits verbrennen Zeit fuer jeden, der danach checkt
out. Bisect bricht. CI-Credits werden verschwendet. e2e lief frueher
out-of-band und blieb unbemerkt rot (veraltete Save-Version) — daher
ab jetzt Teil des Gates bei App-Code-Aenderungen.

**Beispiel-Sequenz:**
```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e && npm run build
```
Bei reinen Doku-/Test-Datei-Commits ohne App-Code-Aenderung kann `npm run e2e`
entfallen. Vermeide ausserdem in e2e-Tests Werte, die mit der Source driften
(z.B. `CURRENT_SAVE_VERSION` zur Laufzeit aus `src/persistence.ts` lesen).

**Pipe-Exit-Falle:** `npm test | tail -3` maskiert den Exit-Code (Pipe
liefert den Status von `tail`) — ein `&&`-Gate laeuft trotz roter Tests
weiter (passiert 2026-07-05: 3 Failures unbemerkt). `set -o pipefail`
setzen oder die Summary-Zeile explizit pruefen.

**Nach einem Merge: kein zweiter Lauf ohne Aenderung** (Owner-Spec
2026-08-14). Das Gate laeuft VOR dem Merge auf dem Topic-Branch; ist
`git diff --quiet topic/<plan> dev` nach dem Merge leer (Baum identisch mit
dem getesteten Stand), wird die Suite NICHT erneut gefahren. Ist der Diff
nicht leer (fremde dev-Commits, Konflikt-Aufloesung), laeuft das volle Gate
auf dem Merge-Ergebnis. Details: Skill
[25-gruene-suite-vor-commit.md](skills/25-gruene-suite-vor-commit.md) →
„Nach einem Merge" + Skill 64 Regel 4.

#### 4.4 Verhaltens-Tests vor Struktur-Tests

**Regel.** Tests pruefen Beobachtbarkeit — was der Code tut, nicht
wie er intern aufgebaut ist. Kein Test, der einen Dateipfad oder eine
interne Klassen-Hierarchie asserten muss, um zu ueberleben.

**Warum.** Struktur-Tests brechen bei jedem harmlos aussehenden
Refactor. Verhaltens-Tests ueberleben Refactors und schuetzen echte
Invarianten.

#### 4.5 Coverage-Luecken: erreichbare Pfade testen, defensive Pfade dokumentieren

**Regel.** Beim Coverage-Lueckenschluss drei Faelle unterscheiden:
(a) **erreichbar ueber API-Aufruf** → Test schreiben; (b) **Defensiv-Guard**
gegen strukturell ausgeschlossene Faelle → entfernen oder im Commit-Body
explizit als solchen benennen; (c) **Hook-Code fuer noch-nicht-eingebautes
Feature** → per `vi.mock(... importOriginal())` mit synthetischem Item-Def
aktivieren (testbare Spec-Doku statt Loeschen).

**Warum.** 100 % Branch-Coverage fuer dead code erzeugt Mock-Theater oder
streicht Schutz-Code. Triage-Ablauf + Chimera-Beispiel (heatPhysics
91.78 % → 97.26 %, letzte 4 Branches = dokumentierte defensive Guards):
Skill [27-coverage-luecken-triage.md](skills/27-coverage-luecken-triage.md).

#### 4.5b Coverage misst Ausfuehrung, nicht Behauptung — die Mutations-Probe

**Regel.** Eine Zeile gilt als „covered", sobald irgendein Test sie durchlaeuft
— unabhaengig davon, ob ueber ihr Ergebnis je etwas assertet wird. Ein Feature
kann **100 % Coverage haben und 0 % getestet sein**. Aus einer Coverage-Zahl
darf darum NIE auf Absicherung geschlossen werden. Der einzige verlaessliche
Nachweis ist die **Mutations-Probe**: die Wirkung abschalten und schauen, ob die
Suite schreit.

| Signal | Aussagekraft |
|---|---|
| Coverage 0 % | Echte Luecke. Zuverlaessig. |
| Coverage 100 % | **Sagt nichts** ueber Absicherung. |
| Mutation ueberlebt | Echtes Loch. Zuverlaessig. |
| Mutation stirbt | Waechter existiert. Zuverlaessig. |

**Warum.** 2026-07-14: die per-Tick-XP-Vergabe (`energyStep`) war komplett
ungetestet. Mit hart deaktivierter Vergabe liefen **8482 Tests gruen durch** —
nach Monaten mit Coverage-Checks und Coverage-Verbesserungen. Der Grund ist
strukturell: die XP-Zeilen laufen in JEDEM Test mit, der die Sim tickt, sind
also bestens „abgedeckt" — und tauchen in einer Luecken-Triage (4.5) per
Definition nie auf, denn die listet nur NICHT-abgedeckte Zeilen. Das
Housekeeping war gegen diese Klasse blind by design.

**Wie.** Pro Subsystem die **beobachtbaren Ausgaben** auflisten (Sim-Kern: `xp`,
`charge`, `heat`, `itemHP`, Schild-Ladung, Fire-Timestamps, Siphon-Netto,
Waermebombe), je eine Mutation, die genau diese Wirkung loescht (Zuweisung
neutralisieren / Rumpf kappen / Setter no-op), volle Suite je Mutation, Restore
im `finally`, danach `git diff` = leer. Details + Skript-Muster:
[skills/59-mutations-probe-statt-coverage-prozent.md](skills/59-mutations-probe-statt-coverage-prozent.md).
Der Mutations-Pass ist seit 2026-07-14 fester Bestandteil des
Idle-Housekeepings (Skill 42, Pass 2).

#### 4.6 e2e-Tests (Playwright): Pyramide + Mechaniken

**Regel.** Die Test-Pyramide ernst nehmen: Effekt-/Logik-Coverage gehoert in
schnelle **Unit-/Integrations-Tests** (vitest, ~1–5 ms), e2e bleibt eine
**duenne Smoke-Schicht** fuer den UI-Pfad (Browser-Render, Klick-Flows). e2e
NICHT „massiv erweitern", um Logik zu pruefen — das ist langsam (2–5 s/Test) und
flaky-anfaellig. Pro Feature wenige repraesentative e2e-Smokes; die
Verhaltens-Breite deckt die Unit-Ebene ab (z.B. tabellengetrieben ueber alle
Varianten). Detail-Plan: `docs/plans/archive/2026-06-08-event-card-test-expansion.md`.

**Konventionen.**
- Selektoren ausschliesslich ueber `data-testid` (NICHT Text — i18n bricht
  sonst). Fehlt eine Test-Anker-id, im Komponenten-Code ergaenzen (+ ggf.
  `data-<state>` fuer Zustaende, z.B. `data-filled`).
- Canvas-Drag via `page.mouse.down()/move()/up()`, nicht `dragTo()`.
- Wartestrategie explizit (`waitForSelector`/`waitForFunction`/`toBeVisible`),
  keine fixen `waitForTimeout`-Werte (Flake-Quelle).

**Chimera-spezifische State-Mechaniken** (teuer erlernt, msg 11676):
- **Der laufende Run wird waehrend des Spielens NICHT debounce-gespeichert**
  (Tick-Starvation: der 500 ms-Auto-Save-Debounce wird durch die State-Updates
  staendig zurueckgesetzt). Persistiert wird erst ueber den `pagehide`-Handler
  beim Reload/Navigieren. Wer den laufenden Run im localStorage braucht, muss
  also erst einen Reload ausloesen.
- **Save-Resume fuehrt NICHT automatisch in die Console.** Nach einem Reload mit
  Run-Save landet man auf dem StartScreen; `start-continue` klicken, um in die
  laufende Console zurueckzukehren. Die Console regulaer erreichen:
  `clickThroughRunStart` (Start-Flow) — `tests/e2e/helpers.ts`.
- **State-Injektion** (z.B. ein Item in einen Run-Slot setzen): Save lesen →
  modifizieren → ueber `context.addInitScript(...)` setzen → `reload`. Ein
  direktes `localStorage.setItem` VOR dem Reload wird vom `pagehide`-Save der
  alten Seite ueberschrieben; `addInitScript` laeuft auf der naechsten
  Navigation NACH dem pagehide-Save und vor den App-Skripten und gewinnt damit.

**Warum.** e2e ist die oberste, teuerste Pyramidenstufe — sie verifiziert „der
echte Browser rendert + reagiert", nicht die Effekt-Mathematik. Die
Save/Resume/Injektions-Mechaniken sind nicht offensichtlich und kosten ohne
Notiz jedes Mal eine Debug-Runde; deshalb fest dokumentiert. Skill:
[docs/skills/50-e2e-tests-erstellen.md](skills/50-e2e-tests-erstellen.md).

#### 4.6a jsdom-Render-Tests: `cleanup()` ist Pflicht + Queries scopen (isolate:false, seit 2026-07-16)

**Regel.** Jede jsdom-Test-Datei (`// @vitest-environment jsdom` + `render(...)`
aus `@testing-library/react`) MUSS `afterEach(() => cleanup())` rufen — es gibt
KEIN Auto-cleanup (die vitest-Config hat kein `globals: true`, also registriert
Testing-Library seinen `afterEach`-Hook nicht selbst). Zusaetzlich: DOM-Queries
im Test bevorzugt auf den eigenen Render-Container scopen
(`within(container).getByText(...)` statt `screen.getByText(...)`); nur
Portal-Inhalt (`createPortal(..., document.body)`, z.B. die `InfoTip`-Blase)
bleibt bei `screen`.

**Warum.** Die Suite laeuft mit `pool: "threads"` + **`isolate: false`**
(vite.config.ts, Boss-Registry-Scan-Kosten) → alle Dateien EINES Workers teilen
sich `document` und den React-Modul-State. Ohne `cleanup()` bleibt der gerenderte
Baum im geteilten `document.body` stehen und **leakt in die naechste Datei
desselben Workers**. Zwei Schadbilder, beide **lokal gruen / in CI rot** (haengt
an Worker-/File-Reihenfolge):
- **`Found multiple elements`** — ein Leaker rendert denselben Text (z.B.
  `taskHud.sectorGoal`), das Opfer findet ihn per `screen.getByText` doppelt.
  Genau so 2026-07-16 (`taskHudNameFallback` → `taskHudTooltips`, commit
  8f6427ab): Fix an zwei Fronten — `cleanup()` im Leaker + `within(container)`
  im Opfer.
- **`NotFoundError: The node to be removed is not a child of this node`** — ein
  spaeterer Portal-Test crasht beim Unmount am haengenden React-Root (commit
  e4833605). `document.body.innerHTML = ""` allein reicht NICHT — erst
  `cleanup()` (React-Unmount), dann optional den Rest-DOM wischen.

Reproduzieren:
`npx vitest run --no-isolate --no-file-parallelism <leaker>.test.tsx <victim>.test.tsx`
erzwingt einen Worker + Reihenfolge; der ehrliche Gegencheck bleibt die volle
`npm test`, weil vitest das `document` je nach File-Scheduling doch mal
zuruecksetzt und der Leak dann nur unter der echten Worker-Packung zuschlaegt.
Siehe `src/__tests__/CLAUDE.md` (Hook-Tests) + Memory
`project_chimera_jsdom_cleanup_isolate_false`.

#### 4.7 Content-/Sim-Invarianten-Tests fuer authored Content (seit 2026-06-13)

**Regel.** Fuer authored Content (Boss-/Run-JSONs) reichen Unit-Tests mit
SYNTHETISCHEN Inputs nicht — ein Boss-JSON kann sauber parsen und sich trotzdem
falsch VERHALTEN. Zwei Suiten-Typen dazu:
- **Sim-Invarianten** (`bossContentSimInvariants.test.ts`): laedt JEDEN echten
  Boss, tickt ihn (z.B. 90 s Kaltstart) und prueft physikalische Invarianten
  (kein Self-Cook, `firesAtPlayer`-Emitter feuert + ueberlebt, Schilde laden,
  Generatoren produzieren).
- **Parse-Vollstaendigkeit** (`bossEventsParse.test.ts`): prueft, dass JEDES Event
  JEDES Bosses parst (kein still-rejected Event). Der Parser verwirft ungueltige
  Payloads nur per `console.warn` + ueberspringt sie — die Sim/Content-Suite merkt
  das sonst nicht (Item/Boss tickt trotzdem).

**Warum.** Beide fingen reale Content-Bugs, die Synthetik-Unit-Tests durchliessen:
fireTimestamps-`gameTimeMs`-Hitze, gestrippte `xp` (Level-0-Schild laedt nicht),
r2-cold `energyEater` `zones`→`zone`. User-Frage msg 12100 („welche Test-Klasse
haben wir vergessen?") → genau diese: echtes authored-Verhalten gegen Invarianten,
nicht nur Funktionen mit konstruierten Inputs. Beim Hinzufuegen neuer Content-Typen
solche Lade-+Tick-Guards mitziehen.

#### 4.8 Erhaltungs-/Physik-Invarianten statt Wert-Asserts (Solver/Sim-Code)

**Regel.** Für Code mit einer **physikalischen/strukturellen Invariante**
(Energie, Hitze, Druck, Flüsse, Aggregation) teste die **Invariante** über
viele/kombinierte Inputs — nicht nur konkrete Output-Werte einzelner Fälle.
`expect(cableFlow).toBe(15)` schreibt bei einem Bug genau den falschen Wert als
„korrekt" fest; `expect(export).toBeLessThanOrEqual(netProduction)` fällt über
JEDE Topologie, die die Erhaltung verletzt.

**Warum.** Wert-Tests entstehen oft charakterisierend (aktuelles Verhalten
beobachten + festschreiben). Leakt die Implementierung, schützt der Test den Bug.
Genau das passierte 2026-06-24: ein `generator(sensor)` exportierte BRUTTO statt
NETTO (lieferte 20 aus 15 Produktion). ~5200 Tests fingen es nicht — sie asserten
Werte, und das Leck sah funktional gesund aus (Schild lud, Sensor lief), selbst
die Boss-Sim-Invarianten blieben grün, WEIL der Bug „funktionierendes" Verhalten
erzeugte. Erst eine Anzeige, die Produktion vs. Verbrauch SUMMIERTE, machte es
sichtbar. Der Regressions-Test ist eine Invariante („Quelle exportiert ≤
effProduction − effDemand", 38 Topologien) — gegengeprüft rot auf Buggy-Code,
grün mit Fix.

**Spiegelfall — dieselbe Invariante, umgekehrt (2026-07-12).** Nicht „zu viel
Fluss", sondern legitimer Fluss STUMM geblockt: ein siphon-getroffener Nicht-Source-
Knoten (Ventilator) wurde Quelle, aber `applyStorageBottlenecks` gab jedem Nicht-
Source-Knoten Export-Kapazität hart `0` → der Überschuss wurde als „aus dem Nichts"
gewertet, der Abfluss genullt; ein gecableter Kondensator lud nie. Wieder fingen es
~6800 Tests nicht: der neue Beitrag war im Reader UND in `buildHydraulicNode`
korrekt, aber die dritte Solver-Stufe kannte ihn nicht — kein Test prüfte end-to-end,
dass ein Produzenten-Überschuss einen gecableten Speicher ERREICHT. Lehre: die
Erhaltung gilt beidseitig (`geliefert ≤ produziert` UND „Überschuss kommt an"), und
ein neuer Beitrag muss durch ALLE Stufen (Reader → Node-Build → Bottleneck → Charge)
gefädelt + per System-Level-Test abgesichert werden.

**How.**
- Erhaltung explizit formulieren: Σ Output ≤ Σ Input (+ Speicher-Entladung);
  beidseitig — verbundener Überschuss muss den Speicher/Verbraucher auch erreichen.
- Ein neuer Beitrag muss durch ALLE Pipeline-Stufen ankommen (Reader → Node-Build →
  Bottleneck → Charge) — System-Level-Test statt nur Per-Funktion.
- Property-/tabellengetrieben über Kombinationen (Bauteile × Verschaltung ×
  Anfangszustände), Invariante in JEDER prüfen — über mehrere Ticks.
- Am richtigen Punkt messen: echte Fluss-/Ladungs-Größe (Kabel-Flow, Ladungs-
  Delta), NICHT eine Display-Aggregat-Kennzahl, die Pool-Bezug + Selbst-
  Entladung mischt (`totalActualConsumption` zählt bei vollem ChargeSink den
  ganzen internalDrain).
- **Wächter verifizieren:** Fix kurz zurücknehmen → Test MUSS rot werden.

**Verwandtes Prinzip (auch außerhalb Tests).** Verifiziere die INVARIANTE, nicht
das Oberflächen-Muster. Content-Migration (46 JSONs): erster Regex-Pass auf
`"zone":"torso"` traf auch Item-`location`s + Events; Catch via Struktur-Check
(nur Objekte mit `maxHp` = echte Zonen-Defs) → revert → Anker auf `maxHp` +
Count-Verifikation. Skill:
[docs/skills/52-erhaltungs-invarianten-testen.md](skills/52-erhaltungs-invarianten-testen.md).

#### 4.9 Diagnose/Anzeige nutzt die echten berechneten Werte (kein Re-Compute)

**Regel.** Eine Anzeige, die eine interne Berechnung „aufschlüsselt" (Energie-
Bilanz, Resolve-Trace), darf die Werte nicht NEU rechnen — sie emittiert die
echten Solver-Zwischenwerte am Berechnungs-Punkt (gegated/optional, damit
Headless/Sim allocation-frei bleibt). Sonst driftet die Anzeige vom tatsächlichen
Verhalten ab und „erklärt" etwas Falsches.

**Warum.** Ein paralleler Re-Compute ist ein zweiter Wahrheits-Strang, der bei
jeder Solver-Änderung still divergiert. Die Energie-Breakdown-Anzeige (msg 12895)
reicht `breakdownOut` IN den Solver und liest dessen echte Werte zurück — dadurch
machte sie den `generator(sensor)`-Leak überhaupt erst sichtbar, statt ihn mit
einer geschönten Parallel-Rechnung zu kaschieren.

#### 4.10 Sim↔Game-Divergenz: das Spiel ist die Referenz

**Regel.** Zeigen Headless-Sim/Bot und das echte Spiel fuer dieselbe Mechanik
unterschiedliches Verhalten, gilt das SPIEL als korrekt (besser getestet) —
die Sim wird angeglichen, nie umgekehrt. Erste Verdachts-Quelle ist Logik,
die nur im React-Hook lebt und nie in die geteilte Engine kam (bewiesene
Faelle: zoneEnergyProduction, Satelliten-Treffer, visibilityTier). Ein
RL-Retrain fixt Platzierungs-Sektoren nicht (`planSector` = Heuristik).
Details in
[docs/skills/63-sim-game-divergenz-spiel-ist-referenz.md](skills/63-sim-game-divergenz-spiel-ist-referenz.md).

#### 4.x Ref-Flags, die setState-Updater steuern, gehören in die Queue

**Regel.** Wenn ein Ref-Flag das Verhalten von setState-Updatern steuert
(z. B. „merge die nächsten Pushes"), müssen die Flag-Flips selbst als
No-op-Updater durch DIESELBE setState-Queue laufen — nie synchron neben ihr.

**Warum.** React batcht: Updater laufen später als der synchrone Code, der
sie enqueued hat. Ein synchrones `end()` im `finally` eines Drop-Handlers
liefe VOR dem noch gequeueten Drop-Push — die Geste zerfiele wieder in
mehrere Undo-Einträge. Beispiel: `useGameHistory.beginHistoryGesture/
endHistoryGesture/resetHistory` (2026-07-06, msg 14271). Zwilling der
bestehenden Regel „keine Ref-Mutationen in Updatern ohne Idempotenz-Guard"
(e2e-Undo-Flake): dort ging es um Mehrfach-Invokes, hier um Reihenfolge.

#### 4.y Effekt-Cleanups fangen ihre Objekte im Closure ein

**Regel.** Ein useEffect-Cleanup meldet sich am SELBEN Objekt ab, an dem es
sich angemeldet hat — Objekt beim Mount in eine Closure-Variable fassen,
nicht im Cleanup erneut durchs globale `window`/Singleton greifen.

**Warum.** Test-Harnesse (und Hot-Reload) können das globale Objekt zwischen
Mount und Unmount austauschen: auf CI lief der RTL-Auto-Unmount NACH
`vi.unstubAllGlobals()` — `window.speechSynthesis` war weg, der Cleanup warf
(lokal grün, CI rot, weil die afterEach-Reihenfolge mit der Datei-Verteilung
auf Worker variiert; 2026-07-06, msg 14250).

#### 4.v Reward-/Zaehler-Emission an denselben Guard koppeln wie die State-Mutation

Ein unbedingtes `reward += X` (oder Stat-Zaehler) direkt NACH einem idempotenten
State-Transformer ist eine Divergenz-Falle: re-entert der Loop den Transformer
(weil ein Sub-State wie `pendingLoot` das Advance blockiert), zaehlt der Reward
mit, der State-Counter nicht (Multi-Pick-Boss-Bug 2026-07-08, engine.ts
if→while). Emission immer an denselben Guard binden wie die Mutation.

#### 4.w Signatur-/Parse-teure Persistenz-Reads NIE ungecacht im Hot-Path

`loadProfiles`/`chimeraSettings`-Reads laufen durch HMAC-SHA256 + Bounded-Parse
ueber den ganzen Blob. Ungecacht im Frame-Pfad (auch versteckt: `getStaticTranslation`
→ Settings → Profile) kostete das 75 % CPU bei 8-12 fps (Bug 2026-06-29, Fix:
In-Memory-Cache in profileStorage). Regel: solche Reads cachen; bei Perf-Jagd
zuerst Chrome-Profiler Bottom-Up (self-time) statt Collection-Groessen raten —
„langsam von Anfang an" + flache Collections = Recompute-Hotspot, kein Leak.

#### 4.z Analyse-Werkzeuge als env-gated Vitest-Dateien

**Regel.** Balance-/Statistik-Werkzeuge, die die echte Engine-Pipeline
brauchen (Loader-Bootstrap!), als `*.analysis.test.ts` mit
`describe.runIf(process.env.X === "1")` anlegen — laufen nie in der
normalen Suite, starten aber mit einem Env-Flag ohne eigene Infrastruktur.

**Warum.** Standalone-Skripte (tsx) scheitern am CJS-`require` des
staticJsonLoader; die Vitest-Umgebung bringt den kompletten Bootstrap
gratis mit. Beispiel: Loot-Odds-Monte-Carlo (`LOOT_ODDS=1`, 2026-07-06) —
Muster analog zur Slow-Suite.

---

### 5. Dokumentation

#### 5.1 Doku im gleichen Commit wie die Code-Aenderung

**Regel.** Wenn ein Feature / ein Refactor / ein Fix die dokumentierte
Realitaet aendert, updated die Doku im selben Commit (oder in einem
direkt folgenden).

**Warum.** Asynchrone Doku veraltert nie von selbst — sie veraltert,
weil niemand sie ihren Weg hinterher pflegt. Im selben Commit zu bleiben
zwingt den Autor, die Doku zu sehen.

#### 5.2 Ein Topic pro Datei, klare Verlinkung

**Regel.** `docs/`-Ordner nach Themen gegliedert (Architektur,
Testing, Deploy, pro grosser Subsystem eine Datei). `CLAUDE.md` oder
`README.md` als Index mit Kurzbeschreibung + Link.

**Warum.** Eine 3000-Zeilen-Monster-Doku liest niemand. Kleine,
topic-fokussierte Dateien werden gelesen und gepflegt.

#### 5.3 Dokumentiere das „Warum" hinter Design-Entscheidungen

**Regel.** Wenn eine Architektur-Entscheidung nicht aus dem Code
ablesbar ist, gehoert sie in die Doku: „Warum pure Reducer statt OOP?
Warum SQLite statt Postgres? Warum diese Zustandsmaschine und nicht
eine andere?".

**Warum.** Sechs Monate spaeter weiss niemand mehr, warum X so ist.
Ohne Doku entsteht die Versuchung, X wegzuoptimieren und dieselben
alten Fallstricke neu zu erleben.

#### 5.4 Keine Duplikate von Code-/Git-Informationen

**Regel.** In Doku niemals Inhalte wiederholen, die sich im Code oder
in `git log` finden lassen. API-Signaturen, Datei-Pfade, Commit-
Historie — das lebt in den originalen Quellen.

**Warum.** Duplizierte Infos driften. Die Kopie veraltet, die Leser
vertrauen der falschen.

#### 5.5 Doku-Konventionen (verbindlich seit 2026-05-05)

Beim naechsten Touch jeder Doku-Datei drive-by anwenden:

**Sprache:** Deutsch. User-Base ist deutschsprachig, neue Docs sind
durchgaengig deutsch. Bestehende mischsprachige Files (heat-system,
energy-system, simulation, bot-training, game-events, features)
werden beim naechsten substantiellen Edit eingedeutscht — kein eigener
Pass noetig.

**H1-Stil:** `# Chimera — <Topic>`. Bei Subsystem-Doku auch `# <Topic>`
(ohne "Chimera —") akzeptiert.

**Test-Coverage-Sektion:** wenn vorhanden, immer `## Test-Coverage`
(nicht "Tests" oder "Test-Abdeckung"). Subsystem-Docs ohne Test-Sektion
bekommen mindestens einen Pointer am Ende ("Tests in
`src/__tests__/<file>.test.ts`").

**Linking:**
- Interne Refs auf Markdown-Files: relative `.md`-Pfade mit Anker:
  `[runs.md → Loot-Pools](runs.md#loot-pools-boss-runjson)`.
- Plan-Refs als Markdown-Link, nicht als Code-Span: `[plan-name.md](plans/...)`
  statt `` `plans/...` ``.
- Aktive Plaene: `docs/plans/...md`. Archivierte: `docs/plans/archive/...md`.
- Cross-Refs immer mit-umbiegen, wenn ein Plan archiviert wird.

**Datei-Granularitaet:** Subsystem-Docs sollten 200-1500 Zeilen halten.
Unter 100 Zeilen: pruefen ob Sektion in einer groesseren Datei besser
aufgehoben waere. Ueber 2000 Zeilen: pruefen ob ein eigenstaendiges
Sub-Topic herausgeloest werden kann (siehe als Beispiel: `tasks.md`
NPC-Subsystem → `npcs.md`).

---

### 6. Commits und Git

#### 6.1 Conventional-Commit-Prefix

**Regel.** Subject-Line beginnt mit `feat|fix|refactor|docs|test|chore|
perf` gefolgt von optionalem Scope und Doppelpunkt. Konsistent im
ganzen Repo.

**Warum.** Macht Changelogs, Filter und bisect trivial. Tools wie
semantic-release hebeln darauf.

**Beispiel.**
```
feat(tasks): Section-SuccessCondition ersetzt per-Task holdMs
fix(runs/pruefstand): kein Start-Clause mehr bei t=0 erfuellt
refactor(hud): Schaltplan-Darstellung fuer Sektor-Condition
docs: dev-analysis.md — Vorgehen fuer Commit-Statistik
```

#### 6.2 Commit-Nachricht erklaert Warum + Was

**Regel.** Subject-Line (kurz, praezise). Leerzeile. Body mit Kontext:
welches Problem, welcher Loesungsansatz, welche Konsequenzen. Body ist
optional bei trivialen Commits, aber empfohlen ab 10+ Zeilen Diff.

**Warum.** In 6 Monaten ist „fix bug" nichts wert. „fix: Sticky-
Fulfilled hat Debounce gecancelt → Sektor endete nie" gibt dir das
gesamte Debugging-Ergebnis zurueck.

#### 6.3 Ein logischer Schritt = ein Commit

**Regel.** Ein Commit sollte genau eine gedankliche Aenderung enthalten.
Zwei unabhaengige Fixes = zwei Commits, auch wenn sie im selben Kontext
aufgefallen sind.

**Warum.** Revert-Granularitaet (siehe 2.2). Einfacher Code-Review.
Klarerer Bisect.

#### 6.3a Pro Schritt SOFORT committen, nicht am Ende sammeln

**Regel.** Bei mehrschrittigen Tasks (Plan mit N Phasen, Bug-Round mit
mehreren Fixes, Refactor in Tranchen) wird **jeder abgearbeitete
Einzelschritt direkt committed** — nicht am Ende eines Tages oder
Mehr-Phasen-Plans gesammelt.

**Warum.** Sammel-Commits verlieren die Phasen-Granularitaet, die 6.3
und 2.2 schuetzen wollen. Wenn drei Phasen am Stueck implementiert und
dann zusammen committed werden, hilft Bisect/Revert nicht mehr — und der
Push-pro-Phase aus 1.5 (Multi-Phasen-Autonomie) wird auch unmoeglich.

**Verhalten.**
1. Schritt N: implementieren → `tsc + tests + build` gruen.
2. **Sofort** `git add <files> && git commit -m "..."`.
3. Push (wenn nicht-trivial; siehe 1.4).
4. Direkt mit Schritt N+1 weiter.

**Anti-Pattern.** Sieben Schritte abarbeiten, am Ende einen riesigen
`feat: damping + count + tests + doku`-Commit. Bisect findet nichts mehr.

#### 6.4 Hooks nicht ueberspringen, commits nicht amenden

**Regel.** Pre-commit-Hooks sind da, um Fehler zu fangen. Wenn einer
schlaegt, den Fehler fixen — nicht `--no-verify` oder `--no-gpg-sign`.
Nach einem Hook-Fail einen NEUEN Commit machen, nicht amenden (der
fehlerhafte Commit existierte nie).

**Warum.** Hooks sind der letzte Schutz vor Kaputtem im Repo. Umgehen
unterwandert das Team-Vertrauen. Amend auf bereits-gepushten Commits
zerstoert Git-Historie anderer Branches.

#### 6.5 Keine `--force`-Pushes auf Shared Branches

**Regel.** `git push --force` auf `main` oder andere Shared Branches
ist verboten — auch wenn der eigene Branch „nur noch schoener" waere.

**Warum.** Zerstoert Arbeit anderer. Einmal gemacht, dauerhaft
schmerzhaft. Fuer eigene Feature-Branches OK; fuer Shared Branches nie.

#### 6.6 Push traegt ALLE unpushed Commits mit

**Regel.** Vor jedem Push `git log @{u}..HEAD --oneline` pruefen: die
Push-Freigabe-Regel gilt fuer den GESAMTEN Commit-Stapel — ein „harmloser"
Doku-Push wuerde sonst ungefragte Commits mitschieben. Details in
[docs/skills/57-push-traegt-alles-mit.md](skills/57-push-traegt-alles-mit.md).

---

#### 6.7 Branch-Workflow: dev / Topic-Branches / main (seit 2026-07-18, Test-Phase)

**Regel** (Owner msg 15495/15497). main deployt automatisch zu ECHTEN
Testern — gearbeitet wird nie mehr direkt auf main:

- **Jeder Plan startet mit einem eigenen Topic-Branch auf dev**
  (`topic/<plan-slug>`); waehrend des Plans wird nur dorthin committet.
  **Topic-Pushes sind frei** (kein CI-/Deploy-Effekt).
- **Abnahme = `git merge --no-ff topic/<plan> ` nach dev** — der Plan
  bleibt als Einheit in der Historie. **dev-Pushes nur nach Freigabe**
  (die fruehere Test/Doku-frei-Ausnahme ist abgeloest); Kleinkram
  ausserhalb von Plaenen wird direkt auf dev committet, Push nach Freigabe.
- **Release = `merge dev → main` ausschliesslich auf explizite
  Owner-Ansage** (+ `release/<n>`-Tag) — der Push deployt sofort. Der
  dev→main-Merge MUSS **`--ff-only`** sein (Owner msg 15904: haelt
  `BUILD_COUNT` auf main identisch zu dev — ein Merge-Commit erzeugte
  ein Off-by-one zu Release-Notes-Namen + Tag; Details Skill 64 Regel 5).
  Nicht verwechseln: topic→dev bleibt `--no-ff`.
- **Lokal laeuft immer dev** (Abnahmen innerhalb eines Plans deployen
  naturgemaess den Topic-Stand, danach wieder dev). CI (ci.yml) laeuft auf
  main- UND dev-Pushes.

Vollstaendig: [docs/skills/64-branch-workflow-dev-topic-main.md](skills/64-branch-workflow-dev-topic-main.md).

---

### 7. Deployment

#### 7.1 Reproduzierbarer Build-/Deploy-Ablauf

**Regel.** Deploy ist eine dokumentierte Kette von Schritten —
idealerweise ein einziges Skript oder eine explizite CLAUDE.md-
Sektion. Keine Geheim-Handgriffe, keine manuelle Reihenfolge, die man
sich merken muss.

**Warum.** Damit ein anderer (Mensch oder Agent) es morgen genauso
machen kann. Jedes „das muss man wissen"-Geheimnis ist ein
Bus-Faktor-1.

**Chimera-Standard:** das 5-Schritt-Commit-Gate aus §4.3, danach Build +
Restart als `&&`-Einheit:
```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e \
  && npm run build && systemctl --user restart chimera
```
Build und Restart gehoeren zusammen (nur build = neuer Code im Filesystem,
alter im Service; nur restart = Service serviert den vorherigen Stand) —
Details in Skill [35-reproducible-deploy.md](skills/35-reproducible-deploy.md)
+ [36-build-vor-restart.md](skills/36-build-vor-restart.md).

#### 7.2 Restart sichtbar verifizieren

**Regel.** Nach einem Restart kurz `systemctl status` oder
`curl localhost:<port>/health` aufrufen, um sicherzugehen, dass der
neue Prozess laeuft. Kein „ich hab restart eingegeben" ohne
Bestaetigung.

**Warum.** Build-Fehler, die Runtime-Fehler wurden, koennen nur so
entdeckt werden bevor der Nutzer sie sieht.

**Bei Remote-/Static-Deploys den INHALT verifizieren, nicht nur den Status
(seit 2026-06-30).** HTTP 200 sagt nur „etwas wird ausgeliefert", nicht „der NEUE
Build". Nach einem GitHub-Pages-Deploy den **tatsächlich ausgelieferten Artefakt-
Inhalt** prüfen: den neuen Build-Hash, oder genau das geänderte Stück im Bundle
(z. B. per `curl` den content-gehashten Chunk holen + auf den erwarteten String
greppen). Beispiele diese Woche: bestätigt, dass die Datenschutz-„15+"-Fassung live
ist, die Klartext-E-Mail NICHT im ausgelieferten Bundle steht und der OFL-Lizenztext
im Chunk liegt. **Lazy-Chunks** sind nicht im `index.html` referenziert → über ihren
content-Hash direkt anfetchen (Vite hasht inhaltsbasiert → lokaler Chunk-Name passt
oft auf den Remote-Chunk). Erst „live + korrekt" melden, wenn das geprüft ist.

#### 7.3 Produktions-Deploy = Release-Merge mit Ansage (revidiert 2026-07-18)

**Regel.** Das oeffentliche Deployment (GitHub Pages, echte Tester) haengt
am main-Branch. Seit dem Branch-Workflow (§6.7) gibt es dorthin nur noch
den **Release-Merge dev→main auf explizite Owner-Ansage**; die fruehere
Regel „Produktiv-Code-Push nach Rueckfrage" ist darin aufgegangen.
Lokale Deploys (build + restart) servieren dev bzw. den Topic-Stand
einer laufenden Abnahme.

#### 7.4 Mehrere Deployment-Targets: lokaler Build ≠ ausgelieferter Build (seit 2026-06-29, 4-Target-Modell 2026-07-01)

**Regel.** Wenn verschiedene Deployments unterschiedlich viel enthalten (gestrippter
Content, ausgeblendete Dev-UI, anderer Base-Pfad), wird das über **ein einziges
Target-Enum** (`VITE_BUILD_TARGET` → `getBuildTarget()`, Ziele `local|test|demo|full`)
plus eine **pure SSOT** (`src/buildTargetConfig.ts`: Base-Pfad + Run-Sets pro Ziel)
gelöst — NICHT über verstreute Ad-hoc-Flags. Zwei Reduktions-Wege: **UI-Prädikate**
(`isPublicBuild()`/`isDemoBuild()`/`showBugReportButton()`) fürs Gating im Bundle und ein
**Content-Strip NACH dem Build** (`scripts/strip-build.ts <target>`, zieht die Run-Liste
aus der SSOT) für Daten/Assets. Der lokale `npm run build` enthält alles (`target=local`
→ `"all"` → kein Strip); der Strip läuft nur im Deploy.

**Warum.** Unveröffentlichte Inhalte dürfen nicht per URL abrufbar sein, aber Dev
braucht den vollen Umfang. Ein Target-Enum statt N Flags, weil sonst jeder neue
Unterschied (Base-Pfad, CTA, Bug-Button) ein weiteres unabhängiges Flag bräuchte, die
sich widersprechen können. **Folge-Fallen:** (a) was der Strip entfernt, darf die App
nicht hart referenzieren → siehe 3.3c (Manifest-getrieben statt fester Liste); (b) der
**Base-Pfad muss target-aware an EINER Stelle** hängen (SSOT `baseForTarget`), sonst
driften `vite base`, `index.html`, Manifest, Service-Worker und Precache auseinander;
(c) ein **Deploy-Gate** bricht ab, wenn Strip-Pflichtinhalte fehlen oder die Härtung
(keine Source-Maps, CSP-`<meta>`) verletzt ist. Referenz: CLAUDE.md → „Build-Varianten",
`docs/plans/2026-07-01-vier-deployment-targets.md`, `docs/security.md`.

#### 7.5 Öffentlicher/kommerzieller Launch: Pflicht-Seiten + Lizenzen (seit 2026-06-30)

**Regel.** Sobald die App öffentlich (und erst recht kommerziell) erreichbar ist,
gehören dazu: **Impressum + Datenschutz** (DE, § 5 DDG / DSGVO), die **vollständigen
Open-Source-Lizenztexte** der ausgelieferten Komponenten (MIT-Permission-Notice je
Paket, voller OFL-Text gebündelter Schriften, Apache-Text) und **altersgerechte/
korrekte Claims** (Datenschutz-Alters­einordnung, Store-Beschreibung).

**Warum.** Rechtliche Pflicht, nicht Kür — und es rippelt: eine Zielgruppen-Änderung
(~12 → 15+) musste durch Datenschutz, README und Docs gezogen werden. Was gebündelt
ausgeliefert wird (Runtime-Libs, Fonts), braucht seinen Lizenztext mit; reine Dev-/
Build-Tools nicht. Kein Rechtsrat — aber die *technische* Pflicht-Anzeige (Seiten +
Lizenztexte) ist umsetzbar und gehört vor den Launch. Referenz: `docs/legal.md`.

#### 7.6 Pilot-Deploy vor Massen-Rollout / bei kritischen + sichtbaren Änderungen (seit 2026-07-01)

**Regel.** Bei **kritischen** oder **sichtbaren UX-/Design-Änderungen** (neues
Interaktionsmuster, Tooltip/Popover-Optik, Layout) — und besonders bei **Massen-
Migrationen**, die dasselbe Muster über viele Stellen ziehen — wird zuerst ein **Pilot**
an EINER Stelle **lokal deployt** (`npm run build && systemctl --user restart chimera`,
HTTP 200 prüfen) und dem User zum Testen gezeigt. Erst nach seiner Freigabe zu Look &
Feel folgt der Rest. **Deploy ≠ Push:** fürs Ansehen reicht der lokale Deploy, ein Push
ist separat (Prod-Code → fragen, 7.3).

**Warum.** Hauptzielgerät ist mobil; wie sich etwas anfühlt (Antipp-Fläche, Blasen-
Position, Unterstreichungs-Stärke) sieht man erst am laufenden Spiel, nicht im Code oder
in Tests. 20 Stellen im falschen Stil zu bauen ist teures Rework — ein Pilot kostet einen
Deploy-Zyklus und verhindert die Sackgasse. Skill
[54](skills/54-kritische-changes-lokal-deployen-testen-lassen.md), Memory
`feedback_local_deploy_test_before_rollout`.

---

#### 7.7 Hintergrund-Prozesse sauber abbrechen: Prozessgruppe + Verifikation (seit 2026-07-17)

**Regel.** Laufende Hintergrund-Jobs (vitest, Sim, Playwright) ueber die
Prozess-GRUPPE abbrechen (`kill -- -<pgid>`), nicht per `pkill` aufs
Kommando-Muster — geforkte Worker matchen das Muster oft nicht, ueberleben
als Waisen und rechnen weiter. Danach IMMER per
`ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head` verifizieren, dass nichts
vom Lauf uebrig ist. Bei `pkill`/`pgrep -f` im Bash-Tool den Bracket-Trick
nutzen (`"[m]uster"`), sonst matcht das Pattern den eigenen Wrapper und
killt den Task selbst (exit 144).

**Warum.** Vorfall 2026-07-17 (msg 15351): ein verwaister vitest-Fork-Worker
brannte 13,5 h einen Kern auf 100 %. Details in
[docs/skills/61-hintergrund-prozesse-sauber-killen.md](skills/61-hintergrund-prozesse-sauber-killen.md).

---

### 8. Speicher / Kontext

#### 8.1 User-Praeferenzen dauerhaft ablegen

**Regel.** Wenn ein User Korrekturen oder Vorlieben aeussert
(„keine Emojis", „Commit-Messages auf Deutsch", „Tests nie stillschweigend
aendern"), persistiere das in einem Memory-System oder einer
Team-Guideline. Nicht jedes Mal neu nachfragen.

**Warum.** Spart Zyklen. Zeigt, dass Feedback ernst genommen wird.

#### 8.2 Lies den Kontext, bevor du fragst

**Regel.** Vor Rueckfragen: CLAUDE.md, dokumentierte Repo-Konventionen,
Memory-Dateien und relevante Dateien durchgehen. Frage nur, wenn die
Antwort nicht abrufbar ist.

**Warum.** Repetitive Klarstellungen sind teuer. Gut vorbereitete
Fragen sind zielgerichtet und respektvoll.

#### 8.3 Verifiziere, bevor du aus Memory handelst

**Regel.** Erinnerungen koennen veralten — bevor auf „X existiert an
Datei Y" gehandelt wird, kurz gegen den aktuellen Stand pruefen (`grep`,
`ls`, Datei-Read). Ein Memory ist ein Snapshot, keine Quelle; „das war mal
so" reicht nicht. Details: Skill
[39-memory-verifizieren.md](skills/39-memory-verifizieren.md).

#### 8.4 Verifiziere Analyse-/Audit-Berichte, bevor du sie umsetzt

**Regel.** Auch Befunde von Sub-Agents / externen Analysen vor der
Umsetzung gegen den aktuellen Code pruefen (gezielter `grep` oder
30-Zeilen-Read auf die genannte Stelle). Stimmt der Befund nicht:
Auftraggeber informieren oder das echte Potenzial neu identifizieren —
nicht blind „nachreichen".

**Warum.** Reports sind hilfreich, aber nicht autoritativ. Beispiel
(2026-04-27): „pressureSolver hat keinen Early-Break" — der Break stand
laengst drin, echter Hebel war die Epsilon-Kalibrierung (1e-6 → 1e-4).
Vorgehens-Details: Skill
[39-memory-verifizieren.md](skills/39-memory-verifizieren.md).

---

### 9. Sicherheit und Geheimnisse

#### 9.1 Keine Secrets ins Repo

**Regel.** `.env`, `credentials.json`, API-Keys gehoeren nicht in
Commits. Pre-commit-Hook oder `.gitignore` enforcen das; im Zweifel
lieber einmal zuviel fragen als zuwenig.

**Warum.** Einmal gepushte Secrets sind kompromittiert, egal wie
schnell man den Commit zurueckzieht. Rollen und Keys muessen dann
rotiert werden.

#### 9.2 Specific-files statt `git add -A`

**Regel.** `git add <explizite-datei>` statt `git add -A` oder
`git add .`. Letzteres nimmt ungewollte temporaere Files, Builds,
Secrets mit.

**Warum.** Kleiner Tippfehler, grosser Schaden. Explizites Staging ist
ein zusaetzlicher Gedanken-Schritt, der Fehler faengt.

---

## Teil B — Workflow (End-to-End)

Der typische Ablauf einer Einzel-Aufgabe, die die Prinzipien aus Teil A
zur Anwendung bringt.

### Schritt 0 — Aufgabe verstehen

Lies die Aufgabe zweimal. Identifiziere:
- Was ist das konkrete Ziel?
- Gibt es unausgesprochene Annahmen?
- Welche Teile des Codes sind betroffen?
- Gibt es Mehrdeutigkeiten (Scope, Optionen)?

Bei Mehrdeutigkeit: Teil A.1.1 greift — frage mit benannten Optionen
nach und **warte** auf die Antwort. Keine Spekulations-Implementierung.

### Schritt 1 — Kontext sammeln

Lies bewusst:
- Die relevanten Quelldateien (nicht raten)
- Die Doku zum Subsystem (falls vorhanden)
- Die letzten Commits in dem Bereich (fuer aktuelle Konventionen)
- Memory / Team-Guidelines fuer User-Praeferenzen

Notiere dir kurz den aktuellen State — mental oder in Tasks.

### Schritt 2 — Plan festhalten

Fuer non-triviale Arbeit: kurzer Plan an den Auftraggeber
kommunizieren (oder in Task-Liste). Hat drei Aufgaben:
- Zeigt, dass du verstanden hast
- Gibt dem Auftraggeber die Chance auf Korrektur bevor Zeit verbrannt
  ist
- Zerschneidet grosse Taetigkeiten in pruefbare Phasen

**Plan-Lifecycle (verbindlich seit 2026-05-05):** Plans, die unter `docs/plans/` als eigenes Markdown-File abgelegt werden, folgen diesem Lifecycle:

0. **Dateiname mit fuehrendem Datum (verbindlich).** `docs/plans/*.md` MUSS mit `YYYY-MM-DD-` beginnen — z.B. `2026-05-24-item-consolidation-audit.md`. Sortiert chronologisch + zeigt Alter auf einen Blick.
1. **Status-Header pflichtig.** Erste oder zweite Zeile nach dem H1: `Status: Entwurf | In Arbeit | Umgesetzt | Deferred | Archiviert`. Bei "Umgesetzt" + "Archiviert" zusaetzlich Implementations-Commit-Hashes ("`commits abc123/def456`") oder ein Verweis "(commits siehe `git log --grep ...`)".
2. **Plan-Ende-Ritual** sobald die Implementierung abgeschlossen ist:
   - **(a) Substanz extrahieren.** Wenn der Plan dauerhaft nuetzliches Konzept-Material enthaelt (Tabellen, Begruendungen, API-Schemata), wandert dieses in den passenden `docs/`-Eintrag (z.B. `architecture.md`, `property-system.md`, `items.md`). Plan-Status auf "Umgesetzt + Substanz extrahiert nach `<file>`".
   - **(b) Archivieren.** Wenn der Plan im Wesentlichen eine Schritt-fuer-Schritt-Anleitung war, deren Spuren im Code stehen, `git mv` nach `docs/plans/archive/` mit aktualisiertem Status-Header. **Nie** archivieren bevor (a) gegengeprueft ist — sonst entsteht eine zwei-Pass-Operation mit git-Luecke.
   - **(c) Loeschen.** Nur wenn der Plan reine Implementierungs-To-Dos ohne dauerhaften Wert enthielt (selten — meistens ist das Plan-Dokument zu schwer fuer "ohne Wert").
3. **Periodisches Audit alle 4 Wochen oder nach grossen Sprints.** Drei Fragen pro Plan: (i) ist der Status-Header noch korrekt? (ii) lebt eine umgesetzte Substanz noch im Plan, die nach `docs/` gehoeren wuerde? (iii) sollte der Plan archiviert werden?

**4. Phase 0 — Coverage-Pre-Check (verbindlich seit 2026-05-29).** Sobald Scope und Phasen-Plan stehen und BEVOR die erste Code-Aenderung der Phase A erfolgt:

- Fuer jede Datei/Funktion, die der Plan modifiziert, die vorhandene Test-Coverage sichten und Luecken nach Teil A.4.5 (`coverage-luecken-triage`, Skill 27) einordnen (erreichbar / defensiv / dead).
- Erreichbare Pfade ohne Tests bekommen **vor** Phase A Charakterisierungs-Tests, die das *aktuelle* Verhalten festschreiben. Stil: Verhaltens-Test, kein Struktur-Test. Diese Tests stuetzen sich auf die heutige Implementierung und werden zur Regressions-Grenze, an der die geplante Aenderung sich auf altes Verhalten messen muss.
- Phase 0 wird als eigene Plan-Phase dokumentiert (Header `Phase 0 — Coverage-Pre-Check`); ihr Commit traegt `test(<area>): pre-impl characterization` und ist sauber vom Refactor-Commit getrennt.
- Wann *nicht* noetig: reine Bug-Fix-Patches (der Regression-Test gemaess A.4 ersetzt den Pre-Check); Funktion ist schon stark abgedeckt (> 80 % Branch + sichtbare Verhaltens-Tests); reine Doku-/Style-Aenderung.

Begruendung: Tests, die NACH einer Aenderung geschrieben werden, sind unbewusst an der neuen Implementierung orientiert — sie pruefen was die Funktion *jetzt* tut, nicht was sie tun *sollte*. Charakterisierungs-Tests vor der Umsetzung verwandeln ein „Refactor mit Vertrauen" in ein „Refactor mit Netz" und decken stille Vertragsabweichungen sofort auf.

Operative Details in `docs/skills/43-coverage-vor-umsetzung.md`.

Konvention fuer Cross-Refs: aktive Plaene als `docs/plans/...md`, archivierte als `docs/plans/archive/...md`. Wenn du einen Plan archivierst, gleichzeitig die Cross-Refs in Code/Doku auf den neuen Pfad umbiegen (sonst entstehen tote Links).

### Schritt 3 — Implementierung

Halte dich an Teil A.2.1 (Scope) und A.3 (Code-Qualitaet).

Waehrend der Arbeit:
- Editiere bestehende Dateien bevorzugt (A.3.1)
- Halte Diff-Groesse ueberschaubar — wenn es waechst, sortiere in
  Phasen (A.2.2)
- Keine Emojis, keine Ueber-Doku-Strings, keine unrequested refactors

### Schritt 4 — Tests ergaenzen

Fuer neue Logik: mindestens ein Unit-Test, der die zentrale Invariante
pruefen wuerde. Fuer Fixes: ein Regression-Test, der ohne den Fix faellt.

Niemals bestehende Tests ohne Grund anpassen (A.4.2).

### Schritt 5 — Lokale Verifikation

Das 5-Schritt-Commit-Gate aus A.4.3:

```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e && npm run build
```

Alle muessen gruen sein. Bei Rot: fixen, nicht committen. Bei reinen
Doku-/Test-Datei-Commits ohne App-Code-Aenderung kann `npm run e2e`
entfallen (A.4.3).

### Schritt 6 — Doku aktualisieren

Wenn die Aenderung die dokumentierte Realitaet aendert (Architektur,
APIs, Konfiguration, Deploy), ziehe die betroffene Doku nach (A.5.1).
Im selben Commit.

### Schritt 7 — Commit

Subject + Body im Conventional-Commit-Format (A.6.1, A.6.2). Explizites
`git add <files>` statt `-A` (A.9.2). Commit-Body erklaert das Warum
— insbesondere bei Bug-Fixes: was war der Fehler, was war die Ursache,
wie wurde er behoben.

Beispiel:

```
fix(successCondition): sticky fulfilled + run-end fuer boss-lose Runs

Zwei Regressionen im Sektor-Abschluss-Pfad:

1) Der Condition-Evaluator war nicht sticky — sobald ein Task waehrend
   eines Ticks currentlyMatched verlor, resettete heldMs auf 0 und der
   Status flippte von "fulfilled" zurueck auf "ongoing". Das konnte
   den 1.5 s-Debounce im Transition-Effekt canceln.
   Fix: Clause bleibt fulfilled nach Erreichen der Haltezeit.

2) Boss-lose Single-Section-Runs beendeten den Run nie.
   Fix: zweiter Pfad fuer "letzte Sektion ohne Boss + fulfilled".

Regressions-Tests in successCondition.test.ts (2 neue Faelle).
```

### Schritt 8 — Deploy (falls relevant)

Bei Live-Services: den dokumentierten Deploy-Ablauf fahren (A.7.1),
danach Status verifizieren (A.7.2).

### Schritt 9 — Push (nach Freigabe)

`git push origin <branch>` — aber **nur** mit expliziter Freigabe des
Auftraggebers, sofern nicht anders vereinbart (A.1.4). Ein „gepusht?"
in 2 Minuten ist besser als ein ungewollter Push.

### Schritt 10 — Rueckmeldung

Kurz + konkret (A.1.2):
- Was wurde gemacht (1 Satz)
- Commit-Hash / PR-Link
- Was noch offen ist (falls etwas ist)
- Falls auf Review / Test gewartet wird: klares „bitte testen"

### Sonder-Workflow: RL-Trainings-Iterationen — Analyse-Sim zuerst

Fuer autonome Trainings-Verbesserungs-Schleifen (Retrains, Fortsetzungs-Zyklen,
Manager-Training) gilt der Owner-Workflow (msg 16204/16209, 2026-07-26; Skill
[66](skills/66-trainings-iteration-analyse-sim-zuerst.md)):

1. **Eval-Messpunkt** am finalen Modell (deterministisch, 12 Seeds) — nie an
   stochastischen Rollout-Logs urteilen.
2. **Diagnose-SIM vor jeder Anpassung:** bei Fehlverhalten einen Simulations-
   Lauf mit Analyse an der Problemstelle fahren (Spy-Bot/Trace/Recording) —
   welche Aktionen waehlt der Bot real, was bietet die Maske, woran scheitert
   der Schritt. Beleg statt Vermutung.
3. **Anpassung auf Analyse-Basis entscheiden** (Reward-Anbindung, Faehigkeit,
   Maske, Content — was die Analyse stuetzt), umsetzen, **committen**.
4. Naechstes Training starten; Rechner darf kontinuierlich voll ausgelastet
   werden. Stopp: Ziel-Gate erreicht (sofort melden) · Plateau (2 Fortsetzungen
   ohne Eval-Fortschritt) · vereinbartes Nacht-Ende (Morgen-Summary).

Begruendung: voreiliges Param-Tuning gegen Trainings-Varianz verbrennt
Iterationen; Diagnose-Laeufe finden Grundlagen-Fehler (Beispiele: Zonen-
Geometrie-Paritaet Bot↔Engine, Puffer-Quellen-Faehigkeit, Waffenketten-
Curriculum — alle aus Analyse-Sims, keiner aus Tuning).

---

## Teil C — Vier-Phasen-Zyklus (praeskriptive Empfehlung)

Teil A und B beschreiben, wie eine einzelne Aufgabe umgesetzt wird.
Dieser Abschnitt schreibt den **Mehr-Tage-Rhythmus** vor: vier
klar abgegrenzte Phasen, die zyklisch durchlaufen werden, mit
Zeitrahmen, die sich an der tatsaechlichen Commit-Historie des
Chimera-Projekts orientieren.

### C.1 Ueberblick

Ein Zyklus ergibt ein schlankes, konsolidiertes Stueck Software:
**Features sind gebaut, Tests abgedeckt, Doku konsistent, Code
architektonisch sauber.** Danach startet der naechste Zyklus mit
Phase 1.

| Phase | Fokus | Empfohlene Dauer |
|---:|---|---|
| 1 | Feature-Addition | **1–3 Tage** |
| 2 | Test-Coverage-Nachzug | **~½ Tag** |
| 3 | Doku-Konsistenz-Pruefung | **~2 Stunden** |
| 4 | Architektur- / Performance- / Redundanz-Analyse + Refactor | **1 Tag** (selten 2) |

**Vollzyklus:** ungefaehr 3–5 Tage Arbeit verteilt auf ~1 Kalenderwoche.
Das liegt im Bereich, in dem in Chimera zwischen zwei Hardening-Tagen
~8–10 Tage lagen (11.04. → 19.04.).

### C.2 Phase 1 — Feature-Addition

**Dauer:** 1–3 zusammenhaengende Arbeitstage.
**Ziel:** thematisch zusammenhaengendes Feature-Paket implementieren.
Tests und Doku fliessen **innerhalb jedes Commits** mit (Baseline aus
Teil A.4.1 und A.5.1). Die weiteren Phasen sind Nachzug-Phasen, nicht
Ersatz fuer die Per-Commit-Pflicht.

**Typische Aktivitaeten:**
- Neue Features, neue Modi, neue UI-Komponenten, neue Subsysteme
- Bugfixes, die auf dem Weg entdeckt werden
- Per-Commit-Tests fuer neue Logik (Baseline)
- Per-Commit-Doku-Update wenn Architektur/APIs beruehrt werden

**Grenzsignal:** nach 3 Tagen in Folge mit feature-dominantem
Profil (>40% der Commits sind `feat:`) wird zwingend zu Phase 2
gewechselt. Chimera hat nie laenger als 3 Tage am Stueck rein auf
Features gefahren — die Signale dafuer sind solide.

**Chimera-Beleg:** Feature-Burst 14.–18.04. (3 intensive Tage
mit 73 + 9 + 55 = ~137 Feature-Commits), ebenso 25.–26.03., 29.–30.03.,
06.–07.04., 18.04. Alle innerhalb des 1–3-Tage-Fensters.

**Exit-Kriterium:** Phase 1 endet, wenn
(a) das geplante Feature-Paket fachlich fertig ist, ODER
(b) das 3-Tage-Limit erreicht ist, auch wenn noch mehr geplant war.

### C.3 Phase 2 — Test-Coverage-Nachzug

**Dauer:** ~½ Arbeitstag (2–4 Stunden).
**Ziel:** Luecken schliessen, die trotz Per-Commit-Tests in Phase 1
aufgelaufen sind. Kein neuer Feature-Code.

**Typische Aktivitaeten:**
- Coverage-Report oder `npm test -- --coverage` pruefen
- Edge-Cases fuer neue Modi identifizieren, die nicht im
  Per-Commit-Test waren
- Regressions-Tests fuer in Phase 1 entdeckte, aber nur flach
  reparierte Bugs
- Integration-Tests ueber mehrere neue Subsysteme hinweg
- Test-Utilities bereinigen, redundante Test-Setups konsolidieren

**Exit-Kriterium:** Coverage fuer die in Phase 1 eingefuehrten
Pfade ist lueckenschluss-nah. Kein absoluter %-Wert — es zaehlt,
dass die naechste Phase auf einer gruenen Suite aufsetzen kann.

**Chimera-Beleg:** In der beobachteten Historie fanden Test-
Nachzuege verzahnt mit Phase 4 statt (z.B. 5 Test-Commits am
14.04., 6 am 16.04., 4 am 19.04.). Die hier empfohlene Trennung
macht die Test-Arbeit sichtbarer und lueckenfrei.

### C.4 Phase 3 — Doku-Konsistenz-Pruefung

**Dauer:** ~2 Stunden.
**Ziel:** sicherstellen, dass docs/ die neue Realitaet wiedergibt.
Kein Code — nur Doku.

**Typische Aktivitaeten:**
- Alle in Phase 1 veraenderten Subsysteme gegen ihre docs/*.md-Datei
  pruefen
- Querverweise hinzufuegen, falls neue Konzepte andere Dokumente
  beruehren
- Veraltete Passagen loeschen oder markieren
- CLAUDE.md / README / Architektur-Doku auf Aktualitaet pruefen
- Falls die Phase lueckenhafte Doku aufdeckt, Mini-Fixes direkt dort

**Exit-Kriterium:** Keine Diskrepanzen zwischen dokumentiertem und
tatsaechlichem Verhalten. Doku-Beispiele stimmen mit aktuellem
Code-Verhalten ueberein.

**Chimera-Beleg:** 40 `docs:`-Commits ueber den Gesamtzeitraum,
oft in den Hardening-Tagen konzentriert (7 am 18.04., 5 am 19.04.).
Die hier empfohlene Trennung macht aus zerstreutem Doku-Nachzug
eine bewusste Konsistenz-Session.

### C.5 Phase 4 — Architektur / Performance / Redundanz + Refactor

**Dauer:** 1 Arbeitstag (selten 2).
**Ziel:** technische Schuld abbauen, die in Phase 1 aufgelaufen ist.
Keine Funktionsaenderungen — nur Struktur/Perf.

**Ablauf:**
1. **Analyse-Schritt (vormittags):**
   - Hotspot-Bericht einholen: `chimera-complexity-analysis.md`
     oder aequivalent — Dateien > 500 LOC, ungesunde
     Abhaengigkeits-Richtungen, doppelte Logik.
   - Performance-Messungen an Hot-Paths (Browser-Profiler,
     `console.time`, Render-Counts).
   - Mentaler Architektur-Scan: Schicht-Sauberkeit (Reducer pure,
     Hooks duenn), Dependency-Richtung, Zustandsfluss-Klarheit.
   - **Ab ~10 k LOC oder beim ersten "richtigen" Architektur-Audit**:
     statt mentalem Scan parallele Subagents — 4-6 Audit-Achsen,
     jeder Agent 600-800 Worte zurueck, Synthese im Plan-Doc. Siehe
     [Skill 47](skills/47-architektur-audit-mit-subagents.md). Hebt
     ohne Token-Schmerz in Senior-Audit-Niveau (Madge-Cycles,
     SOLID/DRY/Hexagonal-Checks, Quick-Win/Mid/Big-Roadmap).
   - Findings auflisten (kurze Notiz pro Befund).

2. **Umsetzungs-Schritt (nachmittags):**
   - Refactor-Commits im Conventional-Prefix `refactor:`
   - Ein Befund pro Commit (Teil A.6.3)
   - Nach jedem Commit `tsc + tests + build` gruen (A.4.3)
   - Bei groesseren Umbauten in Phasen aufteilen (A.2.2)

**Exit-Kriterium:** Alle in der Analyse notierten Befunde sind
entweder umgesetzt oder explizit auf „spaeter" zurueckgestellt
(dann in `docs/future-improvements.md`). Tests gruen.

**Chimera-Beleg:**
- 11.04.: 33 Commits, davon 23 `refactor:` — nach einer
  10-tages-Mixed-Phase.
- 19.04.: 67 Commits, davon 27 `refactor:` — direkt nach dem
  Feature-Burst 14.–18.04.
Beide zeigen, dass sich das Refactor-Volumen eines Hardening-Tages
im Bereich 20–30 Commits bewegt und an einem Arbeitstag machbar ist.

### C.6 Phasenuebergaenge (Gating)

Zwischen Phasen stehen klare Gates — nicht einfach „weitermachen":

| Uebergang | Gate |
|---|---|
| Phase 1 → 2 | Per-Commit-Suite gruen. Feature-Paket funktional fertig oder 3-Tage-Limit erreicht. |
| Phase 2 → 3 | `npm test` zeigt keine in Phase 1 eingefuehrten Coverage-Luecken mehr. |
| Phase 3 → 4 | Ein schneller Doku-Spot-Check gegen ein zufaellig gewaehltes Subsystem ergibt keine Diskrepanz. |
| Phase 4 → 1 | Alle Analyse-Findings umgesetzt oder in `future-improvements.md` verbucht. Build + Tests gruen. |

Wenn ein Gate nicht haltbar ist (Aenderungen zu gross, unklar, usw.),
gilt die Regel aus Teil A.2.1: Scope zurueckziehen, nicht ueberdehnen.

### C.7 Zyklus-Frequenz

Ein Vollzyklus dauert ungefaehr 3–5 Arbeitstage (~1 Kalenderwoche bei
Teilzeit-Engagement). Damit ergibt sich eine **Hardening-Kadenz von
~7–10 Tagen** — exakt das, was die Chimera-Historie gezeigt hat
(11.04. → 19.04.: 8 Tage Abstand).

Laengere Zyklen (>2 Wochen ohne Phase 4) erzeugen technische Schuld,
die sich schwerer abbaut. Kuerzere Zyklen (<4 Tage) werden vom
Overhead der Phasen 2–4 unprofitabel.

### C.8 Was NICHT Teil des Zyklus ist

- **Keine Bugfixes als Phase.** Bugs werden in der Phase behoben, in
  der sie auftauchen — Phase 1 (Feature) oder Phase 4 (Refactor). Ein
  isolierter Bugfix-Sprint ist kein Bestandteil des Zyklus.
- **Keine Release-Phase.** Deploy laeuft durchgaengig (Teil A.7); es
  gibt keinen Code-Freeze am Ende eines Zyklus.
- **Keine Kickoff-/Retro-Meetings.** Der Zyklus ist Arbeitsrhythmus,
  nicht Prozess-Zeremonie.

### C.9 Naechtliches Housekeeping (Daily) — Coverage + Mutations-Probe + Doku- + Skills-Drift

Der Vier-Phasen-Zyklus (C.1–C.7) ist die **Wochen-Kadenz** fuer das
substantielle Hardening. Komplementaer dazu laeuft **taeglich, immer
wenn die Task-Queue leer ist** (typischerweise nachts oder in
Idle-Phasen) ein kleiner Four-Pass — autonom, ohne User-Entscheidung,
weil keine Konflikt-Risiken bestehen und der Aufwand gering ist:

1. **Test-Coverage-Pass.** Coverage-Report fahren, Luecken nach Skill 27
   triagieren (erreichbar → Verhaltens-Test / defensiv → Marker /
   dead → loeschen). **Zuerst pruefen, ob ueberhaupt ein Report entstanden
   ist** (`coverage/coverage-summary.json`) — vitest schreibt ihn nur bei
   gruenem Lauf, ein einziger Timeout laesst ihn ausfallen und der Pass sieht
   dann wie „nichts zu tun" aus. Genau so lief er 2026-08 zwei Wochen ins
   Leere (Housekeeping-Fund 2026-08-21).
2. **Mutations-Pass** (seit 2026-07-14, User-Auftrag msg 15097; §4.5b
   + Skill 59). Zentrale Zustands-Schreibungen je einmal abschalten,
   volle Suite — bleibt sie gruen, fehlt ein Waechter → Test nachziehen.
   Mutationen immer zurueckrollen (`git diff` = leer).
3. **Doku-Drift-Pass.** Alle Dateien unter `docs/` (rekursiv) und alle
   `CLAUDE.md`-Dateien gegen den aktuellen Code-Stand pruefen; dazu die
   **generierten Doku-Bloecke regenerieren** (`npm run docs:items` +
   `npm run docs:deployment`, Owner msg 15459 — veraenderte Ausgabe =
   Drift, mitcommitten). Den Diff dabei LESEN: ein Generator, der ein
   Verzeichnis ausliest, kann maschinen-lokale Artefakte aufsaugen und sie in
   eine committete Doku schreiben (Skill 53, Folge-Falle 3). Ein Link-Walk
   ueber alle `*.md` gehoert in denselben Pass — Doku-Umzuege brechen die
   relativen Links IN der verschobenen Datei, nicht nur die auf sie. Fixes sofort committen (ein Commit pro Datei,
   Skill 32).
4. **Skills-/Practices-Drift-Pass** (seit 2026-07-06, User-Spec
   msg 14133). `docs/skills/` + dieses Dachdokument gegen die seit dem
   letzten Pass angefallenen Learnings pruefen, inkl.
   **Memory→Skills/GDP-Vollabgleich** (User-Auftrag msg 15430,
   2026-07-17). Jede Aenderung dreifach synchron: Skill-File +
   Skills-Index + Dachdokument.

Operative Volldetails aller vier Paesse (Trigger, Schritt-Listen,
Anti-Patterns): Skill
[42-housekeeping-coverage-doku-drift.md](skills/42-housekeeping-coverage-doku-drift.md).

**Warum daily statt nur wochentlich.** Drift ist im Gegensatz zu
Architektur-Schuld linear in der Zeit — Code aendert sich jeden Tag,
Doku altert jeden Tag. Wenn die Korrektur am gleichen Tag passiert wie
die Code-Aenderung, ist sie trivial; sammelt sie sich ueber 1–2
Wochen, ist sie ein eigenes Hardening-Subprojekt. Daily-Housekeeping
schiebt die Drift-Halbwertszeit von Wochen auf einen Tag und
entlastet damit Phase 2 und Phase 3 des Vier-Phasen-Zyklus.

**Branch-Kontext (seit 2026-07-18):** Housekeeping-Commits sind Kleinkram
→ direkt auf dev committen; der Push nach origin/dev braucht eine
Freigabe (§6.7) — nachts lokal committen, Push-Buendel am Morgen melden.

**Autonomie.** Vollautonom triggerbar, ohne User-Entscheidung. Sobald
der User wieder Tasks anstoesst, wird der Pass an der naechsten
Commit-Grenze sauber unterbrochen — daher ist die Pro-Schritt-Commit-
Regel (`32-commit-pro-schritt.md`) hier doppelt wichtig.

---

## Teil D — Beobachteter Arbeitsrhythmus aus der Commit-Historie

Teil A und B beschreiben, wie eine einzelne Aufgabe umgesetzt wird;
Teil C den praeskriptiven Vier-Phasen-Zyklus. Dieser Abschnitt
beschreibt, wie sich Aufgaben ueber laengere Zeit **tatsaechlich
aneinandergereiht** haben — gegruendet auf die echte Commit-Historie
des Chimera-Projekts (18.03.–27.04., 1055 Commits ohne Merges). Kein
Soll-Zustand, sondern empirische Baseline, an der sich der Vier-
Phasen-Zyklus aus Teil C orientiert.

### D.1 Mini-Zyklus — pro Aenderung (Stunden)

Die kleinste Einheit. Pro Aenderung ein Commit, der bereits vollstaendig
ist: Feature-Code + zugehoerige Tests + ggf. Doku-Nachzug. Das heisst:
**Tests und Doku sind keine Phasen, sondern Bestandteil jedes einzelnen
Commits**. Im Projekt sind pro Tag 5–30 solcher Mini-Zyklen ueblich,
mit Ausreissern bis 70+ an intensiven Feature-Tagen (siehe D.3).

Siehe auch Teil A.5.1 (Doku im selben Commit) und A.4.1 (Tests
begleiten Features/Fixes).

### D.2 Mikro-Rhythmus — Feature-Bursts (1–4 Tage)

Die zweitkleinste Einheit. Ein thematisch zusammenhaengendes
Feature-Paket — typischerweise ein neues Subsystem, eine neue
Condition-Grammar, eine neue UI-Komponente — zieht sich ueber
1–4 zusammenhaengende Tage mit feature-dominantem Commit-Profil
(mind. 35–40% der Commits dieses Tages sind `feat:`).

**Beobachtete Feature-Bursts in Chimera:**

| Zeitraum | Dauer | Commits/Tag | Inhalt (Kurzfassung) |
|---|---:|---:|---|
| 25.–26.03. | 2d | 10, 10 | Erste Feature-Welle nach Bootstrap |
| 29.–30.03. | 2d | 7, 15 | — |
| 06.–07.04. | 2d | 11, 8 | — |
| 14.–16.04. | 3d | 73, 9, 55 | Grosses Feature-Paket (~137 Commits) |
| 18.04. | 1d | 48 | Feature-Nachlegen |
| 22.–25.04. | 4d | 37, 53, 34, 61 | Zone-HP-System + RL-Bot-Gap-Closure + heatExtractor (~185 Commits) |

**Durchschnitt:** ~2 Tage pro Feature-Burst, max. 4 Tage zusammenhaengend
(22.–25.04., der laengste bisher beobachtete Burst). Lange reine
Feature-Sequenzen jenseits davon gibt es **nicht** — dazwischen mischen
sich Fixes und Refactors.

### D.3 Makro-Rhythmus — Hardening-Tage (1–2 Tage)

Nach einem groesseren Feature-Paket folgt typisch ein **Hardening-Tag**:
ein konzentrierter Tag mit refactor-/perf-/test-dominantem Profil, der
die in der Feature-Phase aufgelaufene technische Schuld abbaut.
Typischerweise 15–30 refactor-Commits an einem einzigen Tag, oft
ergaenzt durch einen Perf-Pass und einen Test-Coverage-Nachzug.

**Beobachtete Hardening-Tage in Chimera:**

| Datum | Refactor-Commits | Gesamt-Commits | Trigger |
|---|---:|---:|---|
| 11.04. | 23 | 33 | Nach 10-Tages Feature-/Mixed-Phase |
| 19.04. | 27 | 67 | Direkt nach 14.–18.04. Feature-Burst |
| 26.04. | 16 | 50 | Architektur-Klaerung (types-Split, Handler-Maps, RNG-Injection) |
| 27.04. | 8 | 46 | Perf-/Test-/Doku-Nachzug (Heat-Konstanten, ResolveCache, Coverage) |

Typische Aktivitaeten an einem Hardening-Tag:
- **Architektur-Review:** pruefen, ob neue Features ordentlich in
  bestehende Subsysteme eingebettet sind (Schicht-Sauberkeit,
  Abhaengigkeits-Richtung, Zustandsfluss). Beispiele aus 26.04.:
  Handler-Map-Familien (NPC, Tasks, Effects), types-Monolith-Split,
  RNG-Injection durch Boss-Event-Aktivierung.
- **Performance-Pass:** Hot-Paths messen, unnoetige Re-Renders / Hooks
  eliminieren, teure Operationen memoisieren. Beispiele aus 27.04.:
  Per-Tick `ResolveCache` (WeakMap) fuer `resolveTree`, pressureSolver
  Iterations- und Epsilon-Kalibrierung.
- **Redundanz-Elimination:** doppelte Logik zusammenfuehren, Hilfsfunktionen
  extrahieren, parallele Datenstrukturen verschmelzen. Beispiele aus
  27.04.: Heat-Sim-Konstanten in `heatConstants.ts` zentralisieren,
  7 inline-`apply*`-Funktionen aus `useHeatSimulation` in eigene
  pure Module unter `src/run/` ausgelagert (982 → 637 LOC).
- **Test-Coverage-Nachzug:** Luecken schliessen, die waehrend der
  Feature-Phase nicht abgedeckt waren. Beispiele aus 27.04.:
  zoneHealAll Defensive-Guards, propertyBag zeroDemand-Pfade per
  ItemDef-Mock, heatPhysics Branch 91.78% → 97.26%.
- **Doku-Konsistenz-Check:** docs/ durchgehen, ob neue Konzepte in den
  relevanten Subsystem-Dateien beschrieben sind. In Chimera liegen
  docs/ so, dass jedes Subsystem sein eigenes Markdown-File hat
  (`tasks.md`, `game-flow.md`, `runs.md`, etc.). Zusaetzlich fliessen
  neue Erkenntnisse in `good-development-practices.md` ein
  (siehe Commit `0f3bc91` vom 27.04.).

**Faustregel:** ein Hardening-Tag pro 5–10 Feature-Tagen. Beobachtete
Abstaende: 11.04. → 19.04. (8 Tage), 19.04. → 26.04. (7 Tage),
26.04. → 27.04. (1 Tag — 27.04. ist tatsaechlich die zweite Halbzeit
einer zweitaegigen Hardening-Sequenz). Das passt zum lockeren
Wochen-Rhythmus, ist aber nicht erzwungen.

### D.4 Was NICHT gemacht wird (bewusst)

Der Chimera-Rhythmus verzichtet auf einige gaengige Strukturen:

- **Keine dedizierten Test-Tage.** Tests entstehen im selben Commit
  wie das zu pruefende Verhalten (Teil A.4.1). Der Test-Coverage-
  Nachzug findet im Rahmen der Hardening-Tage statt, nicht als
  separate Aktivitaet.
- **Keine dedizierten Doku-Tage.** Doku ist Teil jedes Commits, der
  sie beruehrt (A.5.1). Konsistenz-Reviews finden am Hardening-Tag
  mit statt.
- **Keine Sprint-Zyklen.** Es gibt keine 1- oder 2-Wochen-Sprints mit
  Kickoff/Retro. Die Arbeit fliesst; der Rhythmus emergiert aus dem
  Abwechseln zwischen Feature-Drang und dem Druck aufgelaufener
  Schuld.
- **Keine Pre-Release-Lockdown-Phasen.** Es gibt keinen dedizierten
  „Code Freeze" vor einem Deploy — der Service wird jeden Tag
  mehrfach neu gestartet (siehe A.7).

### D.5 Trigger fuer einen Hardening-Tag

Woran erkennt man, dass der naechste Tag ein Hardening-Tag sein
sollte? Beobachtete Signale aus Chimera:

1. **Mehrere Feature-Bursts hintereinander** ohne Refactor-Tag
   dazwischen (>5 feature-dominante Tage in Folge). Konkretes Beispiel:
   22.–25.04. — vier feature-dominante Tage am Stueck → 26.04.
   Hardening-Tag, der die aufgelaufene Schuld systematisch abbaut.
2. **Groessen-Signal:** eine Datei ist ueber ~500 LOC gewachsen oder
   eine andere ist unklar geworden (die Hotspots aus
   `docs/analysis/complexity-analysis.md` machen das sichtbar). Konkretes Beispiel:
   `useHeatSimulation.ts` war auf 982 LOC angewachsen → am 27.04. in
   7 pure Module zerlegt (637 LOC verbleibend).
3. **Neu-Feature auf wackliger Basis:** wenn die naechste geplante
   Erweiterung sich auf ein schwammiges Subsystem stuetzt, lohnt sich
   der Refactor zuerst. Beispiel: types-Monolith mit 441 LOC wurde am
   26.04. in 5 thematische Files gesplittet, bevor weitere Bot-Features
   draufgesetzt wurden.
4. **Doku driftet sichtbar:** wenn man beim Einarbeiten merkt, dass
   die Doku mehrere Schritte zurueck liegt, oder dass neue
   Praktiken/Erkenntnisse aus den letzten Tagen nicht in
   `good-development-practices.md` reflektiert sind.

### D.6 Zusammenfassung der Kadenzen

| Kadenz | Dauer | Inhalt | Frequenz |
|---|---|---|---|
| **Mini** (pro Aenderung) | Minuten–Stunden | 1 Commit mit Code + Tests + Doku | 5–70/Tag |
| **Mikro** (Feature-Burst) | 1–4 Tage | Thematisch zusammenhaengendes Feature-Paket | ~alle 3–7 Tage |
| **Makro** (Hardening) | 1 Tag (gelegentlich 2) | Refactor + Perf + Redundanz + Test-Backfill + Doku-Pruefung | ~alle 7–10 Tage |

Keine Phase ist scharf von der naechsten abgegrenzt — Commits einer
Kadenz koennen in eine andere hineinreichen (z.B. ein Refactor-Commit
mitten in einem Feature-Burst, wenn es den Fortschritt blockiert, oder
ein Feature-Commit am Hardening-Tag, wenn das Refactor eine kleine
Funktionserweiterung mitnimmt). Der Rhythmus ist das Muster ueber viele
Commits hinweg, nicht eine strikte Rotation.

### D.7 Beobachtungen aus 41 Tagen (18.03.–27.04.)

**Verteilung der Commit-Kategorien (1055 Commits ohne Merges):**

| Kategorie | Commits | Anteil |
|---|---:|---:|
| feature | 373 | 35.4% |
| misc | 216 | 20.5% |
| fix | 178 | 16.9% |
| refactor | 153 | 14.5% |
| doc | 88 | 8.3% |
| test | 47 | 4.5% |

**Was das ueber den Rhythmus sagt:**
- Feature dominiert (35%), aber kein einzelner Tag hat reines
  Feature-Profil — selbst der staerkste Feature-Tag (14.04. mit
  73 Commits) hat 14 Fix- und 14 Refactor-Commits eingestreut.
- Refactor (14.5%) ist hoch genug, um die kontinuierliche
  Schuld-Reduktion sichtbar zu machen — aber konzentriert sich
  in den Hardening-Tagen (allein 11.04., 19.04., 26.04. tragen
  ~66 der 153 Refactor-Commits, also ~43%).
- Test (4.5%) wirkt niedrig — ist es aber nicht: die meisten Tests
  reisen Huckepack mit `feat:`/`fix:`-Commits mit (siehe Teil A.4.1).
  Reine `test:`-Commits sind Coverage-Nachzuege, also vorrangig
  Hardening-Aktivitaet.
- Doc (8.3%) liegt im erwarteten Rahmen, da pro Feature i.d.R. ein
  oder zwei reine Doku-Commits anfallen (zusaetzlich zu der Doku, die
  in Feature-Commits eingebettet ist).

---

## Anti-Pattern-Sammlung (Kurzliste)

Dinge, die **nie** passieren sollten:

- Commit mit fehlschlagenden Tests, „fix ich gleich nach"
- `git push --force origin main`
- Silent-Edit an bestehenden Tests, damit der neue Code gruen wird
- Doku-Update „spaeter mal" (→ nie)
- „Wenn ich schon dabei bin"-Refactor in einem Bug-Fix-Commit
- Major-Version-Bump ohne expliziten Task dafuer
- Secrets mit `git add -A`
- **Ein Werkzeug im Arbeitsbaum laufen lassen, das Dateien anfasst, waehrend man
  nebenher weiterarbeitet** (Mutations-Sweep, Codemod, Auto-Formatter im
  Watch-Modus). Der Baum ist dann zeitweise absichtlich kaputt: `git status` zeigt
  fremde Aenderungen, `git add -A` committet sie, und ein pauschaler
  Aufraeum-Schritt (`git checkout -- src/`) loescht die eigene, nicht-committete
  Arbeit gleich mit. 2026-07-15 genau so passiert (ein halber Test-Refactor war
  weg). Regel: solche Werkzeuge nur auf sauberem Baum starten, sie muessen exakt
  das zurueckrollen was sie selbst angerichtet haben — und nichts sonst.
- Katalog-Klassifikationen aus Einzeilen-Greps an den User schicken (mehrzeilige Entries = stille False-Negatives)
- Emojis in Code oder Commits (ausser explizit gewuenscht)
- Beruecksichtigen von Reminder-Meldungen / System-Messages in der
  Antwort an den Nutzer
- Aus Memory handeln ohne Verifikation, dass der Inhalt noch stimmt
- Analyse-/Audit-Berichte 1:1 umsetzen, ohne den genannten Befund kurz
  am echten Code zu verifizieren
- Coverage-Luecke ueber synthetische Mocks „schliessen", obwohl der
  betroffene Pfad strukturell unerreichbar ist (echte Wahl: loeschen
  oder im Commit-Body als defensiver Guard markieren)
- Grosse inline-Sub-Funktionen in Hooks belassen, wenn sie alle ihre
  Deps ueber Closure-Capture beziehen — Extraktion zu pure Module mit
  explizitem Input-Interface zwingt sich erst beim naechsten Bug-Fix
  in dem Gewuehl auf
- Konstanten zu einem Subsystem ueber 3+ Dateien verteilen, statt eine
  `<bereich>Constants.ts` zu eroeffnen
- Auf Telegram-Eingang per Terminal-Output (oder AskUserQuestion-Dialog)
  antworten — der Sender sieht es nicht
- Mehrschrittige Aufgabe komplett abarbeiten und am Ende einen Sammel-
  Commit machen, statt pro Schritt sofort zu committen
- Plan-Datei in `docs/plans/` ohne fuehrendes `YYYY-MM-DD-` im Dateinamen
- `npm run build` und `systemctl restart` getrennt fahren statt mit `&&`
  als Einheit (entweder serviert alter Stand, oder Service startet ohne
  neue Files)
- Bei Refactor-Scope-Entscheidung das Scaffolding-Minimum waehlen
  („quick-and-dirty, sauber spaeter") statt der substantiell richtigen
  Variante
- Eine Kern-Engine-Semantik (Aggregations-Reihenfolge, Owner-Scope,
  Gate-Propagation, Event-Lifetime) auf einen einzelnen Bug-Report/Screenshot
  hin „geradebiegen", ohne zu pruefen ob das Verhalten eine bewusste
  Design-Invariante ist (z.B. Modifier Child→Parent nie Sibling; Boss-Event
  ohne Dauer = permanent → akkumulierendes Hazard ist eine vergessene
  `durationMs` im JSON, kein Engine-Bug — 2026-06-13 revertiert)
- User-Daten (Save/Export) beim Übernehmen umlayouten/clampen statt 1:1 zu
  übernehmen; oder Werte aus einem Screenshot zurückrechnen, obwohl die
  Quell-Datei vorliegt
- Einen Plan auf „Umgesetzt" setzen, sobald das letzte Feature laeuft —
  ohne die abschliessende Refaktorierungs-Audit-Phase (Architektur/Redundanz
  ueber die Gesamt-Implementierung) tatsaechlich durchzufuehren

---

## Revision

Wenn eine Regel in der Praxis Probleme macht, stelle sie in Frage
statt sie stumm zu umgehen. Praktiken sollten dem Ziel dienen —
funktionierende, wartbare Software schnell und mit niedriger
Fehlerrate auszuliefern — nicht umgekehrt.

Aenderungen an diesem Dokument: per PR, kurze Begruendung, Zustimmung
mindestens eines weiteren Teammitglieds.
