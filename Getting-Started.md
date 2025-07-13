# 🚀 Getting Started

Welcome!
This guide will help you **set up your development environment quickly and seamlessly**.

---

## 0. Install Required Tools

To run this project smoothly, please ensure you have the following installed:

### 1. **Git**

For cloning the repository:

* [Install Git](https://git-scm.com/downloads)

### 2. **Rust Toolchain**

Ensure you have:

* `rustup`
* Stable Rust

Install via:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Check installation:

```bash
rustc --version
cargo --version
```

### 3. **Postgres**

You can either:

* Install locally ([Postgres Downloads](https://www.postgresql.org/download/))
* Or run via Docker (recommended for consistency, see below).

### 4. **Docker**

If you do not have Docker installed, download it here:

* [Install Docker](https://docs.docker.com/get-docker/)

Verify installation:

```bash
docker --version
```

### 5. **sqlx CLI**

Used for migrations and interacting with the database.

Install:

```bash
cargo install sqlx-cli --no-default-features --features postgres
```

Verify installation:

```bash
sqlx --version
```

---

## Next Steps

### Start Postgres Database

If you **do not have Postgres installed locally**, you can run it with Docker:

```bash
docker run -d --rm \
  --name zerox \
  -p 5434:5432 \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=zeroxdb \
  postgres
```

✅ This will start a Postgres container on `localhost:5434`, with:

* Username: `postgres`
* Password: `postgres`
* Database: `zeroxdb`

If you already have Postgres locally, ensure:

* It is **running**.
* You have a database named `zeroxdb`.

---

### 2. Set up your `.env` file

Create a `.env` file at the project root with the following content:

```env
DATABASE_URL=postgresql://postgres:postgres@localhost:5434/zeroxdb
```

This allows your application and `sqlx` tooling to connect to your database.

---

### 3. Expose the database for `sqlx`

Before running `sqlx` commands, ensure your terminal session has access to the `DATABASE_URL`.

Either load your `.env`:

```bash
source .env
```

Or export it directly:

```bash
export DATABASE_URL=postgresql://postgres:postgres@localhost:5434/zeroxdb
```

---

### 4. Run Database Migrations

To set up your database schema, run:

```bash
sqlx migrate run
```

Check the status of migrations:

```bash
sqlx migrate info
```

---

### 5. Common `sqlx` Commands

| Command                   | Description                  |
| :------------------------ | :--------------------------- |
| `sqlx migrate run`        | Apply all pending migrations |
| `sqlx migrate revert`     | Revert the last migration    |
| `sqlx migrate add <name>` | Create a new migration       |

---

### 6. Stopping the Postgres Container

The Docker container started with `--rm` will auto-remove when stopped.

To stop it manually:

```bash
docker stop zerox
```

---

## Development Steps

Once your environment is set up:

* Run your application (`cargo run`) to verify the database connection and migrations.
* Start developing your feature.
