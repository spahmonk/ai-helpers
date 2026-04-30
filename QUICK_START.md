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
- **Детальная документация**: смотри [INSTALL.md](docs/INSTALL.md)
- **Проблемы при установке?** → смотри [Troubleshooting](#troubleshooting) ниже

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
