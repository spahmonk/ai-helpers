# Threat Model для `ai-helpers`

## Статус
Draft v1

## Назначение
Этот документ фиксирует базовую модель угроз для репозитория `ai-helpers` и, в частности, для модуля `ctx-lite`.

---

# 1. Защищаемые активы

Нужно защищать:

- исходный код приватных проектов;
- локальные файлы пользователя;
- shell output;
- архитектурные заметки и память по проекту;
- чувствительные строки и секреты;
- сведения о структуре проекта;
- метаданные разработки.

---

# 2. Основные угрозы

## 2.1 Data exfiltration
Риск:
- отправка кода, памяти, метаданных или telemetry наружу;
- скрытые outbound connections;
- неочевидный sync/update mechanism.

## 2.2 Overbroad file access
Риск:
- чтение данных за пределами project root;
- чтение home/system directories;
- обход root jail через symlink.

## 2.3 Dangerous shell behavior
Риск:
- запись в файлы через shell;
- выполнение нежелательных или опасных команд;
- network-fetch commands;
- shell injection.

## 2.4 Unsafe persistence
Риск:
- сохранение секретов;
- сохранение raw auth output;
- хранение чрезмерного количества чувствительных данных;
- неограниченный рост memory/cache/session state.

## 2.5 Misleading compression
Риск:
- потеря security-relevant lines;
- потеря auth/device code;
- потеря failed/error signals;
- опасная truncation.

## 2.6 Resource abuse
Риск:
- чрезмерное чтение с диска;
- runaway indexing;
- бесконтрольное повторное чтение;
- огромный local state.

---

# 3. Границы доверия

## 3.1 Trusted boundary
Доверенная граница:
- локальная машина пользователя;
- сам репозиторий проекта;
- локальное storage модуля;
- локальный stdio transport.

## 3.2 Untrusted boundary
Недоверенная граница:
- интернет;
- любые cloud services;
- любые сторонние сетевые endpoints;
- произвольные shell-команды вне policy.

---

# 4. Защитные меры

## 4.1 Zero-network core
В core не должно быть outbound network.

## 4.2 PathJail
Все file ops проходят через единый root jail.

## 4.3 CommandPolicy
Все shell-команды валидируются через whitelist-first policy.

## 4.4 RedactionPipeline
Перед выводом и перед записью действует secret redaction.

## 4.5 AuditTrail
Критичные действия отражаются в audit.

## 4.6 Limits/Budgets
Размер чтения, хранения и обработки ограничен budgets.

---

# 5. Security invariants

Система корректна только если одновременно верно:

- нет outbound network connections;
- нельзя читать вне root jail;
- shell не пишет в файлы;
- shell не тянет сеть;
- память не сохраняет raw secrets;
- audit фиксирует чувствительные действия;
- компрессия не скрывает critical markers;
- stats не сохраняют sensitive raw payloads.

---

# 6. Осознанные ограничения

`ctx-lite` не должен пытаться быть:
- удалённым сервисом;
- облачным продуктом;
- автообновляющимся агентом;
- универсальным shell automation engine.

Его задача — быть локальным и предсказуемым runtime-инструментом для контекста и памяти.