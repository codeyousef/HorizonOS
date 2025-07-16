import org.horizonos.config.dsl.*

horizonOS {
    // System configuration
    hostname = "horizonos-security"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Comprehensive security configuration
    security {
        enabled = true
        
        // PAM (Pluggable Authentication Modules) configuration
        pam {
            enabled = true
            failDelay = 3.seconds
            maxTries = 3
            lockoutTime = 15.minutes
            
            // Password policy configuration
            password {
                minLength = 14
                requireDigits = true
                requireLowercase = true
                requireUppercase = true
                requireSpecialChars = true
                maxSequential = 2
                history = 12
                
                // Dictionary checks
                dictionary {
                    enabled = true
                    customWords = listOf("horizonos", "company", "password", "admin")
                }
                
                // Password complexity rules
                complexity {
                    enabled = true
                    minClasses = 3
                    maxRepeat = 2
                    rejectUsername = true
                    enforceForRoot = true
                }
            }
            
            // Account management
            account {
                // Account lockout policy
                lockout {
                    enabled = true
                    denyRetries = 3
                    unlockTime = 30.minutes
                    evenDenyRoot = false
                }
                
                // Time restrictions
                time {
                    enabled = true
                    restrictions = listOf(
                        TimeRestriction("developer", listOf("0700-1900"), listOf("Mo", "Tu", "We", "Th", "Fr")),
                        TimeRestriction("admin", listOf("0000-2359"), listOf("Mo", "Tu", "We", "Th", "Fr", "Sa", "Su")),
                        TimeRestriction("maintenance", listOf("0200-0600"), listOf("Sa", "Su"))
                    )
                }
            }
            
            // Session management
            session {
                // Resource limits
                limits {
                    enabled = true
                    rules = listOf(
                        LimitRule("@users", LimitType.NPROC, LimitItem.HARD, 500),
                        LimitRule("@users", LimitType.NOFILE, LimitItem.SOFT, 32768),
                        LimitRule("@users", LimitType.NOFILE, LimitItem.HARD, 65536),
                        LimitRule("admin", LimitType.NPROC, LimitItem.HARD, 2000),
                        LimitRule("@developers", LimitType.MEMLOCK, LimitItem.HARD, 8388608)
                    )
                }
                
                // Umask settings
                umask {
                    enabled = true
                    default = "0022"
                    userMask = "0077"
                }
            }
        }
        
        // SSH configuration
        ssh {
            enabled = true
            port = 2222
            permitRootLogin = PermitRootLogin.PROHIBIT_PASSWORD
            passwordAuthentication = false
            pubkeyAuthentication = true
            challengeResponseAuthentication = false
            kbdInteractiveAuthentication = false
            x11Forwarding = false
            agentForwarding = false
            tcpForwarding = false
            
            // Protocol configuration
            protocol {
                version = "2"
                ciphers = listOf(
                    "chacha20-poly1305@openssh.com",
                    "aes256-gcm@openssh.com",
                    "aes128-gcm@openssh.com",
                    "aes256-ctr",
                    "aes192-ctr",
                    "aes128-ctr"
                )
                macs = listOf(
                    "hmac-sha2-256-etm@openssh.com",
                    "hmac-sha2-512-etm@openssh.com",
                    "hmac-sha2-256",
                    "hmac-sha2-512"
                )
                kexAlgorithms = listOf(
                    "curve25519-sha256@libssh.org",
                    "diffie-hellman-group16-sha512",
                    "diffie-hellman-group18-sha512",
                    "diffie-hellman-group-exchange-sha256"
                )
                hostKeyAlgorithms = listOf(
                    "ssh-ed25519",
                    "rsa-sha2-512",
                    "rsa-sha2-256",
                    "ecdsa-sha2-nistp521"
                )
            }
            
            // Access control
            access {
                allowUsers = listOf("admin", "developer", "maintenance")
                denyUsers = listOf("guest", "nobody")
                allowGroups = listOf("ssh-users", "admins", "developers")
                denyGroups = listOf("restricted", "guests")
                maxAuthTries = 3
                maxSessions = 5
                maxStartups = "3:50:10"
                loginGraceTime = 60.seconds
            }
            
            // Security settings
            security {
                strictModes = true
                ignoredRhosts = true
                hostbasedAuthentication = false
                emptyPasswords = false
                permitUserEnvironment = false
                compression = SSHCompression.DELAYED
                tcpKeepAlive = true
                clientAliveInterval = 300.seconds
                clientAliveCountMax = 3
                useDNS = false
                
                // Rate limiting
                rateLimit {
                    enabled = true
                    maxConnections = 3
                    timeWindow = 60.seconds
                    blockDuration = 10.minutes
                }
                
                // Geographic blocking
                geoBlocking {
                    enabled = true
                    allowedCountries = listOf("US", "CA", "GB", "DE", "FR")
                    blockedCountries = listOf("CN", "RU", "KP", "IR")
                    whitelistIPs = listOf("192.168.0.0/16", "10.0.0.0/8", "172.16.0.0/12")
                    blacklistIPs = listOf("185.220.100.0/24", "185.220.101.0/24")
                }
            }
            
            // Key management
            keyManagement {
                autoRotation = true
                rotationInterval = 90.days
                keyStrength = 4096
                hostKeyPath = "/etc/ssh"
                
                // Authorized keys management
                authorizedKeys {
                    strictMode = true
                    maxKeys = 5
                    keyTypes = listOf(KeyType.ED25519, KeyType.RSA, KeyType.ECDSA)
                    expiration = 365.days
                    enforceKeyComment = true
                    allowAgentForwarding = false
                }
            }
            
            // Logging and monitoring
            logging {
                enabled = true
                level = SSHLogLevel.VERBOSE
                facility = SSHLogFacility.AUTH
                verboseMode = true
                
                // Audit configuration
                audit {
                    enabled = true
                    logConnections = true
                    logCommands = true
                    logSFTP = true
                    logEnvironment = false
                    compressionLevel = 6
                    maxFileSize = "100M"
                    
                    // Session recording
                    sessionRecording {
                        enabled = true
                        recordInput = true
                        recordOutput = true
                        recordTiming = true
                        storageLocation = "/var/log/ssh-sessions"
                        compressionEnabled = true
                        retentionPeriod = 90.days
                    }
                }
            }
        }
        
        // Sudo configuration
        sudo {
            enabled = true
            
            // Administrative rules
            rule {
                users("admin")
                commands("ALL")
                hosts("ALL")
                runAs("ALL")
                options(SudoOption.NOPASSWD, SudoOption.SETENV)
            }
            
            // System administrators
            rule {
                groups("wheel")
                commands("/usr/bin/systemctl", "/usr/bin/journalctl", "/usr/bin/mount", "/usr/bin/umount")
                hosts("localhost")
                runAs("root")
                options(SudoOption.PASSWD, SudoOption.LOG_INPUT, SudoOption.LOG_OUTPUT)
                timeout = 15.minutes
            }
            
            // Developers
            rule {
                users("developer")
                commands("/usr/bin/docker", "/usr/bin/podman", "/usr/bin/kubectl", "/usr/bin/systemctl restart nginx")
                hosts("localhost")
                runAs("root")
                noPassword = false
                timeout = 10.minutes
            }
            
            // Database administrators
            rule {
                groups("dba")
                commands("/usr/bin/mysql", "/usr/bin/psql", "/usr/bin/mongo")
                runAs("postgres", "mysql", "mongodb")
                options(SudoOption.PASSWD)
            }
            
            // Backup operations
            rule {
                users("backup")
                commands("/usr/bin/rsync", "/usr/bin/tar", "/usr/bin/gzip", "/bin/cp")
                hosts("ALL")
                runAs("root")
                options(SudoOption.NOPASSWD)
            }
            
            // Default settings
            defaults {
                targetPw = false
                rootPw = false
                runAsNonRoot = false
                preserveEnv = false
                requireTty = true
                visiblepw = false
                pwFeedback = false
                fastGlob = true
                insults = false
                
                // Security defaults
                security {
                    usesPty = true
                    closeSessions = true
                    pamService = "sudo"
                    pam = true
                    mailAlways = false
                    mailBadpass = true
                    mailNoHost = true
                    mailNoPerms = true
                    mailNoUser = true
                    mailto = "security@horizonos.local"
                }
                
                // Environment handling
                environment {
                    resetEnv = true
                    keepEnv = listOf("LANG", "LC_*", "TERM", "PATH", "DISPLAY", "XAUTHORITY")
                    checkEnv = listOf("HOME", "USER", "LOGNAME", "USERNAME")
                    setEnv = mapOf(
                        "EDITOR" to "vim",
                        "PAGER" to "less",
                        "SECURE_PATH" to "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
                    )
                }
                
                // Comprehensive logging
                logging {
                    enabled = true
                    logFile = "/var/log/sudo.log"
                    logHost = true
                    logYear = true
                    logInput = true
                    logOutput = true
                    maxLogSize = "50M"
                    
                    // Syslog integration
                    syslog {
                        enabled = true
                        facility = SyslogFacility.AUTHPRIV
                        priority = SyslogPriority.NOTICE
                        maxLength = 2048
                        includeProcessId = true
                    }
                }
                
                // Timeout settings
                timeout {
                    passwd = 5.minutes
                    timestamp = 15.minutes
                    lectureFile = "/etc/sudo_lecture"
                    lecture = SudoLecture.ONCE
                }
            }
        }
        
        // SELinux configuration
        selinux {
            enabled = true
            mode = SELinuxMode.ENFORCING
            policy = SELinuxPolicy.TARGETED
            
            // Boolean settings
            booleans {
                set("httpd_can_network_connect", true)
                set("httpd_can_network_connect_db", true)
                set("httpd_execmem", false)
                set("samba_enable_home_dirs", false)
                set("samba_export_all_rw", false)
                set("ftpd_anon_write", false)
                set("allow_execstack", false)
                set("deny_ptrace", true)
            }
            
            // Custom modules
            modules {
                install("myapp.pp", "nginx-custom.pp", "docker-custom.pp")
                enable("apache", "nginx", "docker")
                disable("games", "wine", "unconfined")
            }
            
            // Port labeling
            ports {
                add(8080, SELinuxPortType.HTTP_PORT_T, SELinuxProtocol.TCP)
                add(8443, SELinuxPortType.HTTP_PORT_T, SELinuxProtocol.TCP)
                add(3306, SELinuxPortType.MYSQLD_PORT_T, SELinuxProtocol.TCP)
                add(5432, SELinuxPortType.POSTGRESQL_PORT_T, SELinuxProtocol.TCP)
                add(27017, SELinuxPortType.MONGOD_PORT_T, SELinuxProtocol.TCP)
            }
            
            // File contexts
            fileContexts {
                add("/opt/myapp(/.*)?", "httpd_exec_t")
                add("/var/log/myapp(/.*)?", "httpd_log_t")
                add("/etc/myapp(/.*)?", "httpd_config_t")
                add("/var/lib/myapp(/.*)?", "httpd_var_lib_t")
                add("/usr/local/bin/myapp", "httpd_exec_t")
            }
            
            // User mappings
            users {
                map("admin", "unconfined_u")
                map("webuser", "user_u")
                map("dbuser", "user_u")
                map("guest", "guest_u")
            }
            
            // Logging configuration
            logging {
                enabled = true
                auditDenials = true
                verboseLogging = true
                logFile = "/var/log/selinux.log"
                maxLogSize = "100M"
                
                // AVC (Access Vector Cache) logging
                avc {
                    enabled = true
                    logLevel = SELinuxLogLevel.INFO
                    rateLimit = 100
                    burstLimit = 1000
                }
            }
        }
        
        // AppArmor configuration
        apparmor {
            enabled = true
            
            // Application profiles
            profiles {
                // Firefox profile
                profile("/usr/bin/firefox") {
                    mode = AppArmorMode.ENFORCE
                    
                    capabilities {
                        allow("net_bind_service", "setgid", "setuid")
                        deny("sys_admin", "sys_ptrace", "sys_rawio")
                    }
                    
                    files {
                        rule("/home/**/Downloads/**", "rw")
                        rule("/home/**/.mozilla/**", "rw")
                        rule("/tmp/**", "rw")
                        rule("/usr/lib/firefox/**", "r")
                        rule("/usr/share/firefox/**", "r")
                        rule("/etc/passwd", "r")
                        rule("/etc/group", "r")
                        rule("/etc/nsswitch.conf", "r")
                        rule("/proc/*/stat", "r")
                        rule("/proc/*/status", "r")
                        rule("/sys/devices/**/uevent", "r")
                        deny("/etc/shadow")
                        deny("/root/**")
                        deny("/home/*/.ssh/**")
                    }
                    
                    network {
                        allow(NetworkFamily.INET, SocketType.STREAM)
                        allow(NetworkFamily.INET6, SocketType.STREAM)
                        allow(NetworkFamily.INET, SocketType.DGRAM)
                        deny(NetworkFamily.UNIX, SocketType.RAW)
                    }
                    
                    dbus {
                        rule(DbusType.SESSION, "org.freedesktop.Notifications", "Notify")
                        rule(DbusType.SESSION, "org.kde.StatusNotifierWatcher", "*")
                        rule(DbusType.SYSTEM, "org.freedesktop.NetworkManager", "GetDevices")
                        rule(DbusType.SYSTEM, "org.freedesktop.UPower", "EnumerateDevices")
                    }
                }
                
                // Web server profile
                profile("/usr/bin/nginx") {
                    mode = AppArmorMode.ENFORCE
                    include("/etc/apparmor.d/abstractions/base")
                    include("/etc/apparmor.d/abstractions/nameservice")
                    include("/etc/apparmor.d/abstractions/openssl")
                    
                    capabilities {
                        allow("dac_override", "setgid", "setuid", "net_bind_service")
                        deny("sys_admin", "sys_ptrace")
                    }
                    
                    files {
                        rule("/var/www/**", "r")
                        rule("/var/log/nginx/**", "rw")
                        rule("/etc/nginx/**", "r")
                        rule("/run/nginx.pid", "rw")
                        rule("/tmp/nginx/**", "rw")
                        deny("/etc/shadow")
                        deny("/home/**")
                    }
                    
                    network {
                        allow(NetworkFamily.INET, SocketType.STREAM)
                        allow(NetworkFamily.INET6, SocketType.STREAM)
                        allow(NetworkFamily.UNIX, SocketType.STREAM)
                    }
                }
                
                // Custom application profile
                profile("/usr/local/bin/myapp") {
                    mode = AppArmorMode.COMPLAIN
                    include("/etc/apparmor.d/abstractions/base")
                    include("/etc/apparmor.d/abstractions/consoles")
                    
                    capabilities {
                        allow("net_bind_service")
                    }
                    
                    files {
                        rule("/opt/myapp/**", "r")
                        rule("/var/lib/myapp/**", "rw")
                        rule("/var/log/myapp/**", "rw")
                        rule("/etc/myapp/**", "r")
                    }
                }
            }
            
            // Global tunables
            tunables {
                set("@{HOME}", "/home/*")
                set("@{PROC}", "/proc/*")
                set("@{SYS}", "/sys/*")
                set("@{TFTP_DIR}", "/var/lib/tftpboot/")
                set("@{HOMEDIRS}", "/home/")
            }
            
            // Logging configuration
            logging {
                enabled = true
                auditMode = AppArmorAuditMode.ALL
                rate = 10
                maxLogSize = "50M"
                logFile = "/var/log/apparmor.log"
            }
        }
        
        // Firewall configuration
        firewall {
            enabled = true
            backend = FirewallBackend.FIREWALLD
            defaultZone = "drop"
            panicMode = false
            
            // Zone configuration
            zones {
                // Public zone for untrusted networks
                zone("public") {
                    target = ZoneTarget.DEFAULT
                    description = "Public zone for external networks"
                    
                    services("ssh")
                    ports("80/tcp", "443/tcp")
                    
                    interfaces("eth0")
                    sources()
                    
                    rich {
                        rule {
                            source("192.168.1.0/24")
                            service("ssh")
                            action = RichRuleAction.ACCEPT
                            log {
                                prefix = "SSH-ALLOW"
                                level = LogLevel.INFO
                            }
                        }
                        
                        rule {
                            source("0.0.0.0/0")
                            port("22", "tcp")
                            action = RichRuleAction.DROP
                            log {
                                prefix = "SSH-BLOCK"
                                level = LogLevel.WARNING
                            }
                        }
                    }
                    
                    masquerade = false
                    icmpBlocks("echo-request", "echo-reply", "timestamp-request")
                }
                
                // Internal zone for trusted networks
                zone("internal") {
                    target = ZoneTarget.ACCEPT
                    description = "Internal trusted network"
                    
                    services("ssh", "http", "https", "dns", "dhcp")
                    ports("8080/tcp", "9090/tcp", "3306/tcp", "5432/tcp")
                    
                    interfaces("eth1")
                    sources("192.168.0.0/16", "10.0.0.0/8")
                    
                    masquerade = true
                    forwardPorts {
                        forward(80, "tcp", 8080, "192.168.1.100")
                        forward(443, "tcp", 8443, "192.168.1.100")
                        forward(3306, "tcp", 3306, "192.168.1.200")
                    }
                }
                
                // DMZ zone for servers
                zone("dmz") {
                    target = ZoneTarget.DEFAULT
                    description = "DMZ for web servers and services"
                    
                    services("http", "https", "smtp", "pop3s", "imaps")
                    ports("8443/tcp", "25/tcp", "587/tcp", "993/tcp", "995/tcp")
                    
                    sources("0.0.0.0/0")
                    
                    rich {
                        rule {
                            source("185.199.108.0/22")
                            service("https")
                            action = RichRuleAction.ACCEPT
                        }
                        
                        rule {
                            protocol("icmp")
                            icmpType("echo-request")
                            action = RichRuleAction.DROP
                        }
                    }
                    
                    masquerade = false
                }
            }
            
            // IP sets for dynamic blocking
            ipsets {
                ipset("blacklist") {
                    type = IpsetType.HASH_IP
                    family = AddressFamily.INET
                    maxelem = 65536
                    timeout = 3600
                    entries("192.0.2.1", "198.51.100.1", "203.0.113.1")
                }
                
                ipset("whitelist") {
                    type = IpsetType.HASH_NET
                    family = AddressFamily.INET
                    maxelem = 1024
                    entries("192.168.0.0/16", "10.0.0.0/8", "172.16.0.0/12", "127.0.0.0/8")
                }
                
                ipset("ssh-brute") {
                    type = IpsetType.HASH_IP
                    family = AddressFamily.INET
                    maxelem = 32768
                    timeout = 86400
                    entries()
                }
            }
            
            // Direct iptables rules for advanced configuration
            iptables {
                enabled = true
                policy = IptablesPolicy.DROP
                logDropped = true
                
                rules {
                    // Allow loopback
                    rule {
                        chain = Chain.INPUT
                        inputInterface = "lo"
                        target = Target.ACCEPT
                    }
                    
                    rule {
                        chain = Chain.OUTPUT
                        outputInterface = "lo"
                        target = Target.ACCEPT
                    }
                    
                    // Allow established and related connections
                    rule {
                        chain = Chain.INPUT
                        match = "state"
                        states = listOf("ESTABLISHED", "RELATED")
                        target = Target.ACCEPT
                    }
                    
                    // SSH with rate limiting
                    rule {
                        chain = Chain.INPUT
                        protocol = "tcp"
                        destinationPort = "2222"
                        source = "192.168.1.0/24"
                        match = "recent"
                        recentName = "ssh"
                        recentUpdate = true
                        recentSeconds = 60
                        recentHitcount = 3
                        target = Target.DROP
                    }
                    
                    rule {
                        chain = Chain.INPUT
                        protocol = "tcp"
                        destinationPort = "2222"
                        source = "192.168.1.0/24"
                        match = "recent"
                        recentName = "ssh"
                        recentSet = true
                        target = Target.ACCEPT
                    }
                    
                    // Web server rules
                    rule {
                        chain = Chain.INPUT
                        protocol = "tcp"
                        destinationPort = "80"
                        target = Target.ACCEPT
                    }
                    
                    rule {
                        chain = Chain.INPUT
                        protocol = "tcp"
                        destinationPort = "443"
                        target = Target.ACCEPT
                    }
                    
                    // ICMP rules
                    rule {
                        chain = Chain.INPUT
                        protocol = "icmp"
                        icmpType = "echo-request"
                        match = "limit"
                        limitRate = "1/second"
                        limitBurst = 2
                        target = Target.ACCEPT
                    }
                    
                    // Drop invalid packets
                    rule {
                        chain = Chain.INPUT
                        match = "state"
                        states = listOf("INVALID")
                        target = Target.DROP
                    }
                    
                    // Log dropped packets
                    rule {
                        chain = Chain.INPUT
                        target = Target.LOG
                        logPrefix = "IPTABLES-DROP"
                        logLevel = 4
                    }
                }
            }
            
            // Port knocking for SSH
            portKnocking {
                enabled = true
                
                sequence("ssh-admin") {
                    ports(7000, 8000, 9000)
                    protocol = "tcp"
                    timeout = 30.seconds
                    openPort = 2222
                    openDuration = 10.minutes
                    source = "192.168.1.0/24"
                }
                
                sequence("emergency-access") {
                    ports(1234, 5678, 9012)
                    protocol = "udp"
                    timeout = 60.seconds
                    openPort = 22
                    openDuration = 5.minutes
                    source = "10.0.0.0/8"
                }
            }
            
            // DDoS protection
            ddosProtection {
                enabled = true
                connectionLimit = 50
                rateLimit = "5/second"
                burstLimit = 10
                
                synFlood {
                    enabled = true
                    rate = "1/second"
                    burst = 3
                    limit = 1000
                }
                
                pingFlood {
                    enabled = true
                    rate = "1/second"
                    burst = 2
                }
                
                portScan {
                    enabled = true
                    hitcount = 10
                    seconds = 60
                    blockTime = 3600
                }
            }
        }
        
        // TPM (Trusted Platform Module) configuration
        tpm {
            enabled = true
            version = TPMVersion.TPM2
            
            // Hierarchy configuration
            hierarchy {
                owner = TPMHierarchy.ENABLED
                endorsement = TPMHierarchy.ENABLED
                platform = TPMHierarchy.ENABLED
                lockout = TPMHierarchy.DISABLED
            }
            
            // PCR (Platform Configuration Register) management
            pcrs {
                policy {
                    useFor = listOf(PCRUseCase.DISK_ENCRYPTION, PCRUseCase.SECURE_BOOT, PCRUseCase.SYSTEM_INTEGRITY)
                    pcrBanks = listOf(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 14, 15)
                    hashAlgorithm = TPMHashAlgorithm.SHA256
                    extendPolicy = PCRExtendPolicy.BOOTLOADER_ONLY
                }
                
                measurements {
                    bootMeasurements = true
                    kernelMeasurements = true
                    initrdMeasurements = true
                    applicationMeasurements = true
                    configMeasurements = true
                }
                
                attestation {
                    enabled = true
                    remoteVerification = false
                    quoteInterval = 6.hours
                    nonceSize = 32
                    includeEventLog = true
                }
            }
            
            // Key management
            keys {
                storageRootKey {
                    algorithm = TPMKeyAlgorithm.RSA2048
                    persistent = true
                    handle = 0x81000001
                    authorization = "owner"
                }
                
                endorsementKey {
                    algorithm = TPMKeyAlgorithm.RSA2048
                    persistent = true
                    certificate = true
                    hierarchy = "endorsement"
                }
                
                attestationKey {
                    algorithm = TPMKeyAlgorithm.ECC_NIST_P256
                    restricted = true
                    signOnly = true
                    parentHandle = 0x81000001
                }
            }
            
            // IMA/EVM (Integrity Measurement Architecture/Extended Verification Module)
            ima {
                enabled = true
                template = IMATemplate.IMA_NG
                hashAlgorithm = TPMHashAlgorithm.SHA256
                
                policy {
                    measureBootAggregate = true
                    measureExecutables = true
                    measureLibraries = true
                    measureModules = true
                    measureFirmware = true
                    measureConfigFiles = true
                    
                    rules = listOf(
                        IMARule(IMAAction.MEASURE, "/usr/bin/**"),
                        IMARule(IMAAction.MEASURE, "/usr/lib/**"),
                        IMARule(IMAAction.MEASURE, "/usr/sbin/**"),
                        IMARule(IMAAction.APPRAISE, "/etc/**"),
                        IMARule(IMAAction.APPRAISE, "/boot/**"),
                        IMARule(IMAAction.DONT_MEASURE, "/tmp/**"),
                        IMARule(IMAAction.DONT_MEASURE, "/var/tmp/**")
                    )
                }
                
                keyring {
                    name = ".ima"
                    certificates = listOf("/etc/keys/ima-cert.pem", "/etc/keys/ima-ca.pem")
                    trustedKeys = listOf("/etc/keys/trusted-key.pem")
                }
            }
            
            // Clevis integration for automatic LUKS unlocking
            clevis {
                enabled = true
                
                tpm2 {
                    enabled = true
                    keyFile = "/etc/clevis/tpm2.key"
                    pcrBank = TPMHashAlgorithm.SHA256
                    pcrIds = listOf(0, 1, 2, 3, 4, 5, 6, 7)
                    policyDigest = ""
                }
                
                tang {
                    enabled = false
                    servers = listOf("tang.example.com:7500", "tang2.example.com:7500")
                    threshold = 1
                    recoveryPin = true
                }
                
                sss {
                    enabled = false
                    threshold = 2
                    pins = listOf("tpm2", "tang")
                }
            }
        }
        
        // GPG configuration
        gpg {
            enabled = true
            
            // Key management
            keys {
                generate {
                    keyType = GPGKeyType.RSA
                    keyLength = 4096
                    subkeys = true
                    subkeyType = GPGKeyType.RSA
                    subkeyLength = 4096
                    expiry = 2.years
                    
                    identity {
                        name = "HorizonOS Security Administrator"
                        email = "security@horizonos.local"
                        comment = "System Security Key"
                    }
                    
                    preferences {
                        cipherAlgorithms = listOf("AES256", "AES192", "AES")
                        hashAlgorithms = listOf("SHA512", "SHA256", "SHA224", "SHA1")
                        compressionAlgorithms = listOf("ZLIB", "BZIP2", "ZIP", "Uncompressed")
                        features = listOf("MDC", "AEAD")
                    }
                }
                
                import {
                    keyFiles = listOf(
                        "/etc/gpg/admin-public.asc",
                        "/etc/gpg/backup-key.asc",
                        "/etc/gpg/ca-key.asc"
                    )
                    keyservers = listOf(
                        "hkps://keys.openpgp.org",
                        "hkps://keyserver.ubuntu.com",
                        "hkps://pgp.mit.edu"
                    )
                    autoImport = true
                    verifySignatures = true
                    importTrust = false
                }
                
                management {
                    autoExpire = true
                    renewalWarning = 30.days
                    backupLocation = "/etc/gpg/backups"
                    encryptBackups = true
                    
                    rotation {
                        enabled = true
                        interval = 1.years
                        keepOldKeys = 3
                        notifyUsers = true
                        autoRotate = false
                    }
                    
                    revocation {
                        enabled = true
                        certificateLocation = "/etc/gpg/revocation-certs"
                        autoGenerate = true
                        reason = "Key compromised"
                    }
                }
            }
            
            // GPG agent configuration
            agent {
                enabled = true
                defaultCacheTtl = 2.hours
                maxCacheTtl = 8.hours
                pinentryProgram = "/usr/bin/pinentry-qt"
                
                caching {
                    enableSshSupport = true
                    grabKeyboard = false
                    grabPointer = false
                    allowMarkTrusted = true
                    disableScdaemon = false
                    ignoreCache = false
                }
                
                security {
                    allowPresetPassphrase = false
                    allowLoopbackPinentry = false
                    enforcePassphraseConstraints = true
                    minPassphraseLen = 12
                    minPassphraseNonalpha = 2
                    checkPassphrasePattern = true
                    forbidNullPin = true
                }
            }
            
            // Signing policies
            signing {
                autoSign = false
                defaultKey = "security@horizonos.local"
                
                policies {
                    requireSignature = listOf("/etc/**", "/usr/bin/**", "/opt/**")
                    trustedSigners = listOf("security@horizonos.local", "admin@horizonos.local")
                    verifySignatures = true
                    rejectUnsigned = false
                    warnOnExpiry = true
                }
                
                email {
                    enabled = true
                    autoEncrypt = true
                    autoSign = true
                    preferredKeyserver = "hkps://keys.openpgp.org"
                    encryptToSelf = true
                    trustModel = GPGTrustModel.PGP
                }
            }
            
            // Keyserver configuration
            keyserver {
                enabled = true
                defaultKeyserver = "hkps://keys.openpgp.org"
                autoRetrieve = true
                autoKeyImport = false
                honorKeyserverUrl = true
                includeCleartext = false
                
                hkp {
                    enabled = true
                    proxy = ""
                    port = 11371
                    timeout = 30.seconds
                    maxTries = 3
                }
                
                hkps {
                    enabled = true
                    caFile = "/etc/ssl/certs/ca-certificates.crt"
                    verifyPeer = true
                    port = 443
                    timeout = 30.seconds
                }
            }
        }
        
        // Certificate management
        certificates {
            enabled = true
            
            // CA configuration
            ca {
                certificates = listOf(
                    "/etc/ssl/certs/horizonos-ca.pem",
                    "/etc/ssl/certs/internal-ca.pem",
                    "/etc/ssl/certs/intermediate-ca.pem"
                )
                trustStore = "/etc/ssl/certs/ca-certificates.crt"
                updateCommand = "update-ca-certificates"
                
                validation {
                    enabled = true
                    checkRevocation = true
                    ocspStapling = true
                    crlCheck = true
                    allowSelfSigned = false
                    maxChainLength = 5
                    requireBasicConstraints = true
                    requireKeyUsage = true
                }
            }
            
            // Client certificates
            client {
                certificates = listOf(
                    CertificateInfo(
                        name = "admin-client",
                        certFile = "/etc/ssl/certs/admin-client.pem",
                        keyFile = "/etc/ssl/private/admin-client-key.pem",
                        purpose = CertificatePurpose.CLIENT_AUTH,
                        keyUsage = listOf("digitalSignature", "keyEncipherment"),
                        extendedKeyUsage = listOf("clientAuth")
                    ),
                    CertificateInfo(
                        name = "web-server",
                        certFile = "/etc/ssl/certs/web-server.pem",
                        keyFile = "/etc/ssl/private/web-server-key.pem",
                        purpose = CertificatePurpose.SERVER_AUTH,
                        keyUsage = listOf("digitalSignature", "keyEncipherment"),
                        extendedKeyUsage = listOf("serverAuth"),
                        subjectAltNames = listOf("DNS:horizonos.local", "DNS:*.horizonos.local", "IP:192.168.1.100")
                    ),
                    CertificateInfo(
                        name = "email-signing",
                        certFile = "/etc/ssl/certs/email.pem",
                        keyFile = "/etc/ssl/private/email-key.pem",
                        purpose = CertificatePurpose.EMAIL_PROTECTION,
                        keyUsage = listOf("digitalSignature", "nonRepudiation"),
                        extendedKeyUsage = listOf("emailProtection")
                    )
                )
                
                // Automatic renewal
                autoRenewal {
                    enabled = true
                    checkInterval = 6.hours
                    renewalThreshold = 30.days
                    notifyBeforeExpiry = 7.days
                    gracePeriod = 1.days
                    
                    acme {
                        enabled = true
                        server = "https://acme-v02.api.letsencrypt.org/directory"
                        email = "security@horizonos.local"
                        keyType = ACMEKeyType.ECDSA256
                        reuseKey = true
                        
                        challenges {
                            http01 {
                                enabled = true
                                webroot = "/var/www/acme-challenge"
                                port = 80
                                timeout = 60.seconds
                            }
                            
                            dns01 {
                                enabled = false
                                provider = "cloudflare"
                                credentials = "/etc/acme/cloudflare.conf"
                                propagationTimeout = 120.seconds
                            }
                            
                            tls01 {
                                enabled = false
                                port = 443
                                timeout = 60.seconds
                            }
                        }
                        
                        rateLimits {
                            respectLimits = true
                            maxAttempts = 5
                            backoffFactor = 2.0
                            maxBackoff = 1.hours
                        }
                    }
                }
            }
            
            // Certificate management
            management {
                autoGenerate = true
                keyStrength = 4096
                hashAlgorithm = CertHashAlgorithm.SHA256
                validityPeriod = 365.days
                
                monitoring {
                    enabled = true
                    checkInterval = 6.hours
                    warnDays = 30
                    criticalDays = 7
                    checkChain = true
                    checkOcsp = true
                    
                    notifications {
                        email = true
                        emailAddress = "security@horizonos.local"
                        syslog = true
                        webhook = "https://hooks.slack.com/services/YOUR/SECURITY/WEBHOOK"
                        desktop = true
                    }
                }
                
                backup {
                    enabled = true
                    location = "/etc/ssl/backups"
                    encryption = true
                    retention = 365.days
                    schedule = "0 3 * * *"
                    compressionLevel = 6
                    
                    remote {
                        enabled = false
                        destination = "backup.horizonos.local:/srv/ssl-backups"
                        method = BackupMethod.RSYNC
                        encryption = true
                    }
                }
            }
        }
    }
    
    // Security-focused user configuration
    users {
        user("admin") {
            shell = "/usr/bin/zsh"
            groups("wheel", "ssh-users", "docker", "systemd-journal")
        }
        
        user("security") {
            shell = "/usr/bin/bash"
            groups("audit", "ssh-users")
        }
        
        user("developer") {
            shell = "/usr/bin/fish"
            groups("developers", "docker", "systemd-journal")
        }
        
        user("backup") {
            shell = "/usr/bin/bash"
            groups("backup")
        }
    }
    
    // Security-related packages
    packages {
        install(
            // Authentication and authorization
            "pam", "sudo", "polkit",
            
            // SSH and networking
            "openssh", "fail2ban", "denyhosts",
            
            // Firewall and network security
            "iptables", "firewalld", "nftables", "ipset",
            
            // SELinux/AppArmor
            "selinux-policy-targeted", "setools", "policycoreutils",
            "apparmor", "apparmor-utils", "apparmor-profiles",
            
            // Encryption and PKI
            "gnupg", "openssl", "gnutls-utils",
            "cryptsetup", "tpm2-tools", "clevis",
            
            // Audit and monitoring
            "auditd", "aide", "tripwire", "samhain",
            "logwatch", "logrotate", "rsyslog",
            
            // Intrusion detection
            "ossec-hids", "suricata", "snort",
            
            // Certificate management
            "certbot", "acme-tiny", "dehydrated",
            
            // Security scanning
            "lynis", "chkrootkit", "rkhunter", "clamav",
            
            // Network security tools
            "nmap", "netcat", "tcpdump", "wireshark-cli",
            
            // System hardening
            "hardening-check", "tiger", "bastille"
        )
    }
    
    // Security-related services
    services {
        enable("sshd") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("fail2ban") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("firewalld") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("auditd") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("gpg-agent") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("tpm2-abrmd") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("aide") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("ossec") {
            autoRestart = true
            restartOnFailure = true
        }
        
        enable("suricata") {
            autoRestart = true
            restartOnFailure = true
        }
        
        disable("telnet") {}
        disable("rsh") {}
        disable("rlogin") {}
        disable("ftp") {}
        disable("tftp") {}
    }
}