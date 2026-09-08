# YieldVault — Solana Devnet DeFi MVP

Учебный **yield / staking vault** на Solana Devnet: депозит SPL-токена, учёт долей (share mint), начисление APY по времени, вывод с комиссией. Mainnet и реальные средства не используются.

## Выбранная механика

**Yield / staking vault**

| Операция | Описание |
|---|---|
| `initialize` | Создание Vault PDA, share mint, vault token account |
| `deposit` | Ввод underlying → mint shares |
| `withdraw` | Burn shares → вывод underlying минус fee |
| `fund_rewards` | Authority пополняет ликвидность под начисленный yield |
| `set_paused` | Пауза депозитов |

## Экономическая модель

- **APY** задаётся в basis points при `initialize` (демо: 10% = 1000 bps).
- При каждом `deposit` / `withdraw` / `fund_rewards` вызывается `accrue_yield`:  
  `interest = total_assets * apy_bps * Δt / (10_000 * seconds_per_year)`.
- **Exchange rate**: `assets_per_share = total_assets / total_shares` (первый депозит 1:1).
- **Withdrawal fee** (демо: 0.5% = 50 bps) уходит на ATA `fee_recipient` (обычно authority).
- Redeemable yield ограничен балансом `vault_token`: виртуальное начисление без `fund_rewards` не позволит вывести больше фактической ликвидности (`InsufficientLiquidity`).

## Архитектура и PDA

**Program ID:** `9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu`

| Аккаунт | Seeds | Назначение |
|---|---|---|
| `Vault` | `["vault", underlying_mint]` | Конфиг + accounting |
| `share_mint` | `["share_mint", vault]` | SPL mint долей (authority = Vault PDA) |
| `vault_token` | `["vault_token", vault]` | Хранилище underlying (authority = Vault PDA) |
| `fee_token` | `["fee_token", vault]` | Accumulated withdrawal fees |

Проверки: signer, `has_one` mint/token, owner ATA, min deposit, max TVL, pause, checked arithmetic (`checked_*` / `u128`).

```
User wallet ──deposit──► vault_token (PDA)
             ◄─shares─── share_mint (PDA mint)
User wallet ◄─withdraw── vault_token (− fee → authority ATA)
Authority  ──fund──────► vault_token
```

## Безопасность и риски

**Сделано**

- Нет приватных ключей в git (`.keys/`, `*-keypair.json` в `.gitignore`).
- CPI только через `transfer_checked` / `mint_to` / `burn`.
- Лимиты APY ≤ 100%, fee ≤ 10%, TVL / min deposit.
- Ошибки откатывают транзакцию целиком (атомарность Solana).

**Риски / ограничения MVP (до mainnet нужно исправить)**

1. Yield **синтетический** — не обеспечен внешним доходом; нужен `fund_rewards`.
2. Нет оракула / нет ликвидации (это не lending).
3. Authority — единая точка управления (pause / fund); для prod — multisig / timelock.
4. Share decimals фиксированы (6), underlying может отличаться.
5. Нет reentrancy-специфики EVM, но важны корректные PDA seeds и ownership (проверены).
6. Нет on-chain emergency withdraw кроме pause + authority liquidity path.

## Быстрый старт

### Требования

- Rust 1.89+, Solana CLI 2.x/3.x, Anchor CLI 1.1+, Node 20+
- Devnet SOL на deployer-кошельке

### 1. Сборка и тесты

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"
export ANCHOR_WALLET=.keys/deployer.json   # или ваш keypair (не коммитить)

# создать deployer, если нужно
mkdir -p .keys
solana-keygen new -o .keys/deployer.json --no-bip39-passphrase
solana airdrop 2 --url https://api.devnet.solana.com --keypair .keys/deployer.json

anchor build
anchor test
```

Тесты: ≥5 unit + ≥5 integration LiteSVM (включая негативные: zero deposit, missing accounts, invalid APY path).

### 2. Деплой на Devnet

```bash
anchor deploy --provider.cluster devnet
anchor idl build -p yield_vault -o target/idl/yield_vault.json
```

### 3. Инициализация vault + демо-транзакции

```bash
npm install
ANCHOR_WALLET=.keys/deployer.json node scripts/setup-devnet.mjs
```

Скрипт создаёт SPL mint, вызывает `initialize`, `fund_rewards`, `deposit`, `withdraw`, пишет:

- `demo-artifacts.json` — Program ID, PDA, Explorer-ссылки
- `web/src/lib/config.ts` — адреса для UI

### 4. Веб-интерфейс

```bash
cd web && npm install && npm run dev
```

Откройте http://localhost:3000, подключите Phantom (**Devnet**), при необходимости вставьте mint из `demo-artifacts.json`.

Статусы: pending → success (ссылка Explorer) / error (текст ошибки).

### Деплой на Vercel

В настройках проекта Vercel обязательно укажите:

- **Root Directory:** `web` (Settings → General → Root Directory)
- Framework Preset: Next.js
- Env: **не нужны**

Без `Root Directory = web` деплой идёт из корня репозитория и отдаёт **404**.

## Program ID и транзакции

**Program ID:** `9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu`  
**Program Explorer:** https://explorer.solana.com/address/9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu?cluster=devnet

| Операция | Explorer |
|---|---|
| Initialize | https://explorer.solana.com/tx/4NMqvX6fkjorw2Waty8muuMXfPm62nyJw51j3XJcbPQUwQZEAjm2XsKbYQEZrkx9Cd2FJL7uQRT6yS5XuXHiRnqB?cluster=devnet |
| Deposit (+ fund rewards) | https://explorer.solana.com/tx/WSiPDZ4XHjAKHsJueX1AErxUiLYeajGR3rx8NWFvbHoVCEYBLWvXpuRgLneASbkTmGjr9Nyy9GnEJWTRE5FpVYT?cluster=devnet |
| Withdraw | https://explorer.solana.com/tx/59arLkie4JHYv8521zVUtS3PY9X4zWnWwQeXNcsWrmfUR32Y7LZwQK4uKAbKJmm81UjCC7MXjW7F7fhLJSA7mM3e?cluster=devnet |

**Underlying mint:** `AX9xgr6HoZG75bdNBjzuRatEUeVR3qXpg82ztzaT5sVL`  
**Vault PDA:** `ExtwCDFeuESr6DFXAMp3kHodBtRqMunJMi7iwo7LpNP2`

Полный набор адресов: `demo-artifacts.json`.

**Демо UI:** `cd web && npm run dev` → http://localhost:3000 (mint уже прописан в `web/src/lib/config.ts`)  
**Видео:** запишите 3–5 мин (кошелёк → deposit → withdraw → Explorer).

## Структура репозитория

```
programs/yield_vault/   # on-chain Anchor program + Rust tests
web/                    # Next.js + wallet-adapter UI
scripts/setup-devnet.mjs
Anchor.toml
README.md
```

## Что улучшить перед mainnet

- Реальный источник yield (lending / LP fees), а не synthetic APY
- Timelock + multisig на authority
- Formal verification / audit, fuzz (Anchor fuzz), invariant tests
- Slippage / min-out на withdraw, rate limiting
- Token-2022 transfer fee / permanent delegate edge cases
- Upgrade authority freeze / immutable program после аудита

## Лицензия

Учебный проект. Apache-2.0 для кода программы по умолчанию Anchor.
