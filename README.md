# Aleph 4 Agents — kit di avvio

> «il luogo dove sono, senza confondersi, tutti i luoghi [...] visti da tutti gli angoli» — J. L. Borges, *L'Aleph*

Questo pacchetto serve a far costruire Aleph a un agente (opencode + DeepSeek) in loop, giorno e notte.
La documentazione per gli agenti è in inglese apposta: costa meno token e i modelli la seguono meglio. Questo README è per noi.

## Cosa c'è

| Percorso | Cosa |
|---|---|
| `spec/SPEC.md` | specifica v0.1 implementabile: lessico, sintassi, tipi, builtin, semantica, permessi, diagnostica, CLI, esempi |
| `spec/grammar.ebnf` | grammatica formale |
| `spec/lexicon.toml` | parole chiave come dati (da ottimizzare sui tokenizer) |
| `adr/` | 14 decisioni architetturali (Rust, AST canonico, sintassi prefissa, backend C, arene, permessi, commit agent-ready, nomi…) |
| `AGENTS.md` | regole per l'agente: fonti di verità, test, formato commit, limiti |
| `TASKS.md` | 31 task ordinati in milestone (M1 core, M2 nativo, M3 tooling, v0.2) |
| `PROGRESS.md` | diario scritto dagli agenti a ogni iterazione |
| `bench/tasks.md` | 15 task per benchmark e per il corpus |
| `loop/prompt-build.md` | prompt per costruire il compilatore |
| `loop/prompt-orbis.md` | prompt per generare il corpus verificato (Orbis), da usare dopo M1 |
| `loop/run.sh` | il loop |
| `opencode.json` | permessi per opencode (blocca push, rete, comandi distruttivi) |
| `docs/aleph-4-agents-spec-v1.pdf` | il documento di progetto |

## Come si avvia

1. **Prerequisiti**: Rust stable (`rustup`), un compilatore C (`cc`), git, [opencode](https://opencode.ai).
2. **DeepSeek**: `opencode auth login`, scegli DeepSeek e inserisci la chiave API. Poi controlla il nome esatto del modello con `opencode models deepseek` e, se è diverso, esportalo:
   `export ALEPH_MODEL=deepseek/<nome-modello>`
3. **Repository**:
   ```bash
   cd aleph && git init && git add -A && git commit -m "init: spec v0.1, ADR, tasks, loop"
   chmod +x loop/run.sh
   ```
4. **Prova breve** (2 iterazioni, guardi cosa fa):
   `MAX_ITER=2 loop/run.sh build`
5. **In background, giorno e notte** (in tmux, così puoi riattaccarti):
   ```bash
   tmux new -s aleph
   loop/run.sh build
   # Ctrl-b d per staccarti; tmux attach -t aleph per tornare
   ```
   oppure `nohup loop/run.sh build > loop/logs/nohup.out 2>&1 &`
6. **Fase Orbis** (quando M1 è completo, il compilatore fa check/test/run): `loop/run.sh orbis`. Si possono far girare entrambi in due copie separate del repository.

## Controllo

- **Fermare**: `touch STOP`. L'iterazione in corso finisce, poi il loop si ferma. Riparti con `rm STOP`.
- **Avanzamento**: `tail -f loop/logs/runs.txt`, `git log --oneline`, `PROGRESS.md`, `TASKS.md`.
- **Freni automatici**: il loop si ferma da solo se non ci sono più task, se fa 4 iterazioni di fila senza commit (`NOPROGRESS`), o a `MAX_ITER`. Ogni iterazione ha un timeout di un'ora (`ITER_TIMEOUT`).
- **Decisioni dell'agente**: quando la spec non basta, l'agente scrive un ADR `Proposed`. Rivedili ogni giorno: siete voi a passarli in `Accepted` o `Rejected`.

## Sicurezza e costi

- Fallo girare in una **VM, un container o un utente dedicato**, mai nella tua home principale: l'agente esegue comandi shell senza chiedere.
- `opencode.json` blocca push, rete, `sudo` e comandi distruttivi, ma i filtri per pattern non sono una sandbox vera.
- Metti un **limite di spesa** sulla chiave DeepSeek. Con il loop continuo i costi si accumulano anche di notte.
- Non mettere segreti nel repository.

## Note

- Non ho potuto provare il loop end-to-end né verificare i nomi attuali dei modelli DeepSeek in opencode: fai la prova breve del punto 4 prima di lasciarlo andare.
- La sintassi v0.1 (prefissa senza parentesi) è il default dell'ADR-0004; il confronto con la variante delimitata è previsto in M5.
