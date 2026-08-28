# Skill: Mehrdeutige Aufgaben mit benannten Optionen klaeren

**Wann.** Vor der ersten Codezeile eines Tasks, der unausgesprochene Annahmen
oder mehrere plausible Umsetzungen hat (Skip-Penalty: 1 / alle / keine?
Confirm-Dialog ja/nein? Welche Sektoren betroffen?).

**Regel.** Stelle die Frage **mit benannten Optionen (A/B/C)** und kurzen
Tradeoffs — nicht als offene Frage. Warte auf die Antwort, beginne nicht
spekulativ.

**Warum.** Jede Sekunde Klarstellung vor der Arbeit spart Stunden Rueckbau.
Optionen sind leichter zu beantworten als "wie soll das genau sein?".

**How.**
- Identifiziere 2–4 plausible Varianten.
- Pro Variante: ein Satz „was passiert" + ein Satz „Tradeoff".
- Markiere ggf. eine als Empfehlung („(Recommended)").
- Stoppe nach der Frage. Keine vorgeschlagene Implementierung waehrend du
  wartest.

**Beispiel.**
```
Skip-Button — drei Optionen:
  A) Skip ohne Penalty (frei). Verlockt zum durchklicken.
  B) Skip mit 1 zufaelliger Penalty. Konsistent mit Fail-Modell.   (Recommended)
  C) Skip mit allen Penalties. Hart, koennte frustrierend sein.
Bitte waehlen.
```

**Anti-Pattern.**
- "Soll ich Skip einbauen? Wie genau?" — offene Frage ohne Optionen.
- Direkt Option B implementieren und dann fragen „passt das so?".

**Verwandt.**
- [02-meta-fragen-direkt.md](02-meta-fragen-direkt.md)
- [09-scope-vor-impl.md](09-scope-vor-impl.md)
