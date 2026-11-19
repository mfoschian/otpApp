# OTP Manager

OTP Manager è un'applicazione desktop per Windows scritta in Rust che legge backup cifrati di Aegis Authenticator, importa i token OTP e li mostra in un'interfaccia grafica con aggiornamento automatico.

Il programma è pensato per l'uso locale su una singola installazione Windows.

## Funzionalità

- interfaccia grafica desktop basata su `egui`/`eframe`
- importazione di un backup JSON cifrato Aegis tramite una finestra di selezione file
- decifratura della master key Aegis mediante `scrypt`
- decifratura del database Aegis mediante `AES-256-GCM`
- supporto alle entry:
  - TOTP
  - HOTP
  - SHA1, SHA256 e SHA512
- visualizzazione di nome, issuer e codice OTP
- aggiornamento dei codici TOTP ogni secondo
- indicatore circolare con il tempo residuo prima dell'avvicendamento del codice
- cambio colore dell'indicatore in base al tempo residuo
- salvataggio locale del vault cifrato
- memorizzazione della password dell'applicazione nel gestore credenziali Windows

## Come funziona

1. All'avvio l'applicazione recupera la password dal gestore credenziali Windows.
2. Se la password non è ancora disponibile, richiede all'utente di configurarla.
3. Cerca il vault locale in `~/.otp-vault`.
4. Decifra i segreti del vault e genera i codici OTP.
5. Dal pannello `Services` è possibile importare un nuovo backup Aegis.

## Vault locale

Il vault locale viene salvato nella cartella home dell'utente:

```text
<home>\.otp-vault
```

Il file contiene le entry del vault con i segreti cifrati individualmente. La cifratura dei segreti usa `Argon2` per derivare la chiave e `AES-256-GCM` per proteggere il token. La password usata dall'applicazione viene salvata nel Windows Credential Manager tramite `keyring`.

ATTENZIONE: attualmente una volta inserita la password non può essere modificata all'interno dell'applicazione, ma farlo dall'esterno con i gestori opportuni ha come conseguenza la perdita del vault. Per il momento non è quindi possibile cambiare la password senza dover cancellare manualmente il file con il vault e re-importare tutti i segreti.

## Importazione

- Una volta avviata l'applicazione il tasto "Import" permette di importare una file json di export da Aegis (protetto da password).
- Il pannello di import presenta una casella per l'inserimento della password
- Con il tasto "Ok" si può selezionare poi il file json da importare
- Le informazioni importate vengono aggiunte ai servizi OTP attualmente caricati
- Il vault dell'applicazione viene aggiornato


## Requisiti

- Windows
- Rust toolchain con Cargo
- Una versione recente di Rust compatibile con le dipendenze del progetto
- Un backup Aegis in formato JSON cifrato


## Utilizzo

1. Avviare OTP Manager.
2. Inserire la password dell'applicazione al primo avvio.
3. Importare il backup Aegis con il pulsante `Import`.
4. Inserire la password del backup Aegis.
5. Selezionare il file JSON esportato da Aegis.
6. Consultare i codici nella lista dei servizi.

## Sicurezza

- Le chiavi non vengono trasmesse di rete dal progetto.
- Il vault locale contiene segreti OTP cifrati, ma la password dell'applicazione è salvata nel Windows Credential Manager.
- Proteggere il sistema Windows con account utente, password/PIN e cifratura del disco.
- Non condividere il backup Aegis originale né il file `.otp-vault`.
- Il progetto è destinato a uso locale e personale; non costituisce un audit di sicurezza indipendente.
