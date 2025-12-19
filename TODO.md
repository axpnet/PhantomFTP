# 📋 Rust FTP Client - TODO List

**Data**: 19 Dicembre 2025  
**Stato Attuale**: TUI funzionante con bug di navigazione da risolvere

---

## 🔴 Bug Critici da Risolvere (TUI)

- [ ] **Navigazione locale non funziona**: Enter/Backspace non navigano nelle cartelle
  - Debug già aggiunto, testare cosa appare nella status bar
  - Potrebbe essere problema di focus o selected_local_file
  
- [ ] **Navigazione remota non funziona**: Il server restituisce file invece di directory
  - Il parsing FTP potrebbe non riconoscere le directory
  - Testare con diversi server FTP per verificare

- [ ] **Verifica selezione file**: Controllare che `selected_local_file` venga aggiornato correttamente

---

## 🟡 Miglioramenti TUI

- [ ] Aggiungere icone diverse per directory/file
- [ ] Mostrare data modifica file
- [ ] Implementare `h` per help popup
- [ ] Progress bar per download
- [ ] Supporto SFTP/FTPS
- [ ] Salvataggio server preferiti
- [ ] Drag & drop con keyboard (selezione multipla)

---

## 🟢 Progetto GUI Separato (rust-ftp-gui)

### Tecnologie Proposte:
- **Tauri** (Backend Rust - riusa `ftp.rs`)
- **React + TypeScript** (Frontend)
- **TailwindCSS + Framer Motion** (UI/Animazioni)

### Features GUI:
- [ ] Design macOS/Finder style con glassmorphism
- [ ] Dark/Light mode toggle
- [ ] Dual-panel layout
- [ ] Drag & drop file upload/download
- [ ] Progress bar animata
- [ ] Sidebar con server salvati
- [ ] Notifiche desktop native
- [ ] Ricerca file
- [ ] Preview immagini/testo

### Setup Progetto GUI:
```bash
# Creare nuovo progetto
npx create-tauri-app@latest rust-ftp-gui
cd rust-ftp-gui

# La libreria ftp.rs può essere condivisa o copiata
```

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
5. ✅ Pubblicato su GitHub (axpnet/rust-ftp-tui)
6. ✅ Aggiunto GitHub Actions per CI/CD
7. ✅ Implementato browser file locali reale
8. ⏳ Debug navigazione in corso

### Prossima sessione:
1. Risolvere bug navigazione
2. Testare su server FTP diversi
3. Decidere se GUI separata o integrata
4. Iniziare sviluppo GUI se deciso

---

## 🔗 Link Utili

- **Repo TUI**: https://github.com/axpnet/rust-ftp-tui
- **Tauri Docs**: https://tauri.app/
- **Ratatui Docs**: https://ratatui.rs/
- **SuppaFTP**: https://github.com/veeso/suppaftp

---

*Buona notte! 🌙 Ci vediamo domani per continuare!*
