# ssh-mcp-client

Личный desktop SSH-клиент для управления своими VPS-серверами (Tauri, Rust + TypeScript). Одновременно поднимает встроенный MCP-сервер, чтобы Claude мог выполнять команды и работать с файлами на тех же SSH-сессиях, что открыты в GUI — с аудит-логом и обязательным подтверждением перед опасными командами.

## Стек

- [Tauri v2](https://tauri.app/) — Rust backend, нативный webview-фронтенд
- [russh](https://github.com/Eugeny/russh) + [russh-sftp](https://github.com/Miyoshi-Ryota/russh-sftp) — SSH/SFTP клиент
- [rmcp](https://github.com/modelcontextprotocol/rust-sdk) — MCP-сервер (официальный Rust SDK)
- [xterm.js](https://xtermjs.org/) — эмулятор терминала
- Vanilla TypeScript + Vite, без фреймворка

## Разработка

```bash
npm install
npm run tauri dev
```

## Архитектура

Одно SSH-соединение на сервер, общее для GUI и MCP (`AppState`, `src-tauri/src/ssh/session.rs`):
- GUI держит на нём PTY-канал (интерактивный терминал для xterm.js).
- MCP `run_command` открывает отдельный exec-канал на каждый вызов — вывод не смешивается с тем, что видно в терминале.
- SFTP (список файлов, загрузка, скачивание) открывает отдельный SFTP-подканал на каждый вызов.

Если MCP обращается к серверу, к которому GUI ещё не подключён, соединение поднимается автоматически.

Общая бизнес-логика лежит в `src-tauri/src/core.rs` и используется и Tauri-командами (`commands.rs`), и MCP-инструментами (`mcp/tools.rs`) — этот паттерн стоит расширять, а не дублировать.

## Модель безопасности

- **Пароли** хранятся только в системном хранилище учётных данных (Windows Credential Manager через крейт `keyring`), никогда не пишутся на диск в открытом виде и не отдаются фронтенду.
- **Опасные команды**: `run_command` сверяет команду с набором паттернов (`src-tauri/src/audit/dangerous.rs`) — `rm -rf`, `dd`, `mkfs`, `shutdown`/`reboot`, `systemctl stop/disable`, запись в `/dev/sd*`, `drop database` и т.п. При совпадении команда **не выполняется**, возвращается причина блокировки и одноразовый токен с TTL. Выполнить её можно только повторным вызовом `confirm_dangerous_command(token)`.
- **Удаление файлов не доступно через MCP** — `delete_remote_file` есть только в GUI, инструмент в MCP не зарегистрирован намеренно.
- **Аудит-лог**: каждый вызов `run_command`/`confirm_dangerous_command` пишется в SQLite (`audit.db` в app data dir приложения) — время, сервер, команда, источник, флаги dangerous/confirmed, код возврата, обрезанный вывод.
- MCP-сервер слушает строго `127.0.0.1`, наружу не смотрит.

## MCP-инструменты

`list_servers`, `connect_server`, `run_command`, `confirm_dangerous_command`, `list_directory`, `upload_file`, `download_file`.

## Подключение к Claude Code

Приложение поднимает MCP-сервер на `http://127.0.0.1:47821/mcp` (порт настраивается в `settings.json` в app data dir приложения) сразу при запуске и держит его, пока приложение открыто. Порт и готовая команда для регистрации также показаны на карточке "MCP" на главном экране приложения.

Зарегистрировать сервер в Claude Code:

```bash
claude mcp add --transport http servertool http://127.0.0.1:47821/mcp
```

После этого Claude сможет вызывать инструменты выше в рамках текущего проекта — приложение должно быть запущено.
