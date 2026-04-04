use sqlx::PgPool;

/// Create all Orni Models tables in the 'orni' schema, idempotently.
/// The connection uses search_path=orni,public so all queries work
/// against the orni schema without SQL changes.
pub async fn ensure_schema(db: &PgPool) -> anyhow::Result<()> {
    // Create schema
    sqlx::query("CREATE SCHEMA IF NOT EXISTS orni").execute(db).await?;

    // Set search_path for this session (all subsequent queries use orni schema first)
    sqlx::query("SET search_path TO orni, public").execute(db).await?;

    // Enums (in orni schema) — each must be a separate query
    sqlx::query("DO $$ BEGIN CREATE TYPE orni.model_status AS ENUM ('draft', 'training', 'live', 'paused', 'failed'); EXCEPTION WHEN duplicate_object THEN NULL; END $$").execute(db).await?;
    sqlx::query("DO $$ BEGIN CREATE TYPE orni.source_type AS ENUM ('text', 'pdf', 'youtube', 'blog'); EXCEPTION WHEN duplicate_object THEN NULL; END $$").execute(db).await?;
    sqlx::query("DO $$ BEGIN CREATE TYPE orni.content_status AS ENUM ('pending', 'processing', 'ready', 'failed'); EXCEPTION WHEN duplicate_object THEN NULL; END $$").execute(db).await?;
    sqlx::query("DO $$ BEGIN CREATE TYPE orni.fine_tune_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled'); EXCEPTION WHEN duplicate_object THEN NULL; END $$").execute(db).await?;
    sqlx::query("DO $$ BEGIN CREATE TYPE orni.chat_role AS ENUM ('system', 'user', 'assistant'); EXCEPTION WHEN duplicate_object THEN NULL; END $$").execute(db).await?;

    // Tables (in orni schema, using original names so queries work unchanged)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS orni.users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            wallet_address VARCHAR(64) UNIQUE,
            email VARCHAR(255) UNIQUE,
            password_hash VARCHAR(255),
            username VARCHAR(64) UNIQUE,
            display_name VARCHAR(128),
            avatar_url TEXT,
            is_creator BOOLEAN NOT NULL DEFAULT FALSE,
            usdc_balance BIGINT NOT NULL DEFAULT 0,
            stripe_customer_id VARCHAR(255),
            slug VARCHAR(64) UNIQUE,
            did TEXT UNIQUE,
            said_verified BOOLEAN NOT NULL DEFAULT FALSE,
            said_profile_url TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#).execute(db).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS orni.models (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            creator_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
            slug VARCHAR(128) NOT NULL UNIQUE,
            name VARCHAR(256) NOT NULL,
            description TEXT,
            avatar_url TEXT,
            system_prompt TEXT NOT NULL,
            base_model VARCHAR(256) NOT NULL DEFAULT 'meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo',
            provider_model_id VARCHAR(256),
            status orni.model_status NOT NULL DEFAULT 'draft',
            price_per_query BIGINT NOT NULL DEFAULT 100000,
            total_queries BIGINT NOT NULL DEFAULT 0,
            total_revenue BIGINT NOT NULL DEFAULT 0,
            category VARCHAR(128),
            tags TEXT[] NOT NULL DEFAULT '{}',
            self_hosted_node_id UUID,
            self_hosted_endpoint TEXT,
            is_featured BOOLEAN NOT NULL DEFAULT FALSE,
            is_platform_model BOOLEAN NOT NULL DEFAULT FALSE,
            free_queries_per_day INT NOT NULL DEFAULT 0,
            avg_rating DOUBLE PRECISION NOT NULL DEFAULT 0,
            review_count INT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
    "#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.content_sources (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        source_type orni.source_type NOT NULL, source_url TEXT, content_text TEXT,
        status orni.content_status NOT NULL DEFAULT 'pending',
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.training_datasets (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        file_key TEXT NOT NULL, num_examples INTEGER NOT NULL DEFAULT 0,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.fine_tune_jobs (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        provider_job_id VARCHAR(256), status orni.fine_tune_status NOT NULL DEFAULT 'pending',
        result_model_id VARCHAR(256), error_message TEXT,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.chat_sessions (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.chat_messages (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        session_id UUID NOT NULL REFERENCES orni.chat_sessions(id) ON DELETE CASCADE,
        role orni.chat_role NOT NULL, content TEXT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.payments (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        amount BIGINT NOT NULL, creator_share BIGINT NOT NULL, platform_share BIGINT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.deposits (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        amount BIGINT NOT NULL, tx_signature VARCHAR(128) NOT NULL UNIQUE,
        verified BOOLEAN NOT NULL DEFAULT FALSE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.credit_purchases (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        amount_micro_usdc BIGINT NOT NULL, amount_usd_cents INT NOT NULL,
        stripe_session_id VARCHAR(255) UNIQUE, status TEXT NOT NULL DEFAULT 'pending',
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.free_query_usage (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        query_date DATE NOT NULL DEFAULT CURRENT_DATE, query_count INT NOT NULL DEFAULT 1,
        UNIQUE(user_id, model_id, query_date)
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.api_keys (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        key_hash VARCHAR(64) NOT NULL UNIQUE, key_prefix VARCHAR(12) NOT NULL,
        name VARCHAR(128), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_used_at TIMESTAMPTZ, is_active BOOLEAN NOT NULL DEFAULT TRUE
    )"#).execute(db).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS orni.model_reviews (
        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
        user_id UUID NOT NULL REFERENCES orni.users(id) ON DELETE CASCADE,
        model_id UUID NOT NULL REFERENCES orni.models(id) ON DELETE CASCADE,
        rating INT NOT NULL CHECK (rating >= 1 AND rating <= 5),
        review_text TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        UNIQUE(user_id, model_id)
    )"#).execute(db).await?;

    // Seed platform user + models
    sqlx::query(r#"
        INSERT INTO orni.users (id, wallet_address, display_name, is_creator)
        VALUES ('00000000-0000-0000-0000-000000000001', NULL, 'Orni Platform', TRUE)
        ON CONFLICT DO NOTHING
    "#).execute(db).await.ok();

    sqlx::query(r#"
        INSERT INTO orni.models (id, creator_id, slug, name, description, system_prompt, base_model, provider_model_id, status, price_per_query, category, is_featured, is_platform_model, free_queries_per_day)
        VALUES
            ('00000000-0000-0000-0000-000000000010', '00000000-0000-0000-0000-000000000001', 'llama-3-8b', 'Llama 3.1 8B', 'Meta''s fast and capable open-source model.', 'You are a helpful AI assistant.', 'llama-3.1-8b-instant', 'llama-3.1-8b-instant', 'live', 50000, 'Technology', TRUE, TRUE, 20),
            ('00000000-0000-0000-0000-000000000011', '00000000-0000-0000-0000-000000000001', 'qwen-32b', 'Qwen3 32B', 'Alibaba''s powerful reasoning model.', 'You are a helpful AI assistant.', 'qwen/qwen3-32b', 'qwen/qwen3-32b', 'live', 50000, 'Technology', TRUE, TRUE, 20),
            ('00000000-0000-0000-0000-000000000012', '00000000-0000-0000-0000-000000000001', 'llama-3-70b', 'Llama 3.3 70B', 'Meta''s most capable open model.', 'You are a helpful AI assistant.', 'llama-3.3-70b-versatile', 'llama-3.3-70b-versatile', 'live', 200000, 'Technology', TRUE, TRUE, 20),
            ('00000000-0000-0000-0000-000000000013', '00000000-0000-0000-0000-000000000001', 'llama-scout-17b', 'Llama 4 Scout 17B', 'Meta''s latest Llama 4 model.', 'You are a coding and reasoning assistant.', 'meta-llama/llama-4-scout-17b-16e-instruct', 'meta-llama/llama-4-scout-17b-16e-instruct', 'live', 100000, 'Technology', TRUE, TRUE, 5)
        ON CONFLICT DO NOTHING
    "#).execute(db).await.ok();

    // Update free tier on existing platform models (seed uses ON CONFLICT DO NOTHING
    // so this handles models created before the free tier bump)
    sqlx::query("UPDATE orni.models SET free_queries_per_day = 20 WHERE is_platform_model = true AND free_queries_per_day < 20")
        .execute(db).await.ok();

    Ok(())
}
