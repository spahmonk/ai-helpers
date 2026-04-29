# Архитектура `ctx-lite`

## Статус
Draft v1

## Модуль
`ctx-lite`

## Репозиторий
`spahmonk/ai-helpers`

## Назначение
Документ описывает целевую архитектуру модуля `ctx-lite` внутри репозитория `ai-helpers`. Архитектура ориентирована на:
- local-only runtime;
- безопасную работу с приватными проектами;
- качество компрессии контекста на уровне референсного решения;
- прозрачную память по сессии и проекту;
- предсказуемую оркестрацию чтения, поиска, упаковки контекста и тестирования.

Референс по UX, качеству компрессии и инженерной дисциплине:
- `yvgude/lean-ctx`
- release checklist / memory-bank подход как источник идей по процессу качества
- без переноса cloud/sync/update/hook-heavy поведения

---

# 1. Архитектурные принципы

## 1.1 Local-first и zero-network core
Ядро `ctx-lite` не должно иметь исходящих сетевых соединений. Архитектура должна исключать cloud dependencies из core path.

## 1.2 Security by default
Любой модуль должен проектироваться по модели «запрещено всё, кроме явно разрешённого»:
- path jail;
- shell whitelist;
- memory redaction;
- auditability;
- explainability.

## 1.3 Small trusted core
Безопасность достигается за счёт компактного ядра и выноса опциональных возможностей в отдельные слои.

## 1.4 Quality parity with reference
Компрессия и orchestration должны быть сопоставимы с `lean-ctx` на практических сценариях:
- git diff/status/log
- test outputs
- grep/rg outputs
- logs
- repo exploration

## 1.5 Explicit orchestration
Планирование, чтение, память, shell, упаковка контекста и тестирование должны быть представлены как отдельные предсказуемые стадии.

---

# 2. Верхнеуровневая схема

Система делится на 8 слоёв:

1. **Interface Layer**
   - CLI
   - MCP stdio server

2. **Application Layer**
   - orchestration
   - task engine
   - context packer
   - session workflows

3. **Domain Layer**
   - reading
   - search
   - overview
   - compression
   - memory
   - stats
   - audit

4. **Security Layer**
   - path jail
   - command validation
   - redaction
   - policy enforcement

5. **Storage Layer**
   - session store
   - project memory store
   - stats store
   - cache store
   - audit store

6. **Execution Layer**
   - safe shell executor
   - file traversal
   - search engine
   - parser adapters

7. **Policy Layer**
   - config resolution
   - retention policy
   - feature flags
   - limits / budgets

8. **Quality Layer**
   - doctor
   - diagnostics
   - self-checks
   - test fixtures
   - benchmark harness

---

# 3. Предлагаемая структура каталогов

```text
ai-helpers/
  specs/
    ctx-lite.md
    ctx-lite-architecture.md
  modules/
    ctx-lite/
      Cargo.toml
      src/
        lib.rs
        main.rs
        cli/
          mod.rs
          commands/
            read.rs
            multi_read.rs
            tree.rs
            search.rs
            overview.rs
            shell.rs
            task.rs
            pack.rs
            memory.rs
            session.rs
            stats.rs
            doctor.rs
            audit.rs
            purge.rs
        mcp/
          mod.rs
          server.rs
          transport_stdio.rs
          tools/
            mod.rs
            ctx_read.rs
            ctx_multi_read.rs
            ctx_tree.rs
            ctx_search.rs
            ctx_overview.rs
            ctx_shell.rs
            ctx_task.rs
            ctx_pack.rs
            ctx_memory.rs
            ctx_session.rs
            ctx_stats.rs
            ctx_doctor.rs
            ctx_audit.rs
            ctx_purge.rs
        app/
          mod.rs
          orchestration.rs
          task_engine.rs
          context_packer.rs
          workflow.rs
        core/
          mod.rs
          config/
            mod.rs
            model.rs
            loader.rs
          security/
            mod.rs
            path_jail.rs
            command_policy.rs
            redaction.rs
            secret_patterns.rs
          read/
            mod.rs
            file_reader.rs
            multi_reader.rs
            read_modes.rs
          search/
            mod.rs
            grep_search.rs
            lexical.rs
            symbol_scan.rs
          overview/
            mod.rs
            project_overview.rs
            tech_stack.rs
          compression/
            mod.rs
            engine.rs
            patterns/
              git_diff.rs
              git_status.rs
              git_log.rs
              pytest.rs
              cargo_test.rs
              logs.rs
              grep.rs
              docker_ps.rs
            safety_needles.rs
          shell/
            mod.rs
            validator.rs
            executor.rs
            normalizer.rs
          memory/
            mod.rs
            session_memory.rs
            project_memory.rs
            recall.rs
            note_types.rs
          session/
            mod.rs
            state.rs
            checkpoint.rs
          stats/
            mod.rs
            counters.rs
            savings.rs
            io_budget.rs
          audit/
            mod.rs
            trail.rs
            events.rs
          limits/
            mod.rs
            budgets.rs
        storage/
          mod.rs
          sqlite/
            mod.rs
            schema.rs
            sessions.rs
            project_memory.rs
            stats.rs
            audit.rs
          files/
            mod.rs
            cache.rs
            tmp.rs
        diagnostics/
          mod.rs
          doctor.rs
          integrity.rs
          config_check.rs
        tests/
          integration/
          fixtures/
          adversarial/
          benchmarks/
```

---

# 4. Core entities

## 4.1 SessionState
Содержит текущее состояние работы:
- task
- findings
- decisions
- files_touched
- commands_run
- next_steps
- checkpoints
- stats snapshot

## 4.2 ProjectMemoryNote
Отдельная запись памяти проекта:
- id
- type
- summary
- details_compact
- source
- path_refs
- tags
- confidence
- created_at
- updated_at
- fingerprint

## 4.3 CompressionRecord
Запись по конкретной операции сжатия:
- tool_name
- operation_kind
- command_or_mode
- input_bytes
- output_bytes
- saved_bytes
- input_tokens
- output_tokens
- saved_tokens
- compression_ratio
- timestamp

## 4.4 AuditEvent
Аудит-событие:
- type
- subject
- path
- command_name
- bytes_read
- outcome
- redactions_applied
- timestamp

## 4.5 PolicySnapshot
Эффективная конфигурация ограничений:
- project_root
- allow_paths
- shell_enabled
- shell_whitelist
- max_read_bytes
- max_shell_output_bytes
- memory_enabled
- redaction_enabled

---

# 5. Storage architecture

## 5.1 Общая стратегия
Использовать гибридное хранение:
- **SQLite** для структурированной памяти, статистики, recall и audit metadata
- **файлы** для временных артефактов, кэша и snapshot outputs

## 5.2 Таблицы SQLite
Минимальные таблицы:
- `sessions`
- `session_findings`
- `session_decisions`
- `session_files`
- `project_memory_notes`
- `compression_stats`
- `io_stats`
- `audit_events`
- `config_snapshots`

## 5.3 Retention
Необходимо поддерживать:
- cleanup старых сессий
- cleanup старых audit events
- cleanup cache
- лимит общего размера локального состояния

---

# 6. Security architecture

## 6.1 PathJail
Все file operations проходят через единый `PathJail`.

Функции:
- resolve relative path
- canonicalize
- reject escape through symlink
- enforce allowed roots
- return normalized safe path

## 6.2 CommandPolicy
Все shell-команды проходят через `CommandPolicy`.

Функции:
- parse command
- normalize command
- classify intent
- whitelist match
- reject dangerous constructs
- return explicit reason on failure

## 6.3 RedactionPipeline
Перед выводом и перед записью в storage применяется pipeline:
1. detect secrets
2. redact secrets
3. mark redaction metadata
4. pass safe text дальше

## 6.4 Security invariants
Инварианты системы:
- core не делает network I/O
- память не хранит raw secrets
- shell не пишет в файлы
- file access не выходит за root jail
- все чувствительные операции отражаются в audit trail

---

# 7. Compression architecture

## 7.1 Модель движка компрессии
`CompressionEngine` принимает:
- operation type
- command
- raw output
- policy

и возвращает:
- compressed output
- safety flags
- savings stats
- audit/compression record

## 7.2 Порядок обработки
1. classify output
2. detect auth-sensitive patterns
3. choose specialized pattern handler
4. preserve safety-relevant lines
5. compute bytes/tokens saved
6. emit metrics

## 7.3 Pattern handlers
Нужны отдельные обработчики:
- `GitDiffPattern`
- `GitStatusPattern`
- `GitLogPattern`
- `PytestPattern`
- `CargoTestPattern`
- `LogsPattern`
- `GrepPattern`
- `DockerPsPattern`

## 7.4 Safety overlay
Перед финальным ответом применяется overlay:
- extract errors/warnings/critical markers
- ensure they survive truncation
- ensure auth/device code flows go to passthrough mode

---

# 8. Memory architecture

## 8.1 Session memory
Назначение:
- краткосрочная память активной работы
- нужна для resume/checkpoint/task continuity

Хранить только:
- summaries
- refs
- structured notes
- compact evidence

## 8.2 Project memory
Назначение:
- долгоживущие знания по проекту
- архитектура, соглашения, gotchas, устойчивые решения

## 8.3 Recall engine
На v1 без vector DB.
Поддержать:
- lexical scoring
- BM25/TF-IDF style ranking
- path-aware boosting
- recency boosting
- confidence boosting

## 8.4 Future compatibility
Архитектура должна позволять позже добавить:
- локальные embeddings
- vector store
- semantic retrieval
без переделки core contracts

---

# 9. Stats and savings architecture

## 9.1 Цель
Система должна уметь показывать, сколько контекста было урезано и насколько эффективно работает модуль.

## 9.2 Считать обязательно
Для каждой операции:
- input_bytes
- output_bytes
- saved_bytes
- input_tokens
- output_tokens
- saved_tokens
- compression_ratio

ДляAggregates:
- per tool
- per command
- per session
- per project
- global

## 9.3 Отображение
Через:
- `ctx-lite stats`
- `ctx-lite doctor`
- `ctx-lite audit`
- MCP tool `ctx_stats`

## 9.4 Ограничение
Статистика должна хранить только безопасные агрегаты и названия режимов/инструментов, без raw sensitive content.

---

# 10. Orchestration model

## 10.1 Общий workflow
Каждая задача проходит стадии:
1. intake
2. planning
3. discovery
4. reading/search
5. compression/packing
6. memory update
7. validation/test
8. audit/stats update

## 10.2 TaskEngine
`TaskEngine` отвечает за:
- разбор запроса
- построение suggested files
- suggested searches
- retrieval relevant memory
- выбор pack strategy

## 10.3 ContextPacker
`ContextPacker` отвечает за:
- сбор overview
- snippets
- search hits
- memory notes
- test summaries
- budget-aware ordering

## 10.4 WorkflowController
Нужен контроллер стадий, чтобы поведение было одинаковым для CLI и MCP.

---

# 11. CLI and MCP contracts

## 11.1 CLI contract
CLI — thin interface over application services.
Команды не должны содержать бизнес-логику.

## 11.2 MCP contract
MCP tools должны быть thin adapters over application services.
Вся логика должна жить в core/app слоях.

## 11.3 Common service layer
CLI и MCP обязаны использовать единые application services:
- `ReadService`
- `SearchService`
- `OverviewService`
- `ShellService`
- `MemoryService`
- `StatsService`
- `AuditService`
- `TaskService`
- `PackService`

---

# 12. Diagnostics and quality controls

## 12.1 Doctor subsystem
`doctor` проверяет:
- config integrity
- root jail correctness
- shell policy correctness
- storage paths
- local-only invariants
- sqlite health
- retention status

## 12.2 Audit subsystem
`audit` показывает:
- files read
- bytes read
- memory writes
- compression stats
- redaction events
- blocked shell commands

## 12.3 Benchmark subsystem
Нужен benchmark harness для оценки:
- compression quality
- token reduction
- search latency
- overview generation latency
- memory recall quality

---

# 13. Навигация по качественной разработке

Архитектура должна поддерживать инженерный процесс, близкий к `memory-bank/release-checklist.md` из референсного репозитория, но адаптированный под `ai-helpers`:

Нужны отдельные артефакты процесса:
- release checklist
- architecture notes
- testing checklist
- regression checklist
- adversarial scenarios list

---

# 14. Инварианты для будущих расширений

Будущие модули (например vector memory) не должны:
- ломать zero-network core
- обходить path jail
- писать secrets в память
- нарушать auditability

Если будет добавляться vector memory, она должна быть:
- локальной
- опциональной
- изолированной по feature flag

---

# 15. Итог

Архитектура `ctx-lite` должна обеспечивать:
- полезность уровня `lean-ctx` по качеству контекстной компрессии;
- более строгую и прозрачную модель безопасности;
- локальную память без cloud-рисков;
- предсказуемую оркестрацию планирования, чтения, сжатия, упаковки и тестирования;
- основу для дальнейшего роста внутри `ai-helpers`.