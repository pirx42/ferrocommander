# Skill: Gruene Suite vor jedem Commit

**Wann.** Vor jedem `git commit`.

**Regel.** Format-Check, Lint, die komplette Test-Suite und der Build
laufen **gruen**, bevor ein Commit gemacht wird. Wenn einer rot ist:
**fixen, nicht committen**.

**Warum.** Rote Commits verbrennen Zeit fuer jeden, der danach checkt
out. Bisect bricht. CI-Credits werden verschwendet. „Fix ich gleich nach"
ist in der Praxis oft „fix ich in drei Wochen".

**How.**

Standard-Sequenz (Rust-Workspace):
```bash
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --release
```
Bei reinen Doku-Commits kann die Suite entfallen; bei Code-Aenderungen ist
sie Pflicht.

`cargo clippy` mit `-D warnings` ist das CI-Gate: EIN Warning laesst den
Workflow fehlschlagen — auch in Test-Code.

Wenn ein Schritt rot ist:
- **fmt rot:** `cargo fmt --all` laufen lassen.
- **clippy rot:** Regel-Verstoss → Code umbauen, nicht die Lint per
  `#[allow]` disablen (ausser mit dokumentiertem Grund).
- **test rot:** siehe [24-tests-nicht-stumm-aendern.md](24-tests-nicht-stumm-aendern.md)
  zur Entscheidung (Test korrekt? Code korrekt?).
- **build rot:** Feature-Flags, missing dep, release-spezifischer Pfad.
  Nicht „funktioniert ja in debug".

**Beispiel-Routine.**
```bash
# Nach jeder Code-Aenderung:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release

# Erst dann:
git add <files>
git commit -m "..."
```

**Nach einem Merge: kein zweiter Lauf ohne Aenderung.** Das Gate laeuft VOR
dem Merge auf dem Topic-Branch. Ist der Ziel-Branch nach dem Merge inhaltlich
identisch mit dem gemergten Stand — keine Datei, weder Code noch Daten,
unterscheidet sich —, dann hat der Merge nichts getestet-Neues erzeugt und die
Suite wird NICHT erneut ausgefuehrt (Owner-Spec 2026-08-14). Ein zweiter Lauf
beweist dort nichts, kostet Minuten und produziert nur neue Gelegenheiten fuer
Flakes.

Die Pruefung ist ein Baum-Vergleich, kein Gefuehl:
```bash
git diff --quiet topic/<plan> dev && echo "identisch → kein Re-Run"
```
Ist der Diff NICHT leer, ist der gemergte Stand ein anderer als der getestete
(fremde Commits auf dev dazwischen, Konflikt-Aufloesung, Nachbesserung im
Merge-Commit) — dann laeuft das volle Gate auf dem Merge-Ergebnis.

**Anti-Pattern.**
- `git commit` ohne `cargo test`.
- **Pipe-Exit-Falle:** `cargo test | tail -3` (o.ae.) maskiert den Exit-Code —
  die Pipe liefert den Status von `tail`, ein `&&`-Gate laeuft trotz roter
  Tests weiter (real passiert 2026-07-05: 3 Failures unbemerkt). Entweder
  `set -o pipefail` setzen oder die Summary-Zeile explizit auf
  `failed` pruefen.
- „nur tsc gemacht, build vergessen" — der Build deckt Path-Aliase und
  Production-spezifische Pfade ab.
- Rot lassen mit Notiz „flaky" — meistens ist es nicht flaky, sondern
  echt kaputt.

**Verwandt.**
- [33-hooks-nicht-skip.md](33-hooks-nicht-skip.md) (Pre-commit-Hooks
  fangen das automatisch).
- [23-tests-pro-commit.md](23-tests-pro-commit.md)
