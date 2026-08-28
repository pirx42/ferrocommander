# CLAUDE.md — TC Clone Linux

Ein tastatur-zentrierter Dual-Pane-Dateimanager fuer Linux im Stil von
Total Commander (ghisler.com), von Grund auf neu in Rust + GTK4.
Design und v1-Scope: [docs/2026-08-28-tc-clone-design.md](docs/2026-08-28-tc-clone-design.md).

**Stack:** Rust + GTK4 (gtk4-rs) · Cargo-Workspace: `tc-core` (UI-freie
Engine: VFS, File-Ops, Suche, Multi-Rename, Listing) + `tc-app` (GTK-Shell)
**Zielplattform:** Linux (X11/Wayland); Entwicklung derzeit unter Windows

## Build & Run

```bash
cargo build                # Debug-Build (Workspace)
cargo run -p tc-app        # App starten
cargo test --workspace     # Unit-/Integrations-Tests (tc-core headless)
cargo fmt --all -- --check # Format-Gate
cargo clippy --workspace --all-targets -- -D warnings  # Lint-Gate (CI)
cargo build --release      # Release-Build
```

Vor jedem Commit die volle Sequenz gruen — siehe
[docs/skills/25-gruene-suite-vor-commit.md](docs/skills/25-gruene-suite-vor-commit.md).

## Search Defaults (Grep/Glob/Find)

Suchen ueber den ganzen Tree sollen standardmaessig ausschliessen:

- `target/` — Cargo-Build-Output.
- `docs/plans/archive/` — historische Plan-Dokumente.

Praktisch fuer Bash-Grep: `grep -r --exclude-dir={target,archive}`.

## "Wo finde ich was?" — Aufgaben-Cookbook

Das Projekt ist im Aufbau; die Tabelle waechst mit dem Code.

| Aufgabe | Startpunkt |
|---|---|
| v1-Scope / Architektur nachschlagen | [docs/2026-08-28-tc-clone-design.md](docs/2026-08-28-tc-clone-design.md) |
| Arbeitsregeln / Workflow | [docs/good-development-practices.md](docs/good-development-practices.md) + Skill-Trigger unten |
| Neues Plan-Dokument | `docs/plans/YYYY-MM-DD-<topic>.md` (Skill [10](docs/skills/10-plan-lifecycle.md)) |

## Documentation

Einstieg: [docs/CLAUDE.md](docs/CLAUDE.md) — Index aller Doku-Dateien.
Jedes Verzeichnis mit eigener Semantik bekommt ein eigenes `CLAUDE.md`
mit `← Parent`-Link (Muster aus dem Chimera-Projekt).

## Coding Standard

- `rustfmt`-Defaults (kein eigener Style), `clippy -D warnings` als Gate.
- Keine Magic Values — benannte Konstanten, pro Subsystem zentralisiert
  (Skills [16](docs/skills/16-keine-magic-values.md) /
  [17](docs/skills/17-konstanten-zentralisieren.md)).
- Kommentare erklaeren das „Warum", nicht das „Was"
  (Skill [18](docs/skills/18-kommentare-warum.md)).
- `tc-core` bleibt GTK-frei und headless testbar; die UI greift nie direkt
  aufs Dateisystem zu.
- Conventional Commits (Skill [31](docs/skills/31-conventional-commit.md)).

**Commit-Autor-Pflichtregel (Owner-Spec, uebernommen aus Chimera):**
**JEDER** Commit traegt den Owner als `author` — `pirx <andreas.rose@framefield.com>`.
Das gilt auch fuer Commits, die ein Agent vollstaendig selbst erarbeitet hat:
die Urheberschaft am Projekt liegt beim Owner, der Agent-Anteil steht im
`Co-Authored-By`-Trailer. In jeder frischen Arbeitsumgebung VOR dem ersten
Commit:

```bash
git config user.name "pirx"
git config user.email "andreas.rose@framefield.com"
```

Nach dem ersten Commit einer Session `git log -1 --format='%an <%ae>'`
gegenpruefen.

Development practices, Workflow und der Mehr-Tages-Zyklus (Feature → Tests →
Doku → Refactor) stehen in
[docs/good-development-practices.md](docs/good-development-practices.md) —
vor nicht-trivialer Arbeit lesen.

## Skill-Trigger (wann welcher [docs/skills/](docs/skills/CLAUDE.md))

Kurzes Regelwerk: in welcher Situation ist welches Skill anzuwenden. Wenn
ein Trigger zieht, wird das Skill obligatorisch — Index + Begruendung in den
Einzeldateien.

| Trigger / Situation | Skill |
|---|---|
| Mehrdeutige Aufgabe ohne klare Optionen | [01](docs/skills/01-clarify-with-options.md) Optionen anbieten |
| Aussage ueber Verhalten/Code/Anforderung steht an | [65](docs/skills/65-nichts-erfinden-verifizieren-oder-fragen.md) verifizieren oder nachfragen — nie aus Annahme |
| User stellt Meta-/Status-Frage | [02](docs/skills/02-meta-fragen-direkt.md) direkt antworten |
| Destruktive / Shared-State-Aktion (push, reset, delete) | [03](docs/skills/03-bestaetigung-destruktiv.md) erst Bestaetigung |
| Plan freigegeben → mehrere Phasen | [04](docs/skills/04-multi-phasen-autonomie.md) autonom durchziehen + [11](docs/skills/11-mehrphasen-commits.md) Phase = Commit + [49](docs/skills/49-plan-abschluss-refaktorierungs-audit.md) letzte Phase = Refaktorierungs-Audit |
| Task laeuft > 5 min | [05](docs/skills/05-updates-bei-langen-tasks.md) Zwischenupdate |
| Eigene Pause / Warte-Modus | [06](docs/skills/06-pausen-ansagen.md) explizit ansagen |
| User schlaeft / offline | [07](docs/skills/07-nacht-autonomie.md) autonom weiterarbeiten |
| User schreibt ueber anderen Kanal | [08](docs/skills/08-reply-gleicher-kanal.md) gleicher Kanal antworten |
| Aufgabe vage → vor Implementierung | [09](docs/skills/09-scope-vor-impl.md) Scope-Q&A |
| Aufwaendiges Artefakt selbst bauen (Fixture/Setup/Daten) | [51](docs/skills/51-vor-aufwand-fragen.md) erst fragen, ob User es schneller vorbereitet |
| Neuer Plan-Doc | [10](docs/skills/10-plan-lifecycle.md) `YYYY-MM-DD-`, Status, Archiv + [43](docs/skills/43-coverage-vor-umsetzung.md) Phase 0 Coverage-Pre-Check + [45](docs/skills/45-aufwand-schaetzung-kalibrieren.md) Aufwand-Faktor anwenden |
| Major-Dependency-Upgrade | [12](docs/skills/12-major-upgrades-isoliert.md) eigener Branch/Commit |
| Refactor mit „Quick-Hack vs sauber"-Wahl | [41](docs/skills/41-korrektere-variante.md) korrektere Variante |
| Neue Datei vs bestehende erweitern | [13](docs/skills/13-bestehende-dateien-bevorzugen.md) bestehende editieren |
| Zahl/String im Code hardcodiert | [16](docs/skills/16-keine-magic-values.md) Konstante + [17](docs/skills/17-konstanten-zentralisieren.md) zentralisieren |
| Kommentar geschrieben | [18](docs/skills/18-kommentare-warum.md) „Warum", nicht „Was" |
| Check fuer bereits garantierte Invariante | [19](docs/skills/19-keine-defensive-prog.md) weglassen |
| Refactor / Rename | [20](docs/skills/20-keine-bw-compat-shims.md) keine Shims, vollstaendig migrieren |
| Gleiche Logik an 2+ Stellen | [44](docs/skills/44-keine-redundanzen.md) DRY — geteilter Helper |
| Ableitbare Liste/Wert hartkodiert | [53](docs/skills/53-generieren-statt-duplizieren.md) generieren statt duplizieren |
| Feature / Fix umgesetzt | [23](docs/skills/23-tests-pro-commit.md) Tests gleich mit |
| Bestehende Tests aendern | [24](docs/skills/24-tests-nicht-stumm-aendern.md) erklaeren + fragen |
| Vor Commit | [25](docs/skills/25-gruene-suite-vor-commit.md) fmt + clippy + tests + build gruen |
| Test schreiben | [26](docs/skills/26-verhaltens-tests.md) Verhalten statt Struktur |
| File-Ops/Engine-Code testen ODER „warum fand kein Test den Bug?" | [52](docs/skills/52-erhaltungs-invarianten-testen.md) Erhaltungs-Invarianten (Byte-Summen, Datei-Anzahl, Pack↔Unpack-Roundtrip) statt Wert-Asserts |
| Coverage-Luecke entdeckt | [27](docs/skills/27-coverage-luecken-triage.md) Triage erreichbar/defensiv/dead |
| „ist das abgesichert?" / Bug in abgedecktem Code | [59](docs/skills/59-mutations-probe-statt-coverage-prozent.md) Mutations-Probe: Wirkung abschalten, schauen ob die Suite schreit |
| Code-Aenderung mit Doku-Auswirkung | [28](docs/skills/28-doku-im-selben-commit.md) Doku im selben Commit |
| Neue Doku-Datei | [29](docs/skills/29-ein-topic-pro-doku.md) ein Topic + cross-link |
| Designentscheidung getroffen | [30](docs/skills/30-warum-dokumentieren.md) „Warum" dokumentieren |
| Commit-Message schreiben | [31](docs/skills/31-conventional-commit.md) Conventional-Format |
| Mehrstufige Aufgabe | [32](docs/skills/32-commit-pro-schritt.md) ein Schritt = ein Commit sofort |
| Git-Hook schlaegt fehl | [33](docs/skills/33-hooks-nicht-skip.md) nicht skip, Ursache fixen |
| `git push --force` auf Shared Branch | [34](docs/skills/34-kein-force-push-shared.md) niemals ohne explizite Freigabe |
| `git push` ansteht | [57](docs/skills/57-push-traegt-alles-mit.md) `git log @{u}..HEAD` — Freigabe gilt fuer den ganzen Stapel |
| Datei mit Secret / `git add -A` | [38](docs/skills/38-secrets-nicht-ins-repo.md) explizit adden, keine Secrets |
| Memory- oder Audit-Report referenziert | [39](docs/skills/39-memory-verifizieren.md) Stand pruefen bevor umgesetzt |
| Groesseres Subsystem nach Feature-Burst | [40](docs/skills/40-vier-phasen-zyklus.md) Feature → Tests → Doku → Refactor |
| Hintergrund-Job abbrechen / `pkill`/`pgrep -f` | [61](docs/skills/61-hintergrund-prozesse-sauber-killen.md) Prozess-GRUPPE killen + ps-Nachkontrolle |
| Aussage ueber ALLE Katalog-/Config-Eintraege | [58](docs/skills/58-block-extraktion-statt-einzeilen-grep.md) Block-Extraktion statt Einzeilen-Grep |
| Skript mit schwer rekonstruierbarer Config | [68](docs/skills/68-wichtige-skripte-committen.md) ins Repo committen, nicht scratchpad |
| Skript/Job laeuft absehbar > ein paar Minuten | [71](docs/skills/71-lange-skripte-melden-fortschritt.md) laufender Etappen-Status mit Zeitstempel |
| Owner gibt Richtung vor + bekannter besserer Ansatz existiert | [73](docs/skills/73-proaktiv-bessere-ansaetze-vorschlagen.md) proaktiv vorschlagen — frueh, nicht erst am Plateau |
| Architektur-Review / Code-Audit (Codebasis > 10 k LOC) | [47](docs/skills/47-architektur-audit-mit-subagents.md) parallele Subagents, Synthese in Plan-Doc |

Vollindex + Begruendungen: [docs/skills/CLAUDE.md](docs/skills/CLAUDE.md).
