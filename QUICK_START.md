# ⚡ Quick Start Guide

Установить и начать работу с `ctx-lite` за 3 минуты.

## 1️⃣ Установка

### Linux / macOS
```bash
curl -fsSL https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.sh | bash
```

### Windows (PowerShell)
```powershell
powershell -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/spahmonk/ai-helpers/main/scripts/install.ps1'))"
```

### Via npm (Node.js 18+)
```bash
npm install -g @spahmonk/ctx-lite
```

## 2️⃣ Проверка

```bash
ctx-lite --version
ctx-lite --help
```

Вы должны увидеть версию и справку по командам.

## 3️⃣ Первое использование

### Прочитать файл
```bash
ctx-lite read src/main.rs
```

### Показать дерево папок
```bash
ctx-lite tree ./src
```

### Поиск в коде
```bash
ctx-lite search "function_name"
```

### Запустить диагностику
```bash
ctx-lite doctor
```

## 📚 Что дальше?

- **Базовые примеры**: `ctx-lite --help`
- **Детальная документация**: смотри [README](README.md) и [MCP Integration](MCP_INTEGRATION.md)
- **Удаление**: смотри [Uninstall / Cleanup](#4️⃣-удаление-и-очистка)
- **Проблемы при установке?** → смотри [Troubleshooting](#troubleshooting) ниже

## 4️⃣ Удаление и очистка

### Удалить установленный бинарник

**Linux/macOS (install.sh):**
```bash
sudo rm -f /usr/local/bin/ctx-lite
```

Если ставил через `CTX_LITE_INSTALL_DIR`, удаляй `ctx-lite` из этого каталога.

**Windows (install.ps1):**
```powershell
Remove-Item "$env:ProgramFiles\ctx-lite" -Recurse -Force
```

После этого при необходимости убери `%ProgramFiles%\ctx-lite` из **User PATH**, если он там остался.

**npm:**
```bash
npm uninstall -g @spahmonk/ctx-lite
```

### Полная очистка локальных данных

Если хочешь удалить и локальные данные/кэш:

**Linux/macOS:**
```bash
rm -rf ~/.ctx-lite ~/.ctx-lite-cache
```

**Windows:**
```powershell
Remove-Item "$HOME\.ctx-lite","$HOME\.ctx-lite-cache" -Recurse -Force -ErrorAction SilentlyContinue
```

### Что удалять не нужно

- `npx @spahmonk/ctx-lite` обычно не требует отдельного uninstall
- временные файлы инсталляторов удаляются самими скриптами

## 🆘 Troubleshooting

### "ctx-lite: command not found"

**Linux/macOS:**
```bash
# Добавить в PATH
export PATH="$PATH:/usr/local/bin"
# Добавить в ~/.bashrc или ~/.zshrc для постоянного эффекта
```

**Windows:**
- Перезагрузи PowerShell/cmd после установки
- Или вручную добавь `%ProgramFiles%\ctx-lite` в PATH

### Ошибка при скачивании

Убедись что:
1. Интернет работает: `ping github.com`
2. curl установлен: `curl --version`
3. Версия релиза существует на GitHub: https://github.com/spahmonk/ai-helpers/releases

### Permission denied

**Linux/macOS:**
```bash
sudo chmod +x /usr/local/bin/ctx-lite
```

**Windows:**
Запусти PowerShell как администратор и повтори установку.

---

**Готово!** Теперь ты можешь использовать `ctx-lite` для работы с кодом. 🚀
