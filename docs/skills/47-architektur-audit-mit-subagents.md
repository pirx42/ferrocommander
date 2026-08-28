# Skill: Architektur-Audit mit parallelen Subagents

**Wann.** Sobald ein Auftrag „Architektur-Review", „Code-Audit",
„Refactor-Roadmap", „Senior-Architect-Perspektive" oder ein
Sub-System-Audit ueber **> 10 000 LOC** verlangt. Auch passend, wenn
ein Plan einen `Phase 4 — Architektur / Performance / Redundanz +
Refactor`-Block braucht (siehe
[40-vier-phasen-zyklus.md](40-vier-phasen-zyklus.md)) und der
Subsystem-Scope so gross ist, dass eine Single-Thread-Lesung den
Kontext sprengt.

**Regel.** Audits ab dieser Groesse werden NICHT sequenziell vom
Main-Loop gelesen, sondern **in 4-6 parallele Subagents zerlegt**, jeder
mit einer klaren, nicht ueberlappenden Audit-Achse. Synthese in einem
Plan-Doc danach.

**Warum.** Drei Effekte:

1. **Token-Oekonomie.** Ein Subagent liest die Hot-Dateien selbst und
   liefert nur einen 600-800-Wort-Report zurueck. Bei 6 Agents x
   ~150 k Subagent-Tokens fliesst die Roh-Lesung NICHT in den
   Main-Loop — der bleibt frei fuer Synthese + User-Dialog.
2. **Echte Parallelitaet.** 6 Agents in 20-30 Minuten Wandzeit liefern
   das, was sequenziell 2-3 h kostet. Wandzeit ist die knappe Ressource;
   parallele Subagents kosten nur Tokens.
3. **Bias-Reduktion.** Jeder Agent kennt nur seinen Audit-Brief — keine
   Quervernetzung mit den Befunden der anderen. Wenn drei unabhaengige
   Agents dieselbe Schwaeche aus verschiedenen Winkeln nennen, ist sie
   robust validiert (z.B. „App.tsx zu gross" sowohl im Hook-
   Komposition- als auch im Dependency-Audit).

**How (sieben Schritte).**

1. **Quantitative Baseline** (Main-Loop selbst).
   - `find src -name '*.ts' -o -name '*.tsx' | grep -v test | wc -l`
   - `find src/__tests__ -name '*.test.ts' | wc -l`
   - LOC pro Subdir-Cluster:
     `for d in src/{run,components,hooks,renderer,simulation,…}; do
        echo "$(find $d -maxdepth 1 -name '*.ts' -o -maxdepth 1 -name '*.tsx' | grep -v test | xargs cat | wc -l)  $d"
      done | sort -rn`
   - Verschafft Skalen-Verstaendnis BEVOR die Audits starten — wer
     ist gross, wer ist klein, wo ist Test-Druck.

2. **Audit-Achsen schneiden.** Pro Audit-Auftrag 4-6 Achsen waehlen,
   die *nicht* ueberlappen. Beispiele aus dem 2026-06-03-Lauf:
   - App.tsx Hook-Komposition + State-/Effekt-Orchestration
   - Sim/Engine-Layer (Pure-vs-Side-Effect, Headless-Parity, Determinismus)
   - Module-Layering + Dependency-Cycles (mit `madge --circular`)
   - Extension-Points + Plugin-Patterns (Handler-Maps, Catalog, Codegen)
   - Test-Strategie + Coverage (Pyramide, Mock-Dichte, Snapshot-Netz)
   - Renderer + Cross-Cutting (i18n, perfMetrics, persistence,
     Audio)
   Pro Achse ein Subagent. Wenn weniger als 4 Achsen sinnvoll sind, ist
   das Problem zu klein fuer dieses Skill — dann sequenziell lesen.

3. **Subagent-Briefs schreiben.** Jeder Brief enthaelt:
   - **CONTEXT** (2-3 Saetze: was ist Chimera, wo lebt dieses Subsystem,
     was sagt die existing Doku dazu).
   - **AUFGABE** (eine Saetze auf Senior-Architect-Niveau).
   - **Konkret zu pruefen:** 6-8 Sub-Fragen, jede mit Datei-Pfaden +
     Symbol-Namen als Anker, damit der Agent direkt einsteigen kann.
   - **Tools-Vorschlag:** welche Dateien Read, welche grep-Patterns,
     welche externen Tools (madge, vitest --coverage).
   - **Output-Format:** „max 800 Woerter" + feste Sektionen
     (Quantitative Befunde / 3-5 Staerken / 3-5 Schwaechen mit
     Datei:Zeile / Verbesserungsvorschlaege priorisiert / Vergleich mit
     Standard-Patterns).
   - **Keine Code-Aenderungen, nur Recherche + Report** (explizit, sonst
     fummelt der Agent am Code).
   Alle Briefs `run_in_background: true` in einem einzigen Message-Block
   absenden, damit sie wirklich parallel laufen.

4. **Plan-Doc-Skeleton parallel anlegen** (Main-Loop).
   - Datei `docs/plans/YYYY-MM-DD-{topic}.md` (Skill 10).
   - Sektionen: Executive Summary / Architektur-Snapshot /
     Standardisierte Prinzipien-Checks / Findings nach Subsystem /
     Quer-Schnitts-Themen / Verbesserungsvorschlaege priorisiert /
     Verdict.
   - Optional schon waehrend die Agents laufen — Skeleton ohne Inhalt.

5. **Synthese nach Eintreffen aller Reports.**
   - Pro Subsystem 3-5 Staerken + 3-5 Schwaechen mit Datei:Zeile-Ankern
     uebernehmen.
   - Quer-Schnitts-Themen finden: was haben mehrere Reports gemeinsam
     genannt? (z.B. „Locale-Bypass" im Renderer-Audit + „i18n ohne
     Schema-Validation" im Extension-Audit → ein Quer-Befund).
   - **Standardisierte Prinzipien-Checks** als eigene Sektion: SOLID
     (S+O+I+D, L meist n/a fuer TS Discriminated Unions), DRY,
     Layering/Onion/Clean, Hexagonal, CQRS, Test-Pyramid,
     Plugin-Patterns. Jedes mit Score + 1-Satz-Befund.
   - Verbesserungsvorschlaege in drei Buckets: **Quick Wins** (je < 4 h),
     **Mid** (je 1-3 Tage), **Big** (je 1-2 Wochen, eigener Plan). Pro
     Vorschlag: Massnahme + Effort + Effekt.

6. **Verdict + Empfohlene erste Schritte.** Letzte Sektion explizit:
   was ist die Codebasis insgesamt, wo steht sie verglichen mit ihrer
   Groessenklasse, und welche 5 Schritte werden konkret als Erstes
   empfohlen (in der Reihenfolge). Macht den Plan-Doc fuer den User
   actionable.

7. **Telegram-Report + Push.** Plan-Doc committen + pushen, dann
   Telegram-Reply mit:
   - Datei-Pfad + LOC + Commit-Hash.
   - 5-7 Zeilen Kurzfassung (Stark-Schwach-Verdict).
   - Frage nach naechstem Schritt (in der Regel: einen der Quick-Win-
     Cluster als Folge-Plan rausziehen, siehe Beispiel
     `2026-06-03-arch-review-qw1-qw2-qw4.md`).

**Wann _nicht_ noetig.**
- Codebasis < 10 000 LOC oder Audit-Scope < 3 Subsysteme — dann reicht
  Single-Thread-Lesung + Plan.
- Reines Bug-Audit oder Single-Hot-Spot-Diagnose — Subagent waere
  Overhead.
- User hat explizit „kurz" / „ueberblickartig" verlangt — Subagent-Aufwand
  passt nicht zum Auftrag.

**Anti-Pattern.**
- Subagents sequenziell starten (foreground): killt den Parallelitaets-
  Vorteil. **Alle in einem Tool-Use-Block absenden.**
- Briefs ohne konkrete Dateipfade — der Agent grept dann blind und
  findet die wichtigen Stellen nicht.
- Output-Format vergessen → Reports kommen in 4 verschiedenen Stilen
  zurueck, Synthese wird unnoetig schwer.
- Synthese-Sektion „Findings nach Subsystem" 1:1 aus den Reports kopieren
  ohne Quer-Schnitts-Themen + Standardisierte Prinzipien-Checks zu
  ergaenzen — der Main-Loop muss den Mehrwert liefern, sonst war
  Parallelitaet eine teure Token-Auswahl.
- Verdict-Sektion vergessen → Plan wird zur Wand aus Befunden, User
  weiss nicht wo anfangen.

**Verwandt.**
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Output ist ein
  Plan-Doc mit `YYYY-MM-DD-`-Prefix und spaeter Archiv-Move.
- [09-scope-vor-impl.md](09-scope-vor-impl.md) — die 4-6 Audit-Achsen
  sind der Scope; nicht mittendrin um weitere Aspekte erweitern.
- [40-vier-phasen-zyklus.md](40-vier-phasen-zyklus.md) — passt in
  Phase 4 (Architektur/Refactor) als Analyse-Werkzeug.
- `42-housekeeping-coverage-doku-drift.md` (nur Chimera)
  — Audit-Befunde, die als „organische Drift" identifiziert wurden,
  wandern ins Daily-Housekeeping.
- [45-aufwand-schaetzung-kalibrieren.md](45-aufwand-schaetzung-kalibrieren.md)
  — Quick-Win/Mid/Big-Klassifikation der Vorschlaege nutzt die
  empirischen Aufwand-Faktoren.

**Beispiel-Lauf (2026-06-03).** Auftrag: „erstelle Architektur-Analyse
auf High + Mid Level mit Prinzipien-Check". 75k LOC, 5271 Tests. 6
Subagents in ~25 Minuten Wandzeit, ~710 k Subagent-Tokens, Resultat:
[`docs/plans/archive/2026-06-03-architecture-review.md`](../plans/archive/2026-06-03-architecture-review.md)
(481 LOC). Findings: 63 Cycles, App.tsx 5 Forward-Ref-Bruecken, 3
Determinismus-Leaks, 0 UI-Tests, plus konkrete Quick-Wins/Mid/Big-Roadmap.
Folge-Plan
[`2026-06-03-arch-review-qw1-qw2-qw4.md`](../plans/archive/2026-06-03-arch-review-qw1-qw2-qw4.md)
hat die ersten drei Quick Wins umgesetzt (Commits `4ce131d4`, `23e162fa`,
`285b1959`).
