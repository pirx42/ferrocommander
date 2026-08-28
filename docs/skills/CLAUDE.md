# docs/skills/

← Parent: [../CLAUDE.md](../CLAUDE.md)

Einzelne, fokussierte Skills — abgeleitet aus
[../good-development-practices.md](../good-development-practices.md).
Jede Datei ist ein **eigenstaendiges Skill** mit Trigger (`Wann`), Regel,
Begruendung (`Warum`), Anwendung (`How`), Anti-Pattern und Cross-Links.

Pro Skill ein File. Cross-Links zwischen verwandten Skills.

> **Herkunft:** uebernommen aus dem Chimera-Projekt (Stand 2026-08-28).
> Die Original-Nummerierung ist beibehalten, damit Cross-Links stabil bleiben —
> Luecken in der Nummernfolge sind Chimera-spezifische Skills (React-Hooks,
> Item-Catalog, RL-Training, Server-Deploy, i18n-Mandat), die hier nicht
> uebernommen wurden. Beispiele in den Texten, die npm/vitest/tsc nennen,
> gelten sinngemaess mit den Cargo-Aequivalenten (fmt/clippy/test/build).
> Verweise auf `msg NNNNN` sind Provenienz aus der Chimera-Historie.
> Neue Skills fuer dieses Projekt bekommen Nummern ab 100.

## Index

### Kommunikation & Workflow
- [01-clarify-with-options.md](01-clarify-with-options.md) — Mehrdeutige Aufgaben mit benannten Optionen klaeren.
- [02-meta-fragen-direkt.md](02-meta-fragen-direkt.md) — Meta-Fragen direkt beantworten.
- [03-bestaetigung-destruktiv.md](03-bestaetigung-destruktiv.md) — Bestaetigung fuer destruktive / shared-state Aktionen.
- [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md) — Multi-Phasen-Plaene autonom durchziehen.
- [05-updates-bei-langen-tasks.md](05-updates-bei-langen-tasks.md) — Updates bei langen Aufgaben (~20-min-Kadenz + Meilenstein-Pings).
- [06-pausen-ansagen.md](06-pausen-ansagen.md) — Pausen explizit ansagen.
- [07-nacht-autonomie.md](07-nacht-autonomie.md) — Nacht-/Offline-Autonomie.
- [08-reply-gleicher-kanal.md](08-reply-gleicher-kanal.md) — Auf demselben Kanal antworten.
- [51-vor-aufwand-fragen.md](51-vor-aufwand-fragen.md) — Vor aufwaendigem Selbst-Bauen (Fixtures/Setups/Daten) kurz fragen, ob der User es schneller vorbereiten kann.
- [65-nichts-erfinden-verifizieren-oder-fragen.md](65-nichts-erfinden-verifizieren-oder-fragen.md) — Verhaltens-/Fakten-Aussagen NIE aus Annahme: verifizieren oder nachfragen.
- [73-proaktiv-bessere-ansaetze-vorschlagen.md](73-proaktiv-bessere-ansaetze-vorschlagen.md) — Bekannte bessere Ansaetze IMMER frueh als Vorschlag daneben stellen; Entscheidung beim Owner.

### Planung & Scope
- [09-scope-vor-impl.md](09-scope-vor-impl.md) — Scope vor Implementierung festzurren.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Plan-Dateinamen + Lifecycle (Entwurf → Archiviert).
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md) — Mehrphasige Aenderungen in separate Commits.
- [12-major-upgrades-isoliert.md](12-major-upgrades-isoliert.md) — Major-Dependency-Upgrades isoliert.
- [41-korrektere-variante.md](41-korrektere-variante.md) — Korrektere Variante statt Quick-and-Dirty.
- [43-coverage-vor-umsetzung.md](43-coverage-vor-umsetzung.md) — Plan-Phase 0: vorhandene Coverage pro modifizierter Funktion sichten, Luecken vorher nachziehen.
- [45-aufwand-schaetzung-kalibrieren.md](45-aufwand-schaetzung-kalibrieren.md) — Plan-Aufwand-Schaetzung mit empirischen Faktoren korrigieren.
- [49-plan-abschluss-refaktorierungs-audit.md](49-plan-abschluss-refaktorierungs-audit.md) — Letzte Plan-Phase = Refaktorierungs-Audit der Gesamt-Implementierung.

### Code-Qualitaet
- [13-bestehende-dateien-bevorzugen.md](13-bestehende-dateien-bevorzugen.md) — Bestehende Dateien editieren statt neue anlegen.
- [16-keine-magic-values.md](16-keine-magic-values.md) — Magic Values als benannte Konstanten.
- [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md) — Subsystem-Konstanten zentralisieren.
- [18-kommentare-warum.md](18-kommentare-warum.md) — Kommentare erklaeren das „Warum".
- [19-keine-defensive-prog.md](19-keine-defensive-prog.md) — Vertraue Framework-/Typsystem-Garantien.
- [20-keine-bw-compat-shims.md](20-keine-bw-compat-shims.md) — Keine Backward-Compat-Reste beim Refactor.
- [44-keine-redundanzen.md](44-keine-redundanzen.md) — Keine Duplikate / gleiche Logik in geteilte Helper.
- [53-generieren-statt-duplizieren.md](53-generieren-statt-duplizieren.md) — Ableitbare Listen/Werte beim Build aus der Source-of-Truth generieren statt hartkodiert duplizieren.
- [58-block-extraktion-statt-einzeilen-grep.md](58-block-extraktion-statt-einzeilen-grep.md) — Katalog-/Config-Audits nie per Einzeilen-Regex; Block-Extraktion oder Struktur laden.
- [68-wichtige-skripte-committen.md](68-wichtige-skripte-committen.md) — Config-tragende Skripte ins Repo committen, nicht scratchpad.
- [71-lange-skripte-melden-fortschritt.md](71-lange-skripte-melden-fortschritt.md) — Lang laufende Skripte emittieren laufend Etappen-Status mit Zeitstempel.

### Testing
- [23-tests-pro-commit.md](23-tests-pro-commit.md) — Tests begleiten Features und Fixes.
- [24-tests-nicht-stumm-aendern.md](24-tests-nicht-stumm-aendern.md) — Erklaeren + fragen vor Test-Aenderung.
- [25-gruene-suite-vor-commit.md](25-gruene-suite-vor-commit.md) — fmt + clippy + tests + build gruen vor jedem Commit (hier auf Cargo adaptiert).
- [26-verhaltens-tests.md](26-verhaltens-tests.md) — Verhaltens-Tests vor Struktur-Tests.
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md) — Coverage-Luecken: erreichbar/defensiv/dead.
- [52-erhaltungs-invarianten-testen.md](52-erhaltungs-invarianten-testen.md) — Erhaltungs-Invarianten testen statt Wert-Asserts (hier z.B.: Byte-Summen bei Copy, Datei-Anzahl bei Move, Roundtrip Pack→Unpack).
- [59-mutations-probe-statt-coverage-prozent.md](59-mutations-probe-statt-coverage-prozent.md) — Wirkung abschalten und schauen, ob die Suite schreit — Coverage misst Ausfuehrung, nicht Behauptung.

### Dokumentation
- [28-doku-im-selben-commit.md](28-doku-im-selben-commit.md) — Doku im selben Commit wie der Code.
- [29-ein-topic-pro-doku.md](29-ein-topic-pro-doku.md) — Ein Topic pro Datei + Verlinkung.
- [30-warum-dokumentieren.md](30-warum-dokumentieren.md) — Design-„Warum" dokumentieren.

### Commits & Git
- [31-conventional-commit.md](31-conventional-commit.md) — Conventional-Commit-Format.
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md) — Ein logischer Schritt = ein Commit, sofort.
- [33-hooks-nicht-skip.md](33-hooks-nicht-skip.md) — Hooks nicht skip, Commits nicht amend.
- [34-kein-force-push-shared.md](34-kein-force-push-shared.md) — Kein force-push auf Shared Branches.
- [57-push-traegt-alles-mit.md](57-push-traegt-alles-mit.md) — Vor jedem Push `git log @{u}..HEAD` pruefen: Freigabe-Regel gilt fuer den ganzen Commit-Stapel.

### Sicherheit & Verifikation
- [38-secrets-nicht-ins-repo.md](38-secrets-nicht-ins-repo.md) — Keine Secrets, explizites git-add.
- [39-memory-verifizieren.md](39-memory-verifizieren.md) — Memory + Audit-Reports vor Umsetzung pruefen.
- [61-hintergrund-prozesse-sauber-killen.md](61-hintergrund-prozesse-sauber-killen.md) — Abbruch von Hintergrund-Jobs ueber die Prozess-GRUPPE + ps-Nachkontrolle.

### Mehr-Tages-Rhythmus
- [40-vier-phasen-zyklus.md](40-vier-phasen-zyklus.md) — Vier-Phasen-Hardening-Zyklus (Feature → Tests → Doku → Refactor).
- [47-architektur-audit-mit-subagents.md](47-architektur-audit-mit-subagents.md) — Architektur-Audits ab > 10 k LOC mit parallelen Subagents zerlegen.

## Nicht uebernommen (Chimera-spezifisch)

14, 15, 60, 67 (React-Hooks/jsdom) · 21 (TS-Handler-Map — in Rust erledigt
`match` das) · 22 (Chimera-i18n-Mandat) · 35–37, 54 (Server-Deploy) ·
42 (Chimera-Housekeeping-Kommandos; Konzept siehe GDP Teil C.9) ·
46, 48, 50, 55, 56, 62, 63, 64, 66, 69, 70, 72 (Spiel-Content, Playwright,
RL-Training, Chimera-Branch-Modell).

## Quelle

Direkte Quelle: [../good-development-practices.md](../good-development-practices.md).
Skills sind die ausfuehrbaren Einzel-Praktiken; das Dachdokument bleibt die
Gesamterzaehlung.
