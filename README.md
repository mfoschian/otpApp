# Aegis Reader

Aegis Reader è un piccolo programma Rust per leggere un backup di Aegis Authenticator, decifrarne il contenuto con una password, e recuperare i codici OTP (TOTP/HOTP) associati ai servizi registrati.

## Funzionalità

- legge un file JSON di backup Aegis
- deriva la master key tramite scrypt
- decripta il database principale con AES-256-GCM
- analizza i servizi e i token salvati nel vault
- genera i codici OTP correnti
- mostra il tempo rimanente di validità del token

## Requisiti

- Rust 1.70+
- Cargo
- Un backup Aegis in formato JSON

## Struttura del progetto

- `src/main.rs`: entry point dell'applicazione
- `src/aegis.rs`: lettura, decifrazione e parsing del vault Aegis
- `src/otp.rs`: generazione dei codici OTP e calcolo del tempo rimanente
- `.vscode/launch.json`: configurazione di debug in VS Code

## Compilazione

```bash
cargo build
```

## Esecuzione

```bash
cargo run -- <percorso_del_backup_aegis.json>
```

Esempio:

```bash
cargo run -- data/backup.json
```

## Password

Il programma richiede la password della cassaforte Aegis tramite input sicuro. In fase di test potrebbe essere utile sostituire temporaneamente la richiesta con una password fissa, ma in produzione va mantenuta l’input interattivo.

## Esempio di output

```text
Lettura del file in corso: data/backup.json

--- OTP RICAVATI DAL VAULT AEGIS ---
[TOTP       ] GitHub (GitHub) | codice: 94287082 | scade tra: 00:01
[TOTP       ] Dropbox | codice: 124385 | scade tra: 00:18
```

## Note

- Il programma supporta le entry TOTP e HOTP presenti nel vault Aegis.
- L’algoritmo supportato include SHA1, SHA256 e SHA512.
- Il backup deve essere un JSON valido prodotto da Aegis Authenticator.

## Sicurezza

Questo progetto è stato creato per scopi educativi e di analisi locale di backup personali. Non condividere mai file sensibili in ambienti non sicuri.
