# YieldVault — Solana Devnet DeFi MVP

Учебный **yield / staking vault** на Solana Devnet: депозит SPL-токена, учёт долей (share mint), начисление APY по времени, вывод с комиссией. Mainnet и реальные средства не используются.

## Выбранная механика

**Yield / staking vault**

| Операция | Описание |
|---|---|
| `initialize` | Создание Vault PDA, share mint, vault token account |
| `deposit` | Ввод underlying → mint shares |
| `withdraw` | Burn shares → вывод underlying минус fee (с `min_out`) |
| `fund_rewards` | Authority пополняет ликвидность под начисленный yield |
| `set_paused` | Пауза депозитов |

## Экономическая модель

- **APY** задаётся в basis points при `initialize` (демо: 10% = 1000 bps).
- При каждом `deposit` / `withdraw` / `fund_rewards` вызывается `accrue_yield`:  
  `interest = total_assets * apy_bps * Δt / (10_000 * seconds_per_year)`.  
  Если interest округляется до 0, `last_update_ts` **не** сдвигается (время накапливается).
- **Exchange rate**: `assets_per_share = total_assets / total_shares` (первый депозит 1:1).
- **Withdrawal fee** (демо: 0.5% = 50 bps) уходит на ATA `fee_recipient` (обычно authority).
- **min_out** на `withdraw`: net после fee должен быть ≥ `min_out`, иначе `SlippageExceeded`.
- Если `fee_recipient == user`, `fee_recipient_ata` передаётся как `None` (иначе Solana отклонит duplicate mutable account); net+fee уходят одним transfer на ATA пользователя.
- Redeemable yield ограничен балансом `vault_token`: виртуальное начисление без `fund_rewards` не позволит вывести больше фактической ликвидности (`InsufficientLiquidity`).

## Архитектура и PDA

**Program ID:** `9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu`

| Аккаунт | Seeds | Назначение |
|---|---|---|
| `Vault` | `["vault", underlying_mint]` | Конфиг + accounting |
| `share_mint` | `["share_mint", vault]` | SPL mint долей (authority = Vault PDA) |
| `vault_token` | `["vault_token", vault]` | Хранилище underlying (authority = Vault PDA) |

`fee_recipient` хранится в `Vault` (по умолчанию = authority); fee на withdraw уходит на его ATA (не в PDA).

Проверки: signer, `has_one` mint/token, owner ATA, min deposit, max TVL, pause, `min_out` на withdraw, checked arithmetic (`checked_*` / `u128`).

```
User wallet ──deposit──► vault_token (PDA)
             ◄─shares─── share_mint (PDA mint)
User wallet ◄─withdraw── vault_token (− fee → fee_recipient ATA)
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

Тесты: ≥5 unit + ≥5 integration LiteSVM (happy-path deposit/withdraw/fund_rewards, authority checks, min_out, негативные missing accounts).

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

**Program ID:** `HCM2iWcvchzhsnZnwbLdxBHWvgn4hkEyZ9CVGdgRkjPg`  
**Program Explorer:** https://explorer.solana.com/address/HCM2iWcvchzhsnZnwbLdxBHWvgn4hkEyZ9CVGdgRkjPg?cluster=devnet

| Операция | Explorer |
|---|---|
| Initialize | https://explorer.solana.com/tx/2dQ7ntbsQMH3qXjVNqhYGbohesB9BHCnrZjycfH633qATwLF8Su7MfwXEeLtiZX7f8LKkkVqEz2m8xXfdp7STHHE?cluster=devnet |
| Deposit (+ fund rewards) | https://explorer.solana.com/tx/3aE7nDLXkbvQoZx4LEWtjW25UvhvqssAxin8PaseBJNgg4x9ZwMkrMTTkc8gndk5n845bG51m6qqHgKJyvdzJuyA?cluster=devnet |
| Withdraw | https://explorer.solana.com/tx/42RUBtajDNat2qAWh468ytQLJiZmNqrd4HfHLuNvdBahrujZkjo29QEwtnfCCTsDLv7U1H2ETp9uvj9MvddxY46z?cluster=devnet |

**Underlying mint:** `GYLPoYJvP24nGTHJ9c2Naq2nRiyX5kL26yZ8Rh72VXUN`  
**Vault PDA:** `5NBgkwgAfAXMSqNKaHmFSKfA5mYeDXXYFF3LnCwp5z4b`

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
- Rate limiting на withdraw
- Token-2022 transfer fee / permanent delegate edge cases
- Upgrade authority freeze / immutable program после аудита

## Лицензия

Учебный проект. Apache-2.0 для кода программы по умолчанию Anchor.
