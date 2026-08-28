# Skill: Erklaeren + fragen, bevor du bestehende Tests veraenderst

**Wann.** Eine Code-Aenderung laesst einen bestehenden Test
fehlschlagen.

**Regel.** Tests werden **nicht einfach „angepasst, damit sie gruen sind"**.
Ein existierender Test hat einen Grund. Wenn eine Aenderung ihn zum
Scheitern bringt:

1. **Zuerst ueberlegen:** soll die Aenderung wirklich das dokumentierte
   Verhalten brechen?
2. Wenn ja: die Aenderung **erklaeren** und — bei echten Verhaltensregeln
   — mit dem Auftraggeber abstimmen.
3. Erst dann den Test mit-anpassen.

**Warum.** Tests sind **kodifizierte Anforderungen**. Einen Test zu
aendern, um einen Build zu reparieren, loescht oft genau die Invariante,
die der Test schuetzt. Memory `feedback_test_changes`: „tests only change
when requirements change, not to silence failures".

**How.**
1. Roten Test analysieren: was war die geschuetzte Invariante?
2. Hypothesen pruefen:
   - **Anforderungs-Aenderung:** Test korrekt anpassen + im
     Commit-Body erklaeren („Verhalten X aenderte sich, Test gerade
     mitgezogen weil ...").
   - **Bug in deinem Code:** dein Code ist falsch, der Test hatte recht
     — Code fixen, Test belassen.
   - **Test-Setup-Drift:** Test verlaesst sich auf zufaellige Details,
     die deine Aenderung neu setzt — Setup minimal stabilisieren, kein
     Assertion-Wert-Aendern.
3. Bei Unsicherheit oder Anforderungs-Konflikt: **Auftraggeber fragen**
   bevor Test geaendert wird.

**Beispiel.**
```
itemCombinations.slow.test.ts: cooler(bypassMod, inverterMod)
expected heat=cooler.heatCooling=300, now needs 0.9*300=270.

Begruendung: invertCount-Modell mit Damping 0.9 wirkt nun per Item,
nicht XOR-aggregiert. Test korrekt mitgezogen — neue formel ist
0.9^N * value. Im Commit-Body verlinkt auf damping plan.
```

**Anti-Pattern.**
- `expect(x).toBe(270)` statt `300` — ohne Kommentar, weil „so geht's
  gruen".
- Test ganz loeschen, weil „dieser Pfad ist jetzt anders". Sicher dass
  niemand mehr darauf laufen soll?
- Bei mehreren Refactor-Schritten: Test in jeder Phase neu „angepasst",
  bis er stumm ist.

**Verwandt.**
- [23-tests-pro-commit.md](23-tests-pro-commit.md)
- [26-verhaltens-tests.md](26-verhaltens-tests.md)
- Memory `feedback_test_changes`.
