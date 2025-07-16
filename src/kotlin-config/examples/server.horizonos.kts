#!/usr/bin/env kotlin

import org.horizonos.config.dsl.*

/**
 * Server Configuration Example
 * 
 * A production-ready server configuration with:
 * - Web server (Nginx)
 * - Database (PostgreSQL)
 * - Container orchestration (Docker)
 * - Security hardening
 * - Monitoring and logging
 */

horizonOS {
    // System configuration
    hostname = "prod-web-01"
    timezone = "UTC"  // Always UTC for servers
    locale = "en_US.UTF-8"
    
    // Server packages
    packages {
        // Core server utilities
        install("base", "base-devel", "linux-lts", "linux-lts-headers")
        install("intel-ucode")  // Or amd-ucode for AMD systems
        
        // Web server stack
        install("nginx", "nginx-mod-brotli", "certbot", "certbot-nginx")
        install("postgresql", "postgresql-libs", "redis")
        
        // Container runtime
        install("docker", "docker-compose", "containerd")
        
        // Monitoring and logging
        install("prometheus-node-exporter", "grafana")
        install("rsyslog", "logrotate")
        
        // Security tools
        install("fail2ban", "ufw", "aide")
        install("rkhunter", "tripwire")
        
        // Backup tools
        install("borgbackup", "rsync")
        
        // System utilities
        install("htop", "iotop", "iftop", "ncdu")
        install("tmux", "vim", "git")
        install("net-tools", "bind-tools", "traceroute")
        
        // Remove unnecessary packages
        remove("wireless-tools", "wpa_supplicant")  // No WiFi on servers
    }
    
    // Service configuration for production
    services {
        // System services
        enable("systemd-timesyncd")
        enable("systemd-resolved")
        enable("sshd") {
            autoRestart = true
            restartOnFailure = true
            environment["PORT"] = "2222"  // Non-standard port
            environment["PermitRootLogin"] = "no"
            environment["PasswordAuthentication"] = "no"
        }
        
        // Web services
        enable("nginx") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("postgresql") {
            autoRestart = true
            environment["POSTGRES_MAX_CONNECTIONS"] = "200"
            environment["POSTGRES_SHARED_BUFFERS"] = "256MB"
        }
        enable("redis") {
            autoRestart = true
            environment["MAXMEMORY"] = "2gb"
            environment["MAXMEMORY_POLICY"] = "allkeys-lru"
        }
        
        // Container services
        enable("docker") {
            autoRestart = true
        }
        enable("containerd")
        
        // Security services
        enable("fail2ban") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("ufw")
        enable("aide")  // File integrity monitoring
        
        // Monitoring
        enable("prometheus-node-exporter")
        enable("grafana")
        
        // Backup service
        enable("borgmatic.timer")  // Automated backups
        
        // Disable unnecessary services
        disable("bluetooth")
        disable("cups")  // No printing
        disable("avahi-daemon")  // No mDNS
        disable("ModemManager")
    }
    
    // User configuration
    users {
        // Admin user (no root login)
        user("sysadmin") {
            uid = 1000
            shell = "/bin/bash"
            groups("wheel", "docker")
        }
        
        // Application users
        user("www-data") {
            uid = 33
            shell = "/usr/sbin/nologin"
            homeDir = "/var/www"
        }
        
        user("postgres") {
            uid = 70
            shell = "/usr/sbin/nologin"
            homeDir = "/var/lib/postgresql"
        }
        
        // Monitoring user
        user("prometheus") {
            uid = 9090
            shell = "/usr/sbin/nologin"
            homeDir = "/var/lib/prometheus"
        }
    }
    
    // Repository configuration
    repositories {
        // Use local mirror for faster updates
        add("core", "https://mirror.example.com/archlinux/core/os/x86_64") {
            priority = 1
        }
        add("extra", "https://mirror.example.com/archlinux/extra/os/x86_64") {
            priority = 2
        }
        
        // OSTree for atomic updates
        ostree("horizonos", "https://ostree.horizonos.org") {
            branch("stable")  // Only stable for production
            gpgVerify = true
            updatePolicy = "manual"  // No automatic updates
        }
    }
    
    // Server automation workflows
    automation {
        // Security updates only
        workflow("security-updates") {
            description = "Apply critical security updates"
            priority = 100
            
            trigger {
                time("02:00")
                onDays(TUESDAY, THURSDAY)  // Patch Tuesdays
            }
            
            conditions {
                // Only run during maintenance window
                timeWindow("02:00", "04:00")
            }
            
            actions {
                // Check for security updates only
                scriptBlock {
                    """
                    pacman -Sy
                    SECURITY_UPDATES=$(pacman -Qu | grep -E '(security|CVE)')
                    if [ ! -z "${'$'}SECURITY_UPDATES" ]; then
                        pacman -S --noconfirm ${'$'}SECURITY_UPDATES
                        systemctl daemon-reload
                    fi
                    """
                }
                
                // Log update activity
                runCommand("logger -t security-updates 'Security update check completed'")
            }
        }
        
        // Backup workflow
        workflow("nightly-backup") {
            description = "Nightly incremental backup"
            
            trigger {
                time("03:30")
                onDays(DAILY)
            }
            
            actions {
                // Database backup
                runCommand("pg_dumpall -U postgres > /backup/postgresql-$(date +%Y%m%d).sql")
                
                // Application data backup
                runCommand("""
                    borgbackup create \
                        --stats \
                        --compression lz4 \
                        /backup/borg::$(date +%Y%m%d) \
                        /var/www \
                        /etc/nginx \
                        /var/lib/redis \
                        /backup/*.sql
                """)
                
                // Prune old backups (keep 7 daily, 4 weekly, 6 monthly)
                runCommand("borgbackup prune --keep-daily 7 --keep-weekly 4 --keep-monthly 6 /backup/borg")
                
                // Sync to remote backup server
                runCommand("rsync -avz /backup/ backup@remote-server:/backups/prod-web-01/")
                
                // Clean up local database dumps older than 3 days
                runCommand("find /backup -name '*.sql' -mtime +3 -delete")
            }
            
            onError {
                runCommand("logger -t backup-failure 'Nightly backup failed'")
                email("ops@example.com", "Backup Failure on prod-web-01", "Check backup logs")
            }
        }
        
        // Certificate renewal
        workflow("cert-renewal") {
            description = "Check and renew SSL certificates"
            
            trigger {
                time("04:00")
                onDays(MONDAY, THURSDAY)
            }
            
            actions {
                // Renew certificates if needed
                runCommand("certbot renew --nginx --quiet")
                
                // Reload nginx if certificates were renewed
                scriptBlock {
                    """
                    if [ -f /var/log/letsencrypt/letsencrypt.log ]; then
                        if grep -q "Cert is due for renewal" /var/log/letsencrypt/letsencrypt.log; then
                            systemctl reload nginx
                            logger -t cert-renewal 'Certificates renewed and nginx reloaded'
                        fi
                    fi
                    """
                }
            }
        }
        
        // Log rotation and cleanup
        workflow("log-management") {
            description = "Rotate and archive logs"
            
            trigger {
                time("00:00")
                onDays(DAILY)
            }
            
            actions {
                // Force log rotation
                runCommand("logrotate -f /etc/logrotate.conf")
                
                // Archive old logs
                runCommand("tar -czf /var/log/archive/logs-$(date +%Y%m%d).tar.gz /var/log/*.1")
                
                // Clean up old archives (keep 30 days)
                runCommand("find /var/log/archive -name '*.tar.gz' -mtime +30 -delete")
                
                // Clean systemd journal (keep 1 week)
                runCommand("journalctl --vacuum-time=1week")
            }
        }
        
        // Health monitoring
        workflow("health-check") {
            description = "Monitor server health"
            
            trigger {
                interval(5.minutes)
            }
            
            actions {
                // Check disk space
                scriptBlock {
                    """
                    DISK_USAGE=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')
                    if [ ${'$'}DISK_USAGE -gt 85 ]; then
                        logger -t health-check "WARNING: Disk usage at ${'$'}DISK_USAGE%"
                        echo "Disk usage critical: ${'$'}DISK_USAGE%" | mail -s "Disk Alert: prod-web-01" ops@example.com
                    fi
                    """
                }
                
                // Check service health
                scriptBlock {
                    """
                    SERVICES="nginx postgresql redis docker"
                    for service in ${'$'}SERVICES; do
                        if ! systemctl is-active --quiet ${'$'}service; then
                            logger -t health-check "ERROR: ${'$'}service is not running"
                            systemctl restart ${'$'}service
                        fi
                    done
                    """
                }
                
                // Check memory usage
                scriptBlock {
                    """
                    MEM_USAGE=$(free | grep Mem | awk '{print int($3/$2 * 100)}')
                    if [ ${'$'}MEM_USAGE -gt 90 ]; then
                        logger -t health-check "WARNING: Memory usage at ${'$'}MEM_USAGE%"
                        # Clear caches if needed
                        sync && echo 3 > /proc/sys/vm/drop_caches
                    fi
                    """
                }
            }
        }
        
        // Security scanning
        workflow("security-scan") {
            description = "Regular security scanning"
            
            trigger {
                time("05:00")
                onDays(SUNDAY)  // Weekly scan
            }
            
            actions {
                // Update security tools
                runCommand("freshclam")
                runCommand("rkhunter --update")
                runCommand("aide --update")
                
                // Run scans
                runCommand("rkhunter --check --skip-keypress")
                runCommand("aide --check")
                runCommand("tripwire --check")
                
                // Check for failed login attempts
                scriptBlock {
                    """
                    FAILED_SSH=$(journalctl --since "1 week ago" | grep -c "Failed password for")
                    if [ ${'$'}FAILED_SSH -gt 100 ]; then
                        echo "High number of failed SSH attempts: ${'$'}FAILED_SSH" | \
                            mail -s "Security Alert: prod-web-01" security@example.com
                    fi
                    """
                }
                
                // Port scan detection
                runCommand("grep 'portscan' /var/log/fail2ban.log | mail -s 'Port Scan Report' security@example.com")
            }
        }
    }
}