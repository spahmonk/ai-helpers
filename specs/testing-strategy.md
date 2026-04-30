# Стратегия тестирования `ai-helpers`

## Статус
Draft v1

## Назначение
Документ определяет обязательную стратегию тестирования для репозитория `ai-helpers`, в первую очередь для модуля `ctx-lite`.

---

# 1. Цели тестирования

Тестирование должно гарантировать:

1. корректность поведения;
2. безопасность поведения;
3. устойчивость к регрессиям;
4. сохранение качества контекстной компрессии;
5. предсказуемость памяти, shell, stats и audit.

---

# 2. Категории тестов

## 2.1 Unit tests
Покрывают:
- отдельные функции;
- validators;
- path resolution;
- redaction;
- parsers;
- counters;
- ranking primitives.

## 2.2 Integration tests
Покрывают:
- взаимодействие слоёв;
- CLI flows;
- MCP flows;
- shell → compression → stats;
- read/search → pack;
- memory → recall → task;
- audit/doctor integration.

## 2.3 Regression tests
Обязательны для:
- compression rules;
- shell policy;
- memory persistence;
- retention;
- stats calculations;
- audit outputs;
- config interactions.

## 2.4 Adversarial tests
Обязательны для:
- security-relevant diffs;
- auth/device-code output;
- secrets in output;
- CVE lines;
- critical logs;
- misleading noisy outputs;
- dangerous command shapes.

## 2.5 Benchmark tests
Нужны для:
- compression quality;
- search latency;
- recall latency;
- overhead of audit/stats;
- pack latency.

---

# 3. Обязательные тестовые наборы для `ctx-lite`

## 3.1 PathJail suite
Проверяет:
- доступ внутри root;
- запрет вне root;
- symlink escape;
- hard-link regression for multiply-linked regular files where the platform exposes link counts;
- allow_paths;
- canonicalization correctness.

## 3.2 ShellPolicy suite
Проверяет:
- разрешённые команды;
- запрет redirects;
- запрет file write;
- запрет remote fetch;
- запрет shell wrapper patterns;
- правильность причин отказа.

## 3.3 Compression suite
Проверяет:
- git diff safety;
- git status compactness;
- git log preservation;
- grep result integrity;
- log critical line preservation;
- test output preservation.

## 3.4 Auth-sensitive suite
Проверяет:
- сохранение device codes;
- сохранение verification URLs;
- отсутствие агрессивной компрессии auth output.

## 3.5 Memory suite
Проверяет:
- save/load session memory;
- project memory insert/update;
- dedup;
- recall relevance;
- retention cleanup;
- no secret persistence.

## 3.6 Stats suite
Проверяет:
- input/output bytes;
- saved bytes;
- input/output tokens;
- saved tokens;
- per-session/per-tool aggregation;
- global counters.

## 3.7 Audit/Doctor suite
Проверяет:
- понятность doctor output;
- полноту audit trail;
- purge behavior;
- integrity of local-only checks.

---

# 4. Правила тестирования

## 4.1 Любая новая фича должна иметь тесты
Минимум:
- unit или integration;
- один negative case;
- если security-sensitive — несколько.

## 4.2 Любая bugfix-правка должна иметь regression test
Сначала — воспроизводящий тест.
Потом — исправление.

## 4.3 Compression changes без adversarial tests запрещены
Если меняется логика компрессии, нужно проверить worst-case сценарии.

## 4.4 Security-sensitive change требует negative cases
Если меняется:
- shell
- root jail
- redaction
- persistence
- audit

то негативные тесты обязательны.

---

# 5. Fixtures policy

Нужны fixtures для:
- small repo
- medium repo
- noisy logs
- auth flow output
- dangerous diff
- grep/rg outputs
- cargo/pytest outputs
- docker/k8s outputs

Fixtures должны быть:
- локальными;
- детерминированными;
- безопасными;
- без реальных секретов.

---

# 6. Acceptance tests

Нужен набор end-to-end сценариев:

1. обзор проекта;
2. исследование бага;
3. сбор контекст-пака;
4. shell inspection;
5. continuity between sessions;
6. stats visibility;
7. purge/reset behavior.

---

# 7. Release gate

Релиз блокируется, если:
- падает adversarial test;
- нарушен shell deny policy;
- компрессия потеряла critical markers;
- doctor больше не подтверждает local-only invariant;
- stats saved_bytes/saved_tokens считают неверно;
- memory сохраняет секреты.
