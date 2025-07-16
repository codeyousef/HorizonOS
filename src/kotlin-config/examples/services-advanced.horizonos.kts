#!/usr/bin/env kotlin

@file:DependsOn("org.horizonos.config:kotlin-config:1.0.0")

import org.horizonos.config.dsl.*

horizonOS {
    hostname = "web-server-01"
    timezone = "UTC"
    locale = "en_US.UTF-8"

    // Enhanced Services Configuration Example
    // This example demonstrates a complete web application stack with:
    // - PostgreSQL database with replication
    // - Nginx web server with load balancing
    // - Redis for caching and sessions
    // - RabbitMQ for message queuing
    // - Docker containers for microservices
    // - Prometheus and Grafana for monitoring
    // - Custom systemd services

    enhancedServices {
        
        // === Database Services ===
        
        // Primary PostgreSQL database with high availability configuration
        database(DatabaseType.POSTGRESQL) {
            enabled = true
            autoStart = true
            port = 5432
            dataDirectory = "/var/lib/postgresql/data"
            version = "15"
            
            postgres {
                maxConnections = 200
                sharedBuffers = "512MB"
                effectiveCacheSize = "2GB"
                maintenanceWorkMem = "128MB"
                checkpointSegments = 64
                checkpointCompletionTarget = 0.9
                walBuffers = "32MB"
                defaultStatisticsTarget = 500
                
                // Enable query logging for performance analysis
                logMinDurationStatement = 1000  // Log queries taking > 1s
                logConnections = true
                logDisconnections = true
                logLockWaits = true
                logStatement = "ddl"
                logLinePrefix = "%t [%p]: [%l-1] user=%u,db=%d,app=%a,client=%h "
                
                // Essential extensions
                extension("pg_stat_statements")
                extension("pg_trgm")
                extension("uuid-ossp")
                extension("pgcrypto")
            }
            
            // Application database user
            user("webapp") {
                password = "webapp_secure_password_123!"
                privilege("SELECT")
                privilege("INSERT")
                privilege("UPDATE")
                privilege("DELETE")
                database("webapp_db")
                database("webapp_analytics")
                host = "%"  // Allow connections from any host
            }
            
            // Read-only user for analytics
            user("analytics") {
                password = "analytics_readonly_pass"
                privilege("SELECT")
                database("webapp_analytics")
                host = "analytics.internal"
            }
            
            // Application databases
            database("webapp_db") {
                charset = "UTF8"
                collation = "en_US.UTF-8"
                owner = "webapp"
            }
            
            database("webapp_analytics") {
                charset = "UTF8"
                collation = "en_US.UTF-8"
                owner = "webapp"
            }
            
            // Custom PostgreSQL configuration overrides
            config("max_wal_size", "2GB")
            config("min_wal_size", "1GB")
            config("random_page_cost", "1.1")  // SSD optimization
        }
        
        // Redis for caching and session storage
        database(DatabaseType.REDIS) {
            enabled = true
            autoStart = true
            port = 6379
            dataDirectory = "/var/lib/redis"
            
            redis {
                maxMemory = "2gb"
                maxMemoryPolicy = "allkeys-lru"
                persistenceMode = RedisPersistence.BOTH  // RDB + AOF
                rdbSavePolicy = listOf("900 1", "300 10", "60 10000")
                aofRewritePolicy = true
                tcpKeepalive = 300
                timeout = 0
                databases = 16
                requirepass = true
                password = "redis_secure_password_456!"
                
                // Master-slave replication setup
                replication {
                    masterHost = "redis-master.internal"
                    masterPort = 6379
                    masterTimeout = 60
                    replicationBacklogSize = "10mb"
                    replicationDisklessSync = true
                }
            }
        }
        
        // === Web Server Configuration ===
        
        // Nginx with advanced load balancing and SSL termination
        webServer(WebServerType.NGINX) {
            enabled = true
            autoStart = true
            port = 80
            sslPort = 443
            serverName = "webapp.example.com"
            documentRoot = "/var/www/html"
            logLevel = "warn"
            accessLog = "/var/log/nginx/access.log"
            errorLog = "/var/log/nginx/error.log"
            enableSSL = true
            sslCertificate = "/etc/ssl/certs/webapp.example.com.crt"
            sslCertificateKey = "/etc/ssl/private/webapp.example.com.key"
            
            nginx {
                workerProcesses = "auto"
                workerConnections = 4096
                keepaliveTimeout = 65
                clientMaxBodySize = "100m"
                gzipCompression = true
                gzipTypes = listOf(
                    "text/plain",
                    "text/css",
                    "text/xml",
                    "text/javascript",
                    "application/javascript",
                    "application/json",
                    "application/xml+rss",
                    "application/atom+xml",
                    "image/svg+xml"
                )
                
                // Backend application servers
                upstream("webapp_backend") {
                    server("10.0.1.10:3000")
                    server("10.0.1.11:3000")
                    server("10.0.1.12:3000")
                    loadBalancing = LoadBalancingMethod.LEAST_CONN
                    healthCheck = true
                }
                
                // API servers
                upstream("api_backend") {
                    server("10.0.2.10:8080")
                    server("10.0.2.11:8080")
                    loadBalancing = LoadBalancingMethod.IP_HASH
                    healthCheck = true
                }
                
                // Rate limiting for API endpoints
                rateLimit {
                    zone("api_limit") {
                        key = "\$binary_remote_addr"
                        size = "100m"
                        rate = "100r/m"  // 100 requests per minute
                    }
                    
                    zone("login_limit") {
                        key = "\$binary_remote_addr"
                        size = "10m"
                        rate = "5r/m"   // 5 login attempts per minute
                    }
                }
            }
            
            // Main application virtual host
            virtualHost("webapp.example.com") {
                port = 443
                documentRoot = "/var/www/webapp"
                sslEnabled = true
                sslCertificate = "/etc/ssl/certs/webapp.example.com.crt"
                sslCertificateKey = "/etc/ssl/private/webapp.example.com.key"
                
                // Static assets
                location("/static/") {
                    alias = "/var/www/webapp/static/"
                    header("Cache-Control", "public, max-age=31536000")
                    header("X-Content-Type-Options", "nosniff")
                }
                
                // Application proxy
                location("/") {
                    proxyPass = "http://webapp_backend"
                    header("X-Real-IP", "\$remote_addr")
                    header("X-Forwarded-For", "\$proxy_add_x_forwarded_for")
                    header("X-Forwarded-Proto", "https")
                    header("Host", "\$host")
                }
                
                // API endpoints with rate limiting
                location("/api/") {
                    proxyPass = "http://api_backend"
                    header("X-Real-IP", "\$remote_addr")
                    header("X-Rate-Limit-Zone", "api_limit")
                }
                
                // Login endpoint with stricter rate limiting
                location("/api/auth/login") {
                    proxyPass = "http://api_backend"
                    header("X-Rate-Limit-Zone", "login_limit")
                }
                
                // Health check endpoint (no rate limiting)
                location("/health") {
                    proxyPass = "http://webapp_backend"
                }
                
                // Redirect old URLs
                redirect("/old-app", "/", permanent = true)
                
                // Error pages
                errorPage(404, "/404.html")
                errorPage(500, "/500.html")
                errorPage(502, "/502.html")
                errorPage(503, "/503.html")
            }
            
            // API subdomain
            virtualHost("api.example.com") {
                port = 443
                sslEnabled = true
                sslCertificate = "/etc/ssl/certs/api.example.com.crt"
                sslCertificateKey = "/etc/ssl/private/api.example.com.key"
                
                location("/") {
                    proxyPass = "http://api_backend"
                    header("X-Real-IP", "\$remote_addr")
                    header("X-Forwarded-For", "\$proxy_add_x_forwarded_for")
                    header("X-Forwarded-Proto", "https")
                    header("Access-Control-Allow-Origin", "*")
                    header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
                    header("Access-Control-Allow-Headers", "Authorization, Content-Type")
                }
            }
            
            // Essential Nginx modules
            module("ssl")
            module("gzip")
            module("headers")
            module("upstream")
            module("limit_req")
            
            // Custom configuration overrides
            config("ssl_protocols", "TLSv1.2 TLSv1.3")
            config("ssl_ciphers", "ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512")
            config("ssl_prefer_server_ciphers", "off")
        }
        
        // === Message Queue Services ===
        
        // RabbitMQ for application messaging
        messageQueue(MessageQueueType.RABBITMQ) {
            enabled = true
            autoStart = true
            port = 5672
            managementPort = 15672
            
            rabbitmq {
                clusterEnabled = true
                clusterNode("rabbit@rabbitmq-01")
                clusterNode("rabbit@rabbitmq-02")
                clusterNode("rabbit@rabbitmq-03")
                memoryHighWatermark = 0.6
                diskFreeLimit = "10GB"
                heartbeat = 60
                
                // Essential plugins
                plugin("rabbitmq_management")
                plugin("rabbitmq_management_agent")
                plugin("rabbitmq_prometheus")
                plugin("rabbitmq_shovel")
                plugin("rabbitmq_federation")
                
                // Administrative user
                user("admin") {
                    password = "rabbitmq_admin_password_789!"
                    tag("administrator")
                    tag("management")
                    permission("/", ".*", ".*", ".*")
                }
                
                // Application user
                user("webapp") {
                    password = "rabbitmq_webapp_password"
                    tag("management")
                    permission("/webapp", "webapp\\..*", "webapp\\..*", "webapp\\..*")
                    permission("/logs", "", "", "logs\\..*")
                }
                
                // Background job user
                user("worker") {
                    password = "rabbitmq_worker_password"
                    permission("/webapp", "worker\\..*", "worker\\..*", "worker\\..*")
                }
                
                // Virtual hosts for isolation
                vhost("/webapp")    // Main application messaging
                vhost("/logs")      // Log aggregation
                vhost("/monitoring") // Monitoring data
            }
            
            // Custom RabbitMQ configuration
            config("vm_memory_high_watermark", "0.6")
            config("disk_free_limit", "10GB")
            config("cluster_formation.peer_discovery_backend", "rabbit_peer_discovery_consul")
        }
        
        // === Container Services ===
        
        // Docker for microservices and development environments
        container(ContainerRuntime.DOCKER) {
            enabled = true
            autoStart = true
            rootless = false
            storageDriver = "overlay2"
            
            // Container registries
            registry("docker.io") {
                username = "mycompany"
                password = "docker_registry_password"
                insecure = false
            }
            
            registry("registry.example.com") {
                username = "internal_user"
                password = "internal_registry_password"
                insecure = false
            }
            
            // Custom networks
            network("webapp_network") {
                driver = "bridge"
                subnet = "172.20.0.0/16"
                gateway = "172.20.0.1"
                ipRange = "172.20.1.0/24"
            }
            
            network("monitoring_network") {
                driver = "bridge"
                subnet = "172.21.0.0/16"
                gateway = "172.21.0.1"
            }
            
            // Persistent volumes
            volume("webapp_uploads") {
                driver = "local"
                mountPoint = "/var/lib/webapp/uploads"
                option("type", "nfs")
                option("o", "addr=nfs.example.com,rw")
                option("device", ":/var/nfs/uploads")
            }
            
            volume("postgres_data") {
                driver = "local"
                mountPoint = "/var/lib/postgresql/data"
            }
            
            volume("redis_data") {
                driver = "local"
                mountPoint = "/var/lib/redis/data"
            }
            
            // Microservice: User Authentication Service
            service("auth-service") {
                image = "registry.example.com/webapp/auth-service"
                tag = "v2.1.0"
                port(8081, 8080)
                volume("webapp_uploads", "/app/uploads", readOnly = false)
                env("DATABASE_URL", "postgresql://webapp:password@postgres:5432/webapp_db")
                env("REDIS_URL", "redis://redis:6379")
                env("JWT_SECRET", "auth_jwt_secret_key")
                env("LOG_LEVEL", "info")
                network("webapp_network")
                restart = RestartPolicy.UNLESS_STOPPED
                
                healthCheck {
                    test = listOf("CMD", "curl", "-f", "http://localhost:8080/health")
                    interval = "30s"
                    timeout = "10s"
                    retries = 3
                    startPeriod = "60s"
                }
                
                resources {
                    memory = "512m"
                    memorySwap = "1g"
                    cpus = "0.5"
                    cpuShares = 512
                }
            }
            
            // Microservice: Notification Service
            service("notification-service") {
                image = "registry.example.com/webapp/notification-service"
                tag = "v1.3.2"
                port(8082, 8080)
                env("RABBITMQ_URL", "amqp://webapp:password@rabbitmq:5672/webapp")
                env("SMTP_HOST", "smtp.example.com")
                env("SMTP_PORT", "587")
                env("SMTP_USER", "notifications@example.com")
                env("SMTP_PASS", "smtp_password")
                network("webapp_network")
                restart = RestartPolicy.UNLESS_STOPPED
                
                healthCheck {
                    test = listOf("CMD", "curl", "-f", "http://localhost:8080/health")
                    interval = "30s"
                    timeout = "5s"
                    retries = 3
                }
                
                resources {
                    memory = "256m"
                    cpus = "0.25"
                }
            }
            
            // Background job processor
            service("job-processor") {
                image = "registry.example.com/webapp/job-processor"
                tag = "v1.8.1"
                env("DATABASE_URL", "postgresql://webapp:password@postgres:5432/webapp_db")
                env("REDIS_URL", "redis://redis:6379")
                env("RABBITMQ_URL", "amqp://worker:password@rabbitmq:5672/webapp")
                env("WORKER_CONCURRENCY", "4")
                network("webapp_network")
                restart = RestartPolicy.UNLESS_STOPPED
                
                resources {
                    memory = "1g"
                    cpus = "1.0"
                }
            }
            
            // Build configuration for custom applications
            build {
                dockerfile("./webapp/Dockerfile") {
                    target = "production"
                    context = "./webapp"
                    tag("registry.example.com/webapp/main:latest")
                    tag("registry.example.com/webapp/main:v3.0.0")
                }
                
                dockerfile("./auth-service/Dockerfile") {
                    target = "production"
                    context = "./auth-service"
                    tag("registry.example.com/webapp/auth-service:latest")
                }
                
                arg("BUILD_VERSION", "3.0.0")
                arg("NODE_ENV", "production")
                arg("COMMIT_SHA", "abc123def456")
                
                cache {
                    enabled = true
                    maxSize = "20GB"
                    maxAge = "168h"  // 1 week
                }
            }
        }
        
        // === Monitoring Services ===
        
        // Prometheus for metrics collection
        monitoring(MonitoringType.PROMETHEUS) {
            enabled = true
            autoStart = true
            port = 9090
            dataRetention = "90d"  // 3 months of metrics
            
            prometheus {
                scrapeInterval = "15s"
                evaluationInterval = "15s"
                
                // Self-monitoring
                scrape("prometheus") {
                    target("localhost:9090")
                    scrapeInterval = "5s"
                    metricsPath = "/metrics"
                }
                
                // System metrics
                scrape("node-exporter") {
                    target("node1.internal:9100")
                    target("node2.internal:9100")
                    target("node3.internal:9100")
                    scrapeInterval = "30s"
                }
                
                // Application metrics
                scrape("webapp") {
                    target("localhost:3000")
                    target("10.0.1.10:3000")
                    target("10.0.1.11:3000")
                    target("10.0.1.12:3000")
                    metricsPath = "/metrics"
                }
                
                // Microservice metrics
                scrape("auth-service") {
                    target("auth-service:8080")
                    metricsPath = "/actuator/prometheus"
                }
                
                scrape("notification-service") {
                    target("notification-service:8080")
                    metricsPath = "/metrics"
                }
                
                // Database metrics
                scrape("postgres-exporter") {
                    target("postgres-exporter:9187")
                }
                
                scrape("redis-exporter") {
                    target("redis-exporter:9121")
                }
                
                // Message queue metrics
                scrape("rabbitmq") {
                    target("rabbitmq:15692")
                    metricsPath = "/metrics"
                }
                
                // Infrastructure metrics
                scrape("nginx-exporter") {
                    target("nginx-exporter:9113")
                }
                
                // Alert rules
                rule("/etc/prometheus/rules/webapp.yml")
                rule("/etc/prometheus/rules/infrastructure.yml")
                rule("/etc/prometheus/rules/database.yml")
                
                // Alertmanager configuration
                alertmanager {
                    target("alertmanager:9093")
                }
            }
            
            // Custom Prometheus configuration
            config("storage.tsdb.retention.time", "90d")
            config("storage.tsdb.retention.size", "50GB")
            config("query.max-concurrency", "20")
        }
        
        // Grafana for visualization and dashboards
        monitoring(MonitoringType.GRAFANA) {
            enabled = true
            autoStart = true
            port = 3000
            dataRetention = "30d"
            
            grafana {
                adminUser = "admin"
                adminPassword = "grafana_admin_password_secure!"
                allowSignUp = false
                
                // Data sources
                datasource("Prometheus") {
                    type = "prometheus"
                    url = "http://prometheus:9090"
                    access = "proxy"
                    isDefault = true
                }
                
                datasource("Loki") {
                    type = "loki"
                    url = "http://loki:3100"
                    access = "proxy"
                    isDefault = false
                }
                
                datasource("Jaeger") {
                    type = "jaeger"
                    url = "http://jaeger:16686"
                    access = "proxy"
                }
                
                // Pre-configured dashboards
                dashboard("Infrastructure Overview", "/var/lib/grafana/dashboards/infrastructure.json")
                dashboard("Application Metrics", "/var/lib/grafana/dashboards/application.json")
                dashboard("Database Performance", "/var/lib/grafana/dashboards/database.json")
                dashboard("Nginx Performance", "/var/lib/grafana/dashboards/nginx.json")
                dashboard("RabbitMQ Monitoring", "/var/lib/grafana/dashboards/rabbitmq.json")
                dashboard("Container Metrics", "/var/lib/grafana/dashboards/containers.json")
                dashboard("Business KPIs", "/var/lib/grafana/dashboards/business.json")
                
                // Useful plugins
                plugin("grafana-piechart-panel")
                plugin("grafana-worldmap-panel")
                plugin("grafana-polystat-panel")
                plugin("grafana-clock-panel")
                plugin("grafana-simple-json-datasource")
            }
            
            // Custom Grafana configuration
            config("server.http_port", "3000")
            config("security.admin_password", "grafana_admin_password_secure!")
            config("auth.anonymous.enabled", "false")
            config("snapshots.external_enabled", "false")
        }
        
        // === Custom Systemd Services ===
        
        // Main web application service
        systemdUnit("webapp-main") {
            unitType = SystemdUnitType.SERVICE
            description = "Main Web Application Service"
            after.add("network.target")
            after.add("postgresql.service")
            after.add("redis.service")
            requires.add("postgresql.service")
            wants.add("redis.service")
            
            execStart = "/usr/local/bin/webapp-server"
            execStop = "/usr/local/bin/webapp-server graceful-stop"
            execReload = "/usr/local/bin/webapp-server reload"
            workingDirectory = "/opt/webapp"
            user = "webapp"
            group = "webapp"
            
            env("NODE_ENV", "production")
            env("PORT", "3000")
            env("DATABASE_URL", "postgresql://webapp:password@localhost:5432/webapp_db")
            env("REDIS_URL", "redis://localhost:6379")
            env("RABBITMQ_URL", "amqp://webapp:password@localhost:5672/webapp")
            env("LOG_LEVEL", "info")
            env("SESSION_SECRET", "webapp_session_secret_key")
            
            restart = SystemdRestartPolicy.ALWAYS
            restartSec = 10
            type = SystemdServiceType.NOTIFY
            
            wantedBy.add("multi-user.target")
        }
        
        // Background job worker
        systemdUnit("webapp-worker") {
            unitType = SystemdUnitType.SERVICE
            description = "Web Application Background Worker"
            after.add("webapp-main.service")
            after.add("rabbitmq-server.service")
            requires.add("rabbitmq-server.service")
            wants.add("webapp-main.service")
            
            execStart = "/usr/local/bin/webapp-worker"
            execStop = "/bin/kill -TERM \$MAINPID"
            workingDirectory = "/opt/webapp"
            user = "webapp"
            group = "webapp"
            
            env("NODE_ENV", "production")
            env("DATABASE_URL", "postgresql://webapp:password@localhost:5432/webapp_db")
            env("REDIS_URL", "redis://localhost:6379")
            env("RABBITMQ_URL", "amqp://worker:password@localhost:5672/webapp")
            env("WORKER_CONCURRENCY", "8")
            env("LOG_LEVEL", "info")
            
            restart = SystemdRestartPolicy.ON_FAILURE
            restartSec = 30
            type = SystemdServiceType.SIMPLE
            
            wantedBy.add("multi-user.target")
        }
        
        // Log aggregation service
        systemdUnit("log-aggregator") {
            unitType = SystemdUnitType.SERVICE
            description = "Centralized Log Aggregation Service"
            after.add("network.target")
            after.add("rabbitmq-server.service")
            requires.add("rabbitmq-server.service")
            
            execStart = "/usr/local/bin/log-aggregator"
            execStop = "/bin/kill -INT \$MAINPID"
            workingDirectory = "/opt/logging"
            user = "logger"
            group = "logger"
            
            env("RABBITMQ_URL", "amqp://webapp:password@localhost:5672/logs")
            env("ELASTICSEARCH_URL", "http://elasticsearch:9200")
            env("LOG_RETENTION_DAYS", "30")
            env("BATCH_SIZE", "1000")
            
            restart = SystemdRestartPolicy.ALWAYS
            restartSec = 15
            type = SystemdServiceType.SIMPLE
            
            wantedBy.add("multi-user.target")
        }
        
        // Backup service (timer-based)
        systemdUnit("webapp-backup") {
            unitType = SystemdUnitType.SERVICE
            description = "Web Application Backup Service"
            after.add("postgresql.service")
            requires.add("postgresql.service")
            
            execStart = "/usr/local/bin/backup-webapp"
            workingDirectory = "/opt/backups"
            user = "backup"
            group = "backup"
            
            env("DATABASE_URL", "postgresql://backup_user:backup_pass@localhost:5432/webapp_db")
            env("BACKUP_DESTINATION", "s3://backups.example.com/webapp")
            env("RETENTION_DAYS", "30")
            env("AWS_ACCESS_KEY_ID", "backup_access_key")
            env("AWS_SECRET_ACCESS_KEY", "backup_secret_key")
            
            type = SystemdServiceType.ONESHOT
        }
        
        // Backup timer (runs daily at 2 AM)
        systemdUnit("webapp-backup") {
            unitType = SystemdUnitType.TIMER
            description = "Daily Web Application Backup Timer"
            requires.add("webapp-backup.service")
            
            wantedBy.add("timers.target")
        }
    }

    // === Package Dependencies ===
    packages {
        group("database") {
            install("postgresql", "postgresql-contrib", "redis")
        }
        
        group("webserver") {
            install("nginx", "nginx-mod-http-geoip2", "certbot", "certbot-nginx")
        }
        
        group("containers") {
            install("docker", "docker-compose", "docker-buildx")
        }
        
        group("messaging") {
            install("rabbitmq", "rabbitmq-server")
        }
        
        group("monitoring") {
            install("prometheus", "grafana", "node_exporter")
        }
        
        group("development") {
            install("git", "curl", "wget", "jq", "htop", "vim")
        }
    }

    // === System Services ===
    services {
        enable("postgresql") {
            autoRestart = true
            restartOnFailure = true
            env("PGDATA", "/var/lib/postgresql/data")
        }
        
        enable("redis") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("nginx") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("docker") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("rabbitmq-server") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("prometheus") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("grafana-server") {
            autoRestart = true
            restartOnFailure = true
        }
        
        // Enable our custom services
        enable("webapp-main")
        enable("webapp-worker")
        enable("log-aggregator")
        enable("webapp-backup.timer")
        
        // Disable unnecessary services for security
        disable("telnet")
        disable("rsh")
        disable("rlogin")
    }

    // === Users ===
    users {
        user("webapp") {
            uid = 1001
            shell = "/usr/bin/bash"
            groups("webapp", "docker")
        }
        
        user("logger") {
            uid = 1002
            shell = "/usr/bin/bash"
            groups("logger")
        }
        
        user("backup") {
            uid = 1003
            shell = "/usr/bin/bash"
            groups("backup")
        }
    }
}