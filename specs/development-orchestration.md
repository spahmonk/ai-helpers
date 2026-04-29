# Оркестрация разработки `ai-helpers`

## Статус
Draft v1

## Назначение
Этот документ определяет базовые правила инженерной оркестрации в репозитории `ai-helpers`.

Он нужен для того, чтобы:
- разработка шла последовательно, а не хаотично;
- сложные модули вроде `ctx-lite` проектировались spec-first;
- качество, безопасность и тестируемость были встроены в процесс;
- AI-агенты и люди работали по единым правилам.

---

# 1. Основные принципы

## 1.1 Spec-first
Любая нетривиальная функциональность должна начинаться со спецификации.

Минимум перед реализацией:
- проблема;
- цель;
- scope;
- non-goals;
- риски;
- acceptance criteria.

## 1.2 Security-first
Если функциональность касается:
- чтения файлов,
- shell,
- памяти,
- кэша,
- индексации,
- контекста проекта,
- агентной оркестрации,

то безопасность всегда важнее удобства и скорости доставки.

## 1.3 Explainability-first
Любое нетривиальное поведение системы должно быть объяснимым:
- почему было разрешено/запрещено;
- почему это было сжато именно так;
- почему это попало в память;
- почему это было отброшено;
- почему это было записано в audit/stats.

## 1.4 Small trusted core
Сложность должна выноситься наружу, а ядро должно оставаться компактным и аудируемым.

## 1.5 Regression intolerance
Любая правка, затрагивающая:
- compression,
- memory,
- shell policy,
- root jail,
- recall,
- stats,

обязана сопровождаться regression tests.

---

# 2. Lifecycle разработки фичи

Каждая фича проходит следующие стадии.

## 2.1 Intake
На этой стадии нужно понять:
- что именно нужно сделать;
- зачем;
- кто пользователь;
- как это улучшит продукт;
- какие ограничения есть.

Артефакт:
- issue / task note / spec update

## 2.2 Scope
Нужно определить:
- что входит в первую реализацию;
- что откладывается;
- что запрещено;
- какие зависимости есть;
- какие риски есть.

Артефакт:
- update spec или mini-design

## 2.3 Design
Нужно зафиксировать:
- архитектурный слой;
- API/контракты;
- storage impact;
- diagnostics impact;
- security impact;
- testing impact.

Артефакт:
- update architecture doc / design note

## 2.4 Implementation
Реализация идёт в порядке:
1. types/contracts
2. core logic
3. integration points
4. diagnostics
5. tests
6. docs

## 2.5 Validation
Проверяются:
- happy path;
- negative path;
- limits;
- regressions;
- adversarial cases;
- docs consistency.

## 2.6 Release readiness
Перед merge/release должно быть ясно:
- что фича безопасна;
- что поведение предсказуемо;
- что документация актуальна;
- что тесты реально покрывают рискованные сценарии.

---

# 3. Обязательная декомпозиция задач

Нельзя вести разработку задачами уровня:
- “сделать весь модуль”
- “реализовать полностью runtime”
- “добавить всё нужное”

Нужно декомпозировать задачи по типам:

- `spec`
- `architecture`
- `security`
- `core`
- `integration`
- `tests`
- `docs`
- `release`

## Пример правильной декомпозиции для `ctx-lite`
Не:
- “сделать shell compression”

А:
- описать shell policy
- реализовать command validator
- реализовать whitelist matcher
- реализовать executor limits
- реализовать git diff pattern
- реализовать auth flow preservation
- добавить adversarial tests
- добавить doctor/audit visibility
- добавить stats saved_tokens/saved_bytes

---

# 4. Quality gates

Ни одна задача не считается готовой, если:

- нет описания scope;
- нет тестов;
- нет негативных сценариев;
- не учтён security impact;
- не учтён diagnostics impact;
- не обновлены docs при изменении поведения.

---

# 5. Обязательные артефакты для сложных модулей

Для модулей класса `ctx-lite` должны существовать:

- основной spec
- architecture doc
- threat model
- testing strategy
- release checklist
- regression/adversarial strategy
- AGENTS/process rules

---

# 6. Приоритеты при конфликте целей

Если цели конфликтуют, приоритет такой:

1. безопасность
2. корректность
3. объяснимость
4. наблюдаемость
5. тестируемость
6. качество компрессии
7. производительность
8. удобство расширения

---

# 7. Правила изменения существующей логики

Если меняется:
- compression pattern;
- shell validator;
- memory persistence;
- storage schema;
- audit format;
- stats format;
- root jail behavior;

то обязательно:
- добавить regression test;
- обновить docs/spec;
- проверить backward compatibility;
- обновить release checklist impact.

---

# 8. Definition of Done

Фича считается завершённой только если:

- реализован заявленный scope;
- acceptance criteria выполняются;
- тесты написаны и проходят;
- docs обновлены;
- doctor/audit/stats не противоречат новому поведению;
- не нарушены security invariants.

---

# 9. Основной инженерный ориентир

`ai-helpers` должен развиваться как набор локальных, прозрачных, качественных инженерных модулей, а не как opaque automation bundle.