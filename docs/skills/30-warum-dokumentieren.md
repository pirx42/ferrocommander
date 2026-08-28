# Skill: Das „Warum" hinter Design-Entscheidungen dokumentieren

**Wann.** Du triffst eine Architektur-Entscheidung, die nicht aus dem Code
ablesbar ist: warum pure Reducer statt OOP? warum SQLite statt Postgres?
warum diese Zustandsmaschine und nicht eine andere? warum dieses
Refactor genau jetzt?

**Regel.** Wenn die Entscheidung **nicht aus dem Code ablesbar** ist,
gehoert die Begruendung in die Doku — zumindest ein Absatz in der
passenden `docs/<subsystem>.md` oder im Plan-Dokument.

**Was NICHT in die Doku gehoert** (siehe Anti-Patterns unten):
- API-Signaturen, Datei-Pfade — die lebt im Code.
- Commit-Historie — `git log` ist authoritativ.
- Bug-Fix-Recipes — der Fix steht im Code, der Commit-Body hat den
  Kontext.

**Warum.** Sechs Monate spaeter weiss niemand mehr, warum X so ist. Ohne
Doku entsteht die Versuchung, X wegzuoptimieren und dieselben alten
Fallstricke neu zu erleben.

**How.**
- Bei jedem groesseren Refactor / Architektur-Entscheidung: einen
  Absatz „Warum dieser Ansatz?" mit ein paar Alternativen und
  Trade-offs.
- Im passenden `docs/<subsystem>.md` oder Plan-Doku.
- Knapp halten — 3–5 Saetze reichen oft.

**Beispiel.**
```markdown
## Pure Reducer statt OOP-Game-State

Game-State ist ein flacher Immutable-Tree, der per pure Reducer
transformiert wird (`advanceToNextSection(state, ...) → state`).

Alternativen verworfen:
- **OOP mit Mutation:** macht Bot-Sim und RL-Rollouts schwerer
  (jedes Rollout muss kopieren).
- **Redux-Toolkit:** zusaetzliche Abhaengigkeit ohne nennenswerten
  Mehrwert ueber `useState(s => reducer(s, ...))`.

Trade-off: viele kleine Object-Spreads. In der Praxis nicht messbar
(siehe Perf-Pass 2026-04-27).
```

**Anti-Pattern.**
- Doku-Absatz, der nur die Signatur von `advanceToNextSection`
  wiederholt — das steht schon im Code.
- „Wir benutzen Pure Reducer." ohne Begruendung — der naechste Refactor
  wird sie ohne Bedenken wegwerfen.

**Verwandt.**
- [18-kommentare-warum.md](18-kommentare-warum.md) (kleinere Skala —
  Inline-Kommentare).
- [28-doku-im-selben-commit.md](28-doku-im-selben-commit.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
