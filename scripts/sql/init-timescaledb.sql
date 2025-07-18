-- HorizonOS AI Integration - TimescaleDB Initialization Script
-- This script sets up the database schema for AI pattern storage and analysis

-- Create the TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create the main database if it doesn't exist
-- (This is handled by the POSTGRES_DB environment variable)

-- Create additional databases for different services
CREATE DATABASE IF NOT EXISTS n8n;
CREATE DATABASE IF NOT EXISTS temporal;
CREATE DATABASE IF NOT EXISTS ai_patterns;

-- Switch to the ai_patterns database for main schema
\c ai_patterns

-- Create the TimescaleDB extension in the ai_patterns database
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- User actions table for continuous behavioral learning
CREATE TABLE IF NOT EXISTS user_actions (
    time TIMESTAMPTZ NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    action_type TEXT NOT NULL,
    target TEXT NOT NULL,
    context JSONB,
    duration_ms INTEGER,
    session_id TEXT,
    application TEXT,
    window_title TEXT,
    file_path TEXT,
    url TEXT,
    screen_coordinates POINT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Convert to hypertable for time-series optimization
SELECT create_hypertable('user_actions', 'time', if_not_exists => TRUE);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_user_actions_type_time ON user_actions (action_type, time DESC);
CREATE INDEX IF NOT EXISTS idx_user_actions_target_time ON user_actions (target, time DESC);
CREATE INDEX IF NOT EXISTS idx_user_actions_user_time ON user_actions (user_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_user_actions_session ON user_actions (session_id);
CREATE INDEX IF NOT EXISTS idx_user_actions_app ON user_actions (application);

-- Continuous aggregate for 15-minute pattern analysis
CREATE MATERIALIZED VIEW IF NOT EXISTS action_patterns_15min
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('15 minutes', time) AS bucket,
    user_id,
    action_type,
    target,
    application,
    COUNT(*) as occurrence_count,
    AVG(duration_ms) as avg_duration_ms,
    MIN(time) as first_occurrence,
    MAX(time) as last_occurrence
FROM user_actions
GROUP BY bucket, user_id, action_type, target, application
WITH NO DATA;

-- Continuous aggregate for hourly patterns
CREATE MATERIALIZED VIEW IF NOT EXISTS action_patterns_hourly
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('1 hour', time) AS bucket,
    user_id,
    action_type,
    target,
    application,
    COUNT(*) as occurrence_count,
    AVG(duration_ms) as avg_duration_ms,
    EXTRACT(hour FROM time) as hour_of_day,
    EXTRACT(dow FROM time) as day_of_week
FROM user_actions
GROUP BY bucket, user_id, action_type, target, application, hour_of_day, day_of_week
WITH NO DATA;

-- Continuous aggregate for daily patterns
CREATE MATERIALIZED VIEW IF NOT EXISTS action_patterns_daily
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('1 day', time) AS bucket,
    user_id,
    action_type,
    target,
    application,
    COUNT(*) as occurrence_count,
    AVG(duration_ms) as avg_duration_ms,
    EXTRACT(dow FROM time) as day_of_week
FROM user_actions
GROUP BY bucket, user_id, action_type, target, application, day_of_week
WITH NO DATA;

-- Learned patterns table for storing detected patterns
CREATE TABLE IF NOT EXISTS learned_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL DEFAULT 'default',
    pattern_type TEXT NOT NULL,
    pattern_name TEXT NOT NULL,
    pattern_data JSONB NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    first_seen TIMESTAMPTZ NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    user_feedback JSONB DEFAULT '{}',
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for learned patterns
CREATE INDEX IF NOT EXISTS idx_learned_patterns_user ON learned_patterns (user_id);
CREATE INDEX IF NOT EXISTS idx_learned_patterns_type ON learned_patterns (pattern_type);
CREATE INDEX IF NOT EXISTS idx_learned_patterns_confidence ON learned_patterns (confidence DESC);
CREATE INDEX IF NOT EXISTS idx_learned_patterns_enabled ON learned_patterns (enabled);
CREATE INDEX IF NOT EXISTS idx_learned_patterns_last_seen ON learned_patterns (last_seen DESC);

-- AI suggestions table
CREATE TABLE IF NOT EXISTS ai_suggestions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL DEFAULT 'default',
    suggestion_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    action_data JSONB NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    pattern_id UUID REFERENCES learned_patterns(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    shown_at TIMESTAMPTZ,
    user_response TEXT, -- 'accepted', 'rejected', 'dismissed'
    response_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

-- Indexes for suggestions
CREATE INDEX IF NOT EXISTS idx_suggestions_user ON ai_suggestions (user_id);
CREATE INDEX IF NOT EXISTS idx_suggestions_type ON ai_suggestions (suggestion_type);
CREATE INDEX IF NOT EXISTS idx_suggestions_created ON ai_suggestions (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_suggestions_expires ON ai_suggestions (expires_at);
CREATE INDEX IF NOT EXISTS idx_suggestions_pattern ON ai_suggestions (pattern_id);

-- Workflows table for storing automation workflows
CREATE TABLE IF NOT EXISTS ai_workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id TEXT NOT NULL DEFAULT 'default',
    name TEXT NOT NULL,
    description TEXT,
    workflow_type TEXT NOT NULL, -- 'n8n', 'temporal', 'prefect', 'custom'
    workflow_definition JSONB NOT NULL,
    created_from TEXT, -- 'manual', 'teaching', 'suggestion'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    modified_at TIMESTAMPTZ DEFAULT NOW(),
    last_executed TIMESTAMPTZ,
    execution_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    enabled BOOLEAN DEFAULT TRUE,
    schedule_expression TEXT, -- cron expression
    tags TEXT[] DEFAULT '{}'
);

-- Indexes for workflows
CREATE INDEX IF NOT EXISTS idx_workflows_user ON ai_workflows (user_id);
CREATE INDEX IF NOT EXISTS idx_workflows_type ON ai_workflows (workflow_type);
CREATE INDEX IF NOT EXISTS idx_workflows_enabled ON ai_workflows (enabled);
CREATE INDEX IF NOT EXISTS idx_workflows_tags ON ai_workflows USING GIN (tags);
CREATE INDEX IF NOT EXISTS idx_workflows_last_executed ON ai_workflows (last_executed DESC);

-- Workflow executions table
CREATE TABLE IF NOT EXISTS workflow_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID REFERENCES ai_workflows(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL DEFAULT 'default',
    execution_id TEXT NOT NULL, -- external execution ID from n8n/temporal/etc
    status TEXT NOT NULL, -- 'running', 'completed', 'failed', 'cancelled'
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    execution_data JSONB,
    logs JSONB DEFAULT '[]'
);

-- Convert workflow executions to hypertable
SELECT create_hypertable('workflow_executions', 'started_at', if_not_exists => TRUE);

-- Indexes for workflow executions
CREATE INDEX IF NOT EXISTS idx_executions_workflow ON workflow_executions (workflow_id);
CREATE INDEX IF NOT EXISTS idx_executions_user ON workflow_executions (user_id);
CREATE INDEX IF NOT EXISTS idx_executions_status ON workflow_executions (status);
CREATE INDEX IF NOT EXISTS idx_executions_started ON workflow_executions (started_at DESC);

-- AI model usage tracking
CREATE TABLE IF NOT EXISTS ai_model_usage (
    time TIMESTAMPTZ NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    model_name TEXT NOT NULL,
    service_name TEXT NOT NULL,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    inference_time_ms INTEGER,
    request_id TEXT,
    session_id TEXT,
    context JSONB
);

-- Convert to hypertable
SELECT create_hypertable('ai_model_usage', 'time', if_not_exists => TRUE);

-- Indexes for model usage
CREATE INDEX IF NOT EXISTS idx_model_usage_user_time ON ai_model_usage (user_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_model_usage_model ON ai_model_usage (model_name);
CREATE INDEX IF NOT EXISTS idx_model_usage_service ON ai_model_usage (service_name);

-- User preferences and settings
CREATE TABLE IF NOT EXISTS user_ai_preferences (
    user_id TEXT PRIMARY KEY,
    preferences JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- System metrics table
CREATE TABLE IF NOT EXISTS system_metrics (
    time TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    tags JSONB DEFAULT '{}',
    host TEXT DEFAULT 'localhost'
);

-- Convert to hypertable
SELECT create_hypertable('system_metrics', 'time', if_not_exists => TRUE);

-- Indexes for system metrics
CREATE INDEX IF NOT EXISTS idx_system_metrics_name_time ON system_metrics (metric_name, time DESC);
CREATE INDEX IF NOT EXISTS idx_system_metrics_host ON system_metrics (host);

-- Set up retention policies (keep detailed data for 30 days, aggregated for 1 year)
SELECT add_retention_policy('user_actions', INTERVAL '30 days');
SELECT add_retention_policy('workflow_executions', INTERVAL '90 days');
SELECT add_retention_policy('ai_model_usage', INTERVAL '30 days');
SELECT add_retention_policy('system_metrics', INTERVAL '7 days');

-- Create refresh policies for continuous aggregates
SELECT add_continuous_aggregate_policy('action_patterns_15min',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '15 minutes',
    schedule_interval => INTERVAL '15 minutes');

SELECT add_continuous_aggregate_policy('action_patterns_hourly',
    start_offset => INTERVAL '4 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

SELECT add_continuous_aggregate_policy('action_patterns_daily',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- Create compression policies for efficient storage
SELECT add_compression_policy('user_actions', INTERVAL '7 days');
SELECT add_compression_policy('workflow_executions', INTERVAL '7 days');
SELECT add_compression_policy('ai_model_usage', INTERVAL '7 days');
SELECT add_compression_policy('system_metrics', INTERVAL '1 day');

-- Create functions for common queries
CREATE OR REPLACE FUNCTION get_user_patterns(
    p_user_id TEXT DEFAULT 'default',
    p_pattern_type TEXT DEFAULT NULL,
    p_limit INTEGER DEFAULT 100
) RETURNS TABLE(
    pattern_id UUID,
    pattern_type TEXT,
    pattern_name TEXT,
    confidence REAL,
    occurrence_count INTEGER,
    last_seen TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        lp.id,
        lp.pattern_type,
        lp.pattern_name,
        lp.confidence,
        lp.occurrence_count,
        lp.last_seen
    FROM learned_patterns lp
    WHERE lp.user_id = p_user_id
        AND lp.enabled = TRUE
        AND (p_pattern_type IS NULL OR lp.pattern_type = p_pattern_type)
    ORDER BY lp.confidence DESC, lp.last_seen DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function to get action patterns for a time range
CREATE OR REPLACE FUNCTION get_action_patterns_for_period(
    p_user_id TEXT DEFAULT 'default',
    p_start_time TIMESTAMPTZ DEFAULT NOW() - INTERVAL '24 hours',
    p_end_time TIMESTAMPTZ DEFAULT NOW(),
    p_bucket_size INTERVAL DEFAULT INTERVAL '1 hour'
) RETURNS TABLE(
    time_bucket TIMESTAMPTZ,
    action_type TEXT,
    target TEXT,
    application TEXT,
    occurrence_count BIGINT,
    avg_duration_ms NUMERIC
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        time_bucket(p_bucket_size, ua.time) AS time_bucket,
        ua.action_type,
        ua.target,
        ua.application,
        COUNT(*) AS occurrence_count,
        AVG(ua.duration_ms) AS avg_duration_ms
    FROM user_actions ua
    WHERE ua.user_id = p_user_id
        AND ua.time >= p_start_time
        AND ua.time <= p_end_time
    GROUP BY time_bucket, ua.action_type, ua.target, ua.application
    ORDER BY time_bucket DESC, occurrence_count DESC;
END;
$$ LANGUAGE plpgsql;

-- Function to update pattern confidence based on user feedback
CREATE OR REPLACE FUNCTION update_pattern_confidence(
    p_pattern_id UUID,
    p_user_response TEXT,
    p_feedback JSONB DEFAULT '{}'
) RETURNS VOID AS $$
DECLARE
    current_confidence REAL;
    new_confidence REAL;
BEGIN
    -- Get current confidence
    SELECT confidence INTO current_confidence
    FROM learned_patterns
    WHERE id = p_pattern_id;
    
    -- Calculate new confidence based on user response
    CASE p_user_response
        WHEN 'accepted' THEN
            new_confidence := LEAST(current_confidence + 0.1, 1.0);
        WHEN 'rejected' THEN
            new_confidence := GREATEST(current_confidence - 0.2, 0.0);
        WHEN 'dismissed' THEN
            new_confidence := GREATEST(current_confidence - 0.05, 0.0);
        ELSE
            new_confidence := current_confidence;
    END CASE;
    
    -- Update the pattern
    UPDATE learned_patterns
    SET confidence = new_confidence,
        user_feedback = user_feedback || p_feedback,
        updated_at = NOW()
    WHERE id = p_pattern_id;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to update timestamps
CREATE OR REPLACE FUNCTION update_modified_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply trigger to tables that need it
CREATE TRIGGER update_learned_patterns_modtime
    BEFORE UPDATE ON learned_patterns
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_ai_workflows_modtime
    BEFORE UPDATE ON ai_workflows
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();

CREATE TRIGGER update_user_preferences_modtime
    BEFORE UPDATE ON user_ai_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_modified_column();

-- Insert default user preferences
INSERT INTO user_ai_preferences (user_id, preferences)
VALUES ('default', '{
    "ai_enabled": true,
    "learning": {
        "enabled": true,
        "applications": true,
        "documents": true,
        "websites": true,
        "workflows": true,
        "min_confidence": 0.7,
        "min_occurrences": 5,
        "excluded_apps": ["1password", "keepassxc"],
        "excluded_paths": ["~/private", "~/secure"],
        "excluded_domains": ["*.bank.com", "*.health.gov"]
    },
    "suggestions": {
        "enabled": true,
        "display_mode": "toast",
        "max_per_hour": 3,
        "quiet_hours": ["22:00", "08:00"],
        "app_launch": true,
        "document_open": true,
        "website_visit": true,
        "workflow_automation": true
    },
    "privacy": {
        "local_only": true,
        "telemetry_enabled": false,
        "data_retention_days": 30,
        "encrypt_storage": true,
        "sensitive_data_filter": true
    }
}')
ON CONFLICT (user_id) DO NOTHING;

-- Grant permissions (adjust as needed for your security model)
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO horizonos;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO horizonos;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO horizonos;

-- Create indexes for better query performance
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_actions_context_gin ON user_actions USING GIN (context);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_learned_patterns_data_gin ON learned_patterns USING GIN (pattern_data);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_suggestions_action_gin ON ai_suggestions USING GIN (action_data);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_workflows_definition_gin ON ai_workflows USING GIN (workflow_definition);

-- Analysis and debugging views
CREATE VIEW IF NOT EXISTS pattern_performance AS
SELECT 
    lp.pattern_type,
    lp.pattern_name,
    lp.confidence,
    COUNT(s.id) as suggestion_count,
    COUNT(CASE WHEN s.user_response = 'accepted' THEN 1 END) as accepted_count,
    COUNT(CASE WHEN s.user_response = 'rejected' THEN 1 END) as rejected_count,
    ROUND(
        COUNT(CASE WHEN s.user_response = 'accepted' THEN 1 END)::NUMERIC / 
        NULLIF(COUNT(s.id), 0) * 100, 2
    ) as acceptance_rate
FROM learned_patterns lp
LEFT JOIN ai_suggestions s ON lp.id = s.pattern_id
WHERE lp.enabled = TRUE
GROUP BY lp.id, lp.pattern_type, lp.pattern_name, lp.confidence
ORDER BY acceptance_rate DESC NULLS LAST;

CREATE VIEW IF NOT EXISTS user_activity_summary AS
SELECT 
    user_id,
    DATE(time) as activity_date,
    COUNT(*) as total_actions,
    COUNT(DISTINCT action_type) as unique_action_types,
    COUNT(DISTINCT application) as unique_applications,
    MIN(time) as first_action,
    MAX(time) as last_action,
    EXTRACT(EPOCH FROM (MAX(time) - MIN(time))) / 60 as active_minutes
FROM user_actions
GROUP BY user_id, DATE(time)
ORDER BY activity_date DESC;

-- Create a function to clean up old suggestions
CREATE OR REPLACE FUNCTION cleanup_expired_suggestions() RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM ai_suggestions 
    WHERE expires_at < NOW() - INTERVAL '24 hours';
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Example data (remove in production)
INSERT INTO user_actions (time, action_type, target, application, duration_ms, context)
VALUES 
    (NOW() - INTERVAL '1 hour', 'app_launch', 'firefox', 'Firefox', 2000, '{"startup_type": "normal"}'),
    (NOW() - INTERVAL '45 minutes', 'web_navigate', 'github.com', 'Firefox', 0, '{"url": "https://github.com"}'),
    (NOW() - INTERVAL '30 minutes', 'file_open', 'document.pdf', 'Okular', 1500, '{"file_size": 1024000}'),
    (NOW() - INTERVAL '15 minutes', 'app_launch', 'code', 'VS Code', 3000, '{"workspace": "/home/user/project"}');

-- Final message
SELECT 'TimescaleDB schema initialized successfully for HorizonOS AI integration' AS status;