# Skill: Meta-Fragen direkt beantworten

**Wann.** Auftraggeber stellt eine geschlossene Statusfrage:
„gepusht?", „committed?", „in welcher Datei?", „laeuft das Service?".

**Regel.** Antwort zuerst. Kontext danach, wenn ueberhaupt noetig.

**Warum.** Ein Absatz Vorrede ist irrelevant, wenn die Frage ein „ja/nein"
oder ein einzelner Hash ist. Direkte Antworten respektieren die
Aufmerksamkeit des Auftraggebers.

**How.**
- 1. Wort: Antwort (Ja / Nein / Hash / Pfad).
- 2. Satz (optional): unmittelbarer Kontext.
- Keine Vor-Rede ("Lass mich kurz schauen ..."), keine Nachschuetzung.

**Beispiel.**
```
gepusht?
→ Ja, Commit abc1234 auf main.

in welcher Datei liegt der HeatManager?
→ src/run/heatManager.ts:42.
```

**Anti-Pattern.**
```
gepusht?
→ Ich habe gerade die Tests laufen lassen, alle sind gruen, dann den Commit
   gemacht und anschliessend...
```

**Verwandt.**
- [01-clarify-with-options.md](01-clarify-with-options.md) (umgekehrte Richtung
  — bei Mehrdeutigkeit ist Frage statt Antwort richtig).
