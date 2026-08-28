# Skill: Updates bei langen Aufgaben (~20-min-Kadenz + Meilensteine)

**Wann.** Aufgabe braucht mehr als ~5 Minuten reine Arbeitszeit
(grosser Build, lange Test-Suite, batch-Refactor, Sub-Agent laeuft).

**Regel.** Etwa alle **~20 Minuten** eine kurze Zeile — was passiert gerade,
was ist der naechste Schritt (Kadenz vom User am 2026-07-09 von ~5 min auf
~20 min hochgesetzt, Memory `feedback_long_task_updates`). **Zusaetzlich** bei
jedem abgeschlossenen Meilenstein (Plan-Teilaufgabe, Phase, Sub-Agent-Ergebnis)
ein kurzer Status-Ping — nicht mehr im starren 5-Minuten-Takt. Bei
Sub-Agent-Spawns gilt das Stagger-Update fuer den auf Antwort wartenden
Hauptthread.

**Warum.** Stille ueber mehr als ein paar Minuten erzeugt Unsicherheit:
ist es noch dran, haengt es, ist es vergessen? Kurze Sichtbarkeit kostet
wenig und beruhigt viel.

**How.**
- Eine Zeile reicht: „noch beim build, 70% durch", „warte auf Sub-Agent X,
  ETA ~3 min", „Tests laufen, bisher gruen".
- Bei jedem nennenswerten Fortschritts-Schritt: Mini-Update.
- Bei Hindernissen sofort melden, nicht erst nachdem der Plan implodiert.
- **Hintergrund-Jobs beobachtbar machen** (Vorfall 2026-07-16, msg 15330):
  Output in eine Log-DATEI umleiten (`> job.log 2>&1`), nie nur durch
  `| tail` — sonst ist Fortschritt unsichtbar und „arbeitet" von „haengt"
  nicht unterscheidbar. VOR einer Haenger-Diagnose die Lauf-Konfiguration
  pruefen (Default-Parameter! `SIM_RUNS_PER_BOT`-Default 100 machte aus
  einem Smoke eine Stunden-Kampagne); 100 % CPU beweist nur „rechnet",
  nicht „terminiert".
- **10-Minuten-Kontrollpunkt (PFLICHT, Owner msg 15522):** jeder Lauf
  > ~10 min wird nach SPAETESTENS 10 Minuten inhaltlich geprueft — laeuft
  er WIE GEWUENSCHT (richtiger Scope/Parameter, plausibler Durchsatz,
  erwartete Zwischen-Ausgaben, Restdauer-Hochrechnung)? Bei Abweichung
  sofort abbrechen/korrigieren. Ausloeser: die volle Sim-Config-Suite
  enthielt unbemerkt die simulateAll-Statistik-Kampagne und brannte 1,9 h
  (2026-07-18) — ein 10-min-Log-Blick haette es sofort gezeigt.
- **Monitoring braucht keine Rueckfrage** (Blanket-Permission 2026-04-30,
  Memory `feedback_monitor_autonomy`): `Monitor`/`tail -F`/`watch`/
  `inotifywait` auf beliebige Prozesse/Dateien unter `/home/pirx/projects`
  + `/tmp` + zugaengliche Logs einfach starten — proaktiv, ohne Vorab-Frage.
  Externe Hosts/SSH bleiben permission-pflichtig.

**Beispiel.**
```
[t=0]   Starte build:rl-worker + 50-Runs-Sweep (~40 min gesamt).
[t=8]   10-min-Kontrollpunkt: richtiger Scope, Durchsatz plausibel.
[t=20]  Meilenstein build durch, Sweep bei 20/50, reward 12.3, keine Crashes.
[t=40]  Fertig, 50/50 gruen — Ergebnis-Ping.
```

**Anti-Pattern.**
- Eine halbe Stunde schweigen, dann fertig ohne Zwischenmeldung.
- Umgekehrt: im 5-Minuten-Takt inhaltslos pingen (Kadenz wurde bewusst gelockert).
- „warte" / „bin gleich" — nichtssagende Fueller.

**Verwandt.**
- [06-pausen-ansagen.md](06-pausen-ansagen.md)
- [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)
