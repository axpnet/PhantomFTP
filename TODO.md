# 📋 Rust FTP Client - TODO List

**Data**: 19 Dicembre 2025  
**Stato Attuale**: TUI funzionante con bug di navigazione da risolvere

---

## 🔴 Bug Critici da Risolvere (TUI)

- [x] **Navigazione locale non funziona**: Enter/Backspace non navigano nelle cartelle
  - Debug già aggiunto, testare cosa appare nella status bar
  - Potrebbe essere problema di focus o selected_local_file
  
- [x] **Navigazione remota non funziona**: Il server restituisce file invece di directory
  - Il parsing FTP potrebbe non riconoscere le directory
  - Testare con diversi server FTP per verificare

- [x] **Verifica selezione file**: Controllare che `selected_local_file` venga aggiornato correttamente

---

## 🟡 Miglioramenti TUI

- [x] Aggiungere icone diverse per directory/file
- [ ] Mostrare data modifica file
- [x] Implementare `h` per help popup
- [x] Progress bar per download
- [x] Spinner animato per upload
- [x] Supporto SFTP/FTPS
- [x] Salvataggio server preferiti con supporto FTPS/SFTP
- [ ] Drag & drop con keyboard (selezione multipla)
- [x] Retry automatico per trasferimenti
- [x] Supporto cancellazione trasferimenti (Ctrl+C)
- [x] Anteprima file con tasto 'p'

---

## 🟢 Progetto GUI Separato (rust-ftp-gui)

### Tecnologie Proposte:
- **Tauri** (Backend Rust - riusa `ftp.rs`)
- **React + TypeScript** (Frontend)
- **TailwindCSS + Framer Motion** (UI/Animazioni)

### Features GUI:
- [x] Design macOS/Finder style con glassmorphism
- [x] Dark/Light mode toggle
- [x] Dual-panel layout
- [x] Drag & drop file upload/download
- [x] Progress bar animata
- [ ] Sidebar con server salvati
- [ ] Notifiche desktop native
- [ ] Ricerca file
- [ ] Preview immagini/testo

---

## 📦 Release & Deployment

- [x] GitHub Actions per build Linux/Windows
- [ ] Creare Release v0.1.0 con .deb e .exe
- [ ] Testare su Windows
- [ ] Aggiungere screenshot al README
- [ ] Pubblicare su crates.io (opzionale)

---

## 🏗️ Architettura Proposta

```
/var/www/html/
├── FTP_CLIENT/          # TUI Client (attuale)
│   ├── src/
│   │   ├── ftp.rs       # ← Core FTP logic (condivisibile)
│   │   ├── app.rs       # TUI app state
│   │   ├── ui.rs        # TUI rendering
│   │   └── ...
│   └── Cargo.toml
│
└── rust-ftp-gui/        # GUI Client (nuovo)
    ├── src-tauri/
    │   ├── src/
    │   │   ├── main.rs
    │   │   └── ftp_backend.rs  # Usa ftp.rs o lo importa
    │   └── Cargo.toml
    └── src/              # React frontend
        ├── App.tsx
        └── components/
```

### Opzioni per condividere il codice FTP:
1. **Crate separato** `rust-ftp-core` usato da entrambi
2. **Copia** di ftp.rs nel progetto GUI
3. **Git submodule** per il core

---

## 📝 Note della Sessione

### Cosa abbiamo fatto:
1. ✅ Analizzato il progetto originale di KIMI K2
2. ✅ Fixato tutte le incompatibilità API (suppaftp, ratatui, tokio)
3. ✅ Compilato con successo
4. ✅ Creato pacchetto .deb
5. ✅ Pubblicato su GitHub (axpnet/PhantomFTP)
6. ✅ Aggiunto GitHub Actions per CI/CD
7. ✅ Implementato browser file locali reale
8. ✅ Risolti bug di navigazione locale e remota
9. ✅ Aggiunte icone per file e directory
10. ✅ Implementata progress bar per i download
11. ✅ Implementato spinner animato per gli upload
12. ✅ Aggiunta guida in-app con tasto 'h'
13. ✅ Aggiornati i crediti nei file README
14. ✅ Risolti potenziali panic nella navigazione
15. ✅ Aggiunto retry automatico per trasferimenti
16. ✅ Aggiunto supporto cancellazione trasferimenti (Ctrl+C)
17. ✅ Aggiunto supporto FTPS per connessioni sicure
18. ✅ Aggiunto supporto SFTP per connessioni SSH
19. ✅ Aggiunto supporto per anteprima file

### Prossima sessione:
1. Testare su server FTP/SFTP diversi
2. Aggiungere la visualizzazione della data di modifica dei file
3. Continuare lo sviluppo della GUI

---

## 🔗 Link Utili

- **Repo TUI**: https://github.com/axpnet/PhantomFTP
- **Tauri Docs**: https://tauri.app/
- **Ratatui Docs**: https://ratatui.rs/
- **SuppaFTP**: https://github.com/veeso/suppaftp

---

*Buona notte! 🌙 Ci vediamo domani per continuare!*