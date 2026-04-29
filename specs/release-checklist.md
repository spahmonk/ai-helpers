# Release Checklist для `ai-helpers`

## Статус
Draft v1

## Назначение
Этот checklist вдохновлён подходом к release quality из референсного репозитория `lean-ctx`, включая идею `memory-bank/release-checklist.md`, но адаптирован под `ai-helpers` и local-only модель.

Референс:
- https://github.com/yvgude/lean-ctx/blob/main/memory-bank/release-checklist.md

---

# 1. Specs и архитектура
- [ ] Основные спецификации актуальны
- [ ] Архитектурные документы актуальны
- [ ] Breaking changes описаны
- [ ] Non-goals не нарушены

# 2. Security
- [ ] Нет новых сетевых зависимостей
- [ ] Нет telemetry/update/cloud логики
- [ ] PathJail tests зелёные
- [ ] Shell deny tests зелёные
- [ ] Redaction tests зелёные
- [ ] Threat model инварианты не нарушены

# 3. Compression quality
- [ ] Regression tests зелёные
- [ ] Adversarial tests зелёные
- [ ] Critical lines не теряются
- [ ] Auth flows сохраняются корректно
- [ ] Compression ratio не деградировал критически

# 4. Memory
- [ ] Session memory работает
- [ ] Project memory работает
- [ ] Retention cleanup работает
- [ ] Secrets не сохраняются
- [ ] Recall не ломает privacy model

# 5. Stats / Savings / Audit
- [ ] `saved_bytes` считаются корректно
- [ ] `saved_tokens` считаются корректно
- [ ] Per-session stats корректны
- [ ] Per-tool stats корректны
- [ ] Audit trail полон
- [ ] Doctor показывает корректный local-only state

# 6. UX и diagnostics
- [ ] CLI help актуален
- [ ] Ошибки понятны
- [ ] Purge/reset работают
- [ ] Doctor понятен
- [ ] Audit понятен
- [ ] Нет скрытых background behaviors

# 7. Testing
- [ ] Unit tests зелёные
- [ ] Integration tests зелёные
- [ ] Regression tests зелёные
- [ ] Adversarial tests зелёные
- [ ] Benchmarks без критической деградации

# 8. Release readiness
- [ ] Документация обновлена
- [ ] Acceptance criteria выполнены
- [ ] Релиз можно объяснить простым языком
- [ ] Пользователь может безопасно использовать релиз в рабочем проекте

# 9. Финальное правило
Если есть сомнение между:
- более красивой автоматизацией
- и более безопасным/объяснимым поведением

выбирать более безопасное и объяснимое.