# Rust Distributed Task Scheduler

> 🇯🇵 **Note:** [日本語のガイドはこちら (Japanese Guide)](#-日本語ガイド-japanese-guide)

A robust, concurrent, and persistent task scheduler built with **Rust**, **Axum**, **SQLx (SQLite)**, and **Tokio**. This system runs as a standalone service that triggers HTTP webhooks when tasks are due.

## 🏗 Architecture

```text
[Client] -> HTTP POST /tasks -> [Axum API]
                                    |
                               (Writes to DB)
                                    v
                               [SQLite DB] <== (Polls/Updates) == [Scheduler Loop]
                                    ^                                    |
                                    |                             (Executes Task)
                                    |                                    |
                               (Reads Data) <----(Wake Signal)----[Channel]
```

## ✨ Features

* **Dynamic Scheduling:** Support for One-off (run once) and Interval (recurring) tasks.
* **Resilience:** Atomic transactions, soft deletes, and graceful shutdowns.
* **Observability:** Structured JSON logging (Production) and Pretty logging (Dev).
* **Persistence:** SQLite with WAL mode enabled for high concurrency.
* **Dockerized:** Production-ready multi-stage Docker setup.

---

## 🚀 Quick Start

### Option A: Docker (Recommended)

This handles database creation, permissions, and networking automatically.

1.  **Start the service:**
    ```bash
    docker-compose up --build
    ```

2.  **Access the Dashboard:**
    Open [http://localhost:8080](http://localhost:8080) in your browser.

### Option B: Local Development

1.  **Install Prerequisites:**
    ```bash
    cargo install sqlx-cli
    ```

2.  **Initialize Database:**
    ```bash
    mkdir data
    export DATABASE_URL="sqlite:data/tasks.db"
    sqlx database create
    sqlx migrate run
    ```

3.  **Run Application:**
    ```bash
    cargo run
    ```

4.  **Run Tests:**
    ```bash
    cargo test
    ```

---

## 📡 API Reference

### 1. Create a One-Time Task
Schedules a webhook to fire at a specific ISO-8601 time.

```bash
curl -i -X POST http://localhost:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "One-off Reminder",
    "task_type": "once",
    "trigger_at": "2025-12-31T23:59:59Z",
    "payload": {
      "url": "[https://httpbin.org/post](https://httpbin.org/post)",
      "method": "POST",
      "body": { "msg": "Scheduled Event" }
    }
  }'
```

### 2. Create an Interval Task
Fires repeatedly (e.g., every 10 seconds).

```bash
curl -i -X POST http://localhost:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Health Check",
    "task_type": "interval",
    "trigger_at": "2024-01-01T00:00:00Z",
    "interval_seconds": 10,
    "payload": {
      "url": "[https://httpbin.org/get](https://httpbin.org/get)",
      "method": "GET"
    }
  }'
```

### 3. List All Tasks
Returns a summary of all active and soft-deleted tasks.

```bash
curl http://localhost:8080/tasks
```

### 4. Delete a Task
Performs a soft delete. The scheduler stops processing it, but history is preserved.

```bash
curl -X DELETE http://localhost:8080/tasks/<TASK_ID>
```

---
---

# 🇯🇵 日本語ガイド (Japanese Guide)

**Rust**, **Axum**, **SQLx (SQLite)**, **Tokio** を用いて構築された、堅牢で並行処理可能な永続的タスクスケジューラです。このシステムはスタンドアロンサービスとして動作し、タスクの実行時期が来るとHTTP Webhookをトリガーします。

## 🏗 アーキテクチャ

```text
[クライアント] -> HTTP POST /tasks -> [Axum API]
                                        |
                                   (DB書き込み)
                                        v
                                   [SQLite DB] <== (ポーリング/更新) == [スケジューラループ]
                                        ^                                    |
                                        |                              (タスク実行)
                                        |                                    |
                                   (データ読込) <----(起動シグナル)----[チャンネル]
```

## ✨ 主な機能

* **動的スケジューリング:** 1回限りの実行（ワンオフ）と、繰り返し実行（インターバル）をサポートします。
* **耐障害性 (Resilience):** アトミックなトランザクション管理、履歴を残すソフトデリート、および安全なシャットダウン機能を備えています。
* **可観測性 (Observability):** 本番環境向けの構造化JSONログと、開発環境向けの可読性の高いログを切り替え可能です。
* **永続性:** 高い並行性能を実現するため、WALモードを有効にしたSQLiteを使用しています。
* **Docker対応:** パーミッション管理を自動化した、本番運用可能なマルチステージDocker環境を含みます。

---

## 🚀 クイックスタート

### オプション A: Docker (推奨)

データベースの作成、権限設定、ネットワーク設定を自動的に処理します。

1.  **サービスを起動:**
    ```bash
    docker-compose up --build
    ```

2.  **ダッシュボードにアクセス:**
    ブラウザで [http://localhost:8080](http://localhost:8080) を開いてください。

### オプション B: ローカル開発

1.  **前提ツールのインストール:**
    ```bash
    cargo install sqlx-cli
    ```

2.  **データベースの初期化:**
    ```bash
    mkdir data
    export DATABASE_URL="sqlite:data/tasks.db"
    sqlx database create
    sqlx migrate run
    ```

3.  **アプリケーションの実行:**
    開発モード（整形されたログ）で起動します。
    ```bash
    cargo run
    ```

4.  **テストの実行:**
    ```bash
    cargo test
    ```

---

## 📡 API リファレンス

### 1. ワンタイムタスクの作成
指定した ISO-8601 形式の日時に Webhook をトリガーします。

```bash
curl -i -X POST http://localhost:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "One-off Reminder",
    "task_type": "once",
    "trigger_at": "2025-12-31T23:59:59Z",
    "payload": {
      "url": "[https://httpbin.org/post](https://httpbin.org/post)",
      "method": "POST",
      "body": { "msg": "Scheduled Event" }
    }
  }'
```

### 2. インターバルタスクの作成
繰り返し実行されるタスクを作成します（例：10秒ごと）。

```bash
curl -i -X POST http://localhost:8080/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Health Check",
    "task_type": "interval",
    "trigger_at": "2024-01-01T00:00:00Z",
    "interval_seconds": 10,
    "payload": {
      "url": "[https://httpbin.org/get](https://httpbin.org/get)",
      "method": "GET"
    }
  }'
```

### 3. 全タスクのリスト表示
すべてのアクティブなタスクとソフトデリートされたタスクの概要を取得します。

```bash
curl http://localhost:8080/tasks
```

### 4. タスクの削除
タスクをソフトデリート（論理削除）します。スケジューラによる処理は停止しますが、実行履歴はデータベースに残ります。

```bash
curl -X DELETE http://localhost:8080/tasks/<TASK_ID>
```
