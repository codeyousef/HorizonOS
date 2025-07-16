package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.collections.shouldNotContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.days

class SecurityTest : StringSpec({
    
    "should create basic security configuration" {
        val config = horizonOS {
            hostname = "security-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                enabled = true
                
                pam {
                    enabled = true
                    failDelay = 3.seconds
                    maxTries = 3
                    lockoutTime = 15.minutes
                }
                
                ssh {
                    enabled = true
                    port = 22
                    permitRootLogin = PermitRootLogin.NO
                    passwordAuthentication = false
                    pubkeyAuthentication = true
                }
                
                sudo {
                    enabled = true
                    
                    rule {
                        users("admin")
                        commands("ALL")
                        options(SudoOption.NOPASSWD)
                    }
                }
            }
        }
        
        config.security shouldNotBe null
        config.security!!.enabled.shouldBeTrue()
        
        val pam = config.security!!.pam
        pam.enabled.shouldBeTrue()
        pam.failDelay shouldBe 3.seconds
        pam.maxTries shouldBe 3
        pam.lockoutTime shouldBe 15.minutes
        
        val ssh = config.security!!.ssh
        ssh.enabled.shouldBeTrue()
        ssh.port shouldBe 22
        ssh.permitRootLogin shouldBe PermitRootLogin.NO
        ssh.passwordAuthentication.shouldBeFalse()
        ssh.pubkeyAuthentication.shouldBeTrue()
        
        val sudo = config.security!!.sudo
        sudo.enabled.shouldBeTrue()
        sudo.rules shouldHaveSize 1
        sudo.rules[0].users shouldContain "admin"
        sudo.rules[0].commands shouldContain "ALL"
        sudo.rules[0].options shouldContain SudoOption.NOPASSWD
    }
    
    "should configure PAM authentication" {
        val config = horizonOS {
            hostname = "pam-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                pam {
                    enabled = true
                    failDelay = 5.seconds
                    maxTries = 5
                    lockoutTime = 30.minutes
                    
                    password {
                        minLength = 12
                        requireDigits = true
                        requireLowercase = true
                        requireUppercase = true
                        requireSpecialChars = true
                        maxSequential = 3
                        history = 5
                        
                        dictionary {
                            enabled = true
                            customWords = listOf("company", "password")
                        }
                        
                        complexity {
                            enabled = true
                            minClasses = 3
                            maxRepeat = 2
                            rejectUsername = true
                            enforceForRoot = true
                        }
                    }
                    
                    account {
                        lockout {
                            enabled = true
                            denyRetries = 3
                            unlockTime = 60.minutes
                            evenDenyRoot = false
                        }
                        
                        time {
                            enabled = true
                            restrictions = listOf(
                                TimeRestriction("user1", listOf("0800-1800"), listOf("Mo", "Tu", "We", "Th", "Fr")),
                                TimeRestriction("admin", listOf("0000-2359"), listOf("Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"))
                            )
                        }
                    }
                    
                    session {
                        limits {
                            enabled = true
                            rules = listOf(
                                LimitRule("@users", LimitType.NPROC, LimitItem.HARD, 100),
                                LimitRule("admin", LimitType.NOFILE, LimitItem.SOFT, 65536)
                            )
                        }
                        
                        umask {
                            enabled = true
                            default = "0022"
                            userMask = "0077"
                        }
                    }
                }
            }
        }
        
        val pam = config.security!!.pam
        pam.enabled.shouldBeTrue()
        pam.failDelay shouldBe 5.seconds
        pam.maxTries shouldBe 5
        pam.lockoutTime shouldBe 30.minutes
        
        val password = pam.password
        password.minLength shouldBe 12
        password.requireDigits.shouldBeTrue()
        password.requireLowercase.shouldBeTrue()
        password.requireUppercase.shouldBeTrue()
        password.requireSpecialChars.shouldBeTrue()
        password.maxSequential shouldBe 3
        password.history shouldBe 5
        
        password.dictionary.enabled.shouldBeTrue()
        password.dictionary.customWords shouldContain "company"
        password.dictionary.customWords shouldContain "password"
        
        password.complexity.enabled.shouldBeTrue()
        password.complexity.minClasses shouldBe 3
        password.complexity.maxRepeat shouldBe 2
        password.complexity.rejectUsername.shouldBeTrue()
        password.complexity.enforceForRoot.shouldBeTrue()
        
        val account = pam.account
        account.lockout.enabled.shouldBeTrue()
        account.lockout.denyRetries shouldBe 3
        account.lockout.unlockTime shouldBe 60.minutes
        account.lockout.evenDenyRoot.shouldBeFalse()
        
        account.time.enabled.shouldBeTrue()
        account.time.restrictions shouldHaveSize 2
        account.time.restrictions[0].user shouldBe "user1"
        account.time.restrictions[1].user shouldBe "admin"
        
        val session = pam.session
        session.limits.enabled.shouldBeTrue()
        session.limits.rules shouldHaveSize 2
        session.limits.rules[0].domain shouldBe "@users"
        session.limits.rules[0].type shouldBe LimitType.NPROC
        session.limits.rules[1].domain shouldBe "admin"
        session.limits.rules[1].type shouldBe LimitType.NOFILE
        
        session.umask.enabled.shouldBeTrue()
        session.umask.default shouldBe "0022"
        session.umask.userMask shouldBe "0077"
    }
    
    "should configure SSH security" {
        val config = horizonOS {
            hostname = "ssh-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                ssh {
                    enabled = true
                    port = 2222
                    permitRootLogin = PermitRootLogin.PROHIBIT_PASSWORD
                    passwordAuthentication = false
                    pubkeyAuthentication = true
                    challengeResponseAuthentication = false
                    kbdInteractiveAuthentication = false
                    x11Forwarding = false
                    
                    protocol {
                        version = "2"
                        ciphers = listOf("chacha20-poly1305@openssh.com", "aes256-gcm@openssh.com", "aes128-gcm@openssh.com")
                        macs = listOf("hmac-sha2-256-etm@openssh.com", "hmac-sha2-512-etm@openssh.com")
                        kexAlgorithms = listOf("curve25519-sha256@libssh.org", "diffie-hellman-group16-sha512")
                        hostKeyAlgorithms = listOf("ssh-ed25519", "rsa-sha2-512", "rsa-sha2-256")
                    }
                    
                    access {
                        allowUsers = listOf("admin", "developer")
                        denyUsers = listOf("guest")
                        allowGroups = listOf("ssh-users", "admins")
                        denyGroups = listOf("restricted")
                        maxAuthTries = 3
                        maxSessions = 10
                        maxStartups = "10:30:100"
                        loginGraceTime = 2.minutes
                    }
                    
                    security {
                        strictModes = true
                        ignoredRhosts = true
                        hostbasedAuthentication = false
                        emptyPasswords = false
                        permitUserEnvironment = false
                        compression = SSHCompression.DELAYED
                        tcpKeepAlive = true
                        clientAliveInterval = 5.minutes
                        clientAliveCountMax = 3
                        useDNS = false
                        
                        rateLimit {
                            enabled = true
                            maxConnections = 5
                            timeWindow = 60.seconds
                            blockDuration = 10.minutes
                        }
                        
                        geoBlocking {
                            enabled = true
                            allowedCountries = listOf("US", "CA", "GB")
                            blockedCountries = listOf("CN", "RU", "KP")
                            whitelistIPs = listOf("192.168.1.0/24", "10.0.0.0/8")
                        }
                    }
                    
                    keyManagement {
                        autoRotation = true
                        rotationInterval = 90.days
                        keyStrength = 4096
                        hostKeyPath = "/etc/ssh"
                        
                        authorizedKeys {
                            strictMode = true
                            maxKeys = 10
                            keyTypes = listOf(KeyType.ED25519, KeyType.RSA, KeyType.ECDSA)
                            expiration = 365.days
                        }
                    }
                    
                    logging {
                        enabled = true
                        level = SSHLogLevel.INFO
                        facility = SSHLogFacility.AUTH
                        verboseMode = false
                        
                        audit {
                            enabled = true
                            logConnections = true
                            logCommands = true
                            logSFTP = true
                            compressionLevel = 6
                        }
                    }
                }
            }
        }
        
        val ssh = config.security!!.ssh
        ssh.enabled.shouldBeTrue()
        ssh.port shouldBe 2222
        ssh.permitRootLogin shouldBe PermitRootLogin.PROHIBIT_PASSWORD
        ssh.passwordAuthentication.shouldBeFalse()
        ssh.pubkeyAuthentication.shouldBeTrue()
        ssh.x11Forwarding.shouldBeFalse()
        
        val protocol = ssh.protocol
        protocol.version shouldBe "2"
        protocol.ciphers shouldContain "chacha20-poly1305@openssh.com"
        protocol.macs shouldContain "hmac-sha2-256-etm@openssh.com"
        protocol.kexAlgorithms shouldContain "curve25519-sha256@libssh.org"
        
        val access = ssh.access
        access.allowUsers shouldContain "admin"
        access.denyUsers shouldContain "guest"
        access.allowGroups shouldContain "ssh-users"
        access.denyGroups shouldContain "restricted"
        access.maxAuthTries shouldBe 3
        access.maxSessions shouldBe 10
        access.loginGraceTime shouldBe 2.minutes
        
        val security = ssh.security
        security.strictModes.shouldBeTrue()
        security.hostbasedAuthentication.shouldBeFalse()
        security.emptyPasswords.shouldBeFalse()
        security.compression shouldBe SSHCompression.DELAYED
        security.clientAliveInterval shouldBe 5.minutes
        security.clientAliveCountMax shouldBe 3
        
        security.rateLimit.enabled.shouldBeTrue()
        security.rateLimit.maxConnections shouldBe 5
        security.rateLimit.timeWindow shouldBe 60.seconds
        
        security.geoBlocking.enabled.shouldBeTrue()
        security.geoBlocking.allowedCountries shouldContain "US"
        security.geoBlocking.blockedCountries shouldContain "CN"
        
        val keyMgmt = ssh.keyManagement
        keyMgmt.autoRotation.shouldBeTrue()
        keyMgmt.rotationInterval shouldBe 90.days
        keyMgmt.keyStrength shouldBe 4096
        
        keyMgmt.authorizedKeys.strictMode.shouldBeTrue()
        keyMgmt.authorizedKeys.maxKeys shouldBe 10
        keyMgmt.authorizedKeys.keyTypes shouldContain KeyType.ED25519
        
        val logging = ssh.logging
        logging.enabled.shouldBeTrue()
        logging.level shouldBe SSHLogLevel.INFO
        logging.facility shouldBe SSHLogFacility.AUTH
        
        logging.audit.enabled.shouldBeTrue()
        logging.audit.logConnections.shouldBeTrue()
        logging.audit.logCommands.shouldBeTrue()
    }
    
    "should configure sudo rules and policies" {
        val config = horizonOS {
            hostname = "sudo-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                sudo {
                    enabled = true
                    
                    rule {
                        users("admin")
                        commands("ALL")
                        options(SudoOption.NOPASSWD)
                    }
                    
                    rule {
                        groups("wheel")
                        commands("/usr/bin/systemctl", "/usr/bin/journalctl")
                        hosts("localhost")
                        runAs("root")
                        options(SudoOption.PASSWD, SudoOption.LOG_INPUT, SudoOption.LOG_OUTPUT)
                    }
                    
                    rule {
                        users("developer")
                        commands("/usr/bin/docker", "/usr/bin/podman", "/usr/bin/kubectl")
                        noPassword = false
                        timeout = 15.minutes
                    }
                    
                    defaults {
                        targetPw = false
                        rootPw = false
                        runAsNonRoot = false
                        preserveEnv = true
                        requireTty = true
                        visiblepw = false
                        pwFeedback = false
                        fastGlob = true
                        insults = false
                        
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
                            mailto = "admin@localhost"
                        }
                        
                        environment {
                            resetEnv = true
                            keepEnv = listOf("LANG", "LC_*", "TERM", "PATH")
                            checkEnv = listOf("HOME", "USER", "LOGNAME")
                            setEnv = mapOf("EDITOR" to "vim", "PAGER" to "less")
                        }
                        
                        logging {
                            enabled = true
                            logFile = "/var/log/sudo.log"
                            logHost = true
                            logYear = true
                            logInput = false
                            logOutput = false
                            maxLogSize = "10M"
                            
                            syslog {
                                enabled = true
                                facility = SyslogFacility.AUTHPRIV
                                priority = SyslogPriority.NOTICE
                                maxLength = 2048
                            }
                        }
                        
                        timeout {
                            passwd = 5.minutes
                            timestamp = 15.minutes
                            lectureFile = "/etc/sudo_lecture"
                            lecture = SudoLecture.ONCE
                        }
                    }
                }
            }
        }
        
        val sudo = config.security!!.sudo
        sudo.enabled.shouldBeTrue()
        sudo.rules shouldHaveSize 3
        
        val adminRule = sudo.rules[0]
        adminRule.users shouldContain "admin"
        adminRule.commands shouldContain "ALL"
        adminRule.options shouldContain SudoOption.NOPASSWD
        
        val wheelRule = sudo.rules[1]
        wheelRule.groups shouldContain "wheel"
        wheelRule.commands shouldContain "/usr/bin/systemctl"
        wheelRule.hosts shouldContain "localhost"
        wheelRule.runAs shouldBe "root"
        wheelRule.options shouldContain SudoOption.PASSWD
        wheelRule.options shouldContain SudoOption.LOG_INPUT
        
        val devRule = sudo.rules[2]
        devRule.users shouldContain "developer"
        devRule.commands shouldContain "/usr/bin/docker"
        devRule.noPassword.shouldBeFalse()
        devRule.timeout shouldBe 15.minutes
        
        val defaults = sudo.defaults
        defaults.targetPw.shouldBeFalse()
        defaults.preserveEnv.shouldBeTrue()
        defaults.requireTty.shouldBeTrue()
        defaults.insults.shouldBeFalse()
        
        val security = defaults.security
        security.usesPty.shouldBeTrue()
        security.closeSessions.shouldBeTrue()
        security.pam.shouldBeTrue()
        security.mailBadpass.shouldBeTrue()
        security.mailto shouldBe "admin@localhost"
        
        val env = defaults.environment
        env.resetEnv.shouldBeTrue()
        env.keepEnv shouldContain "LANG"
        env.checkEnv shouldContain "HOME"
        env.setEnv["EDITOR"] shouldBe "vim"
        
        val logging = defaults.logging
        logging.enabled.shouldBeTrue()
        logging.logFile shouldBe "/var/log/sudo.log"
        logging.logHost.shouldBeTrue()
        logging.syslog.enabled.shouldBeTrue()
        logging.syslog.facility shouldBe SyslogFacility.AUTHPRIV
        
        val timeout = defaults.timeout
        timeout.passwd shouldBe 5.minutes
        timeout.timestamp shouldBe 15.minutes
        timeout.lecture shouldBe SudoLecture.ONCE
    }
    
    "should configure SELinux policies" {
        val config = horizonOS {
            hostname = "selinux-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                selinux {
                    enabled = true
                    mode = SELinuxMode.ENFORCING
                    policy = SELinuxPolicy.TARGETED
                    
                    booleans {
                        set("httpd_can_network_connect", true)
                        set("samba_enable_home_dirs", false)
                        set("ftpd_anon_write", false)
                    }
                    
                    modules {
                        install("mymodule.pp")
                        enable("apache")
                        disable("games")
                    }
                    
                    ports {
                        add(8080, SELinuxPortType.HTTP_PORT_T, SELinuxProtocol.TCP)
                        add(9443, SELinuxPortType.HTTP_PORT_T, SELinuxProtocol.TCP)
                    }
                    
                    fileContexts {
                        add("/opt/myapp(/.*)?", "httpd_exec_t")
                        add("/var/log/myapp(/.*)?", "httpd_log_t")
                    }
                    
                    users {
                        map("admin", "unconfined_u")
                        map("webuser", "user_u")
                    }
                    
                    logging {
                        enabled = true
                        auditDenials = true
                        verboseLogging = false
                    }
                }
            }
        }
        
        val selinux = config.security!!.selinux
        selinux.enabled.shouldBeTrue()
        selinux.mode shouldBe SELinuxMode.ENFORCING
        selinux.policy shouldBe SELinuxPolicy.TARGETED
        
        selinux.booleans["httpd_can_network_connect"] shouldBe true
        selinux.booleans["samba_enable_home_dirs"] shouldBe false
        selinux.booleans["ftpd_anon_write"] shouldBe false
        
        selinux.modules.install shouldContain "mymodule.pp"
        selinux.modules.enable shouldContain "apache"
        selinux.modules.disable shouldContain "games"
        
        selinux.ports shouldHaveSize 2
        selinux.ports[0].port shouldBe 8080
        selinux.ports[0].type shouldBe SELinuxPortType.HTTP_PORT_T
        selinux.ports[0].protocol shouldBe SELinuxProtocol.TCP
        
        selinux.fileContexts shouldHaveSize 2
        selinux.fileContexts[0].path shouldBe "/opt/myapp(/.*)?"
        selinux.fileContexts[0].context shouldBe "httpd_exec_t"
        
        selinux.users["admin"] shouldBe "unconfined_u"
        selinux.users["webuser"] shouldBe "user_u"
        
        selinux.logging.enabled.shouldBeTrue()
        selinux.logging.auditDenials.shouldBeTrue()
        selinux.logging.verboseLogging.shouldBeFalse()
    }
    
    "should configure AppArmor profiles" {
        val config = horizonOS {
            hostname = "apparmor-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                apparmor {
                    enabled = true
                    
                    profiles {
                        profile("/usr/bin/firefox") {
                            mode = AppArmorMode.ENFORCE
                            
                            capabilities {
                                allow("net_bind_service")
                                deny("sys_admin")
                            }
                            
                            files {
                                rule("/home/**/Downloads/**", "rw")
                                rule("/tmp/**", "rw")
                                rule("/usr/lib/firefox/**", "r")
                                rule("/etc/passwd", "r")
                                deny("/etc/shadow")
                                deny("/root/**")
                            }
                            
                            network {
                                allow(NetworkFamily.INET, SocketType.STREAM)
                                allow(NetworkFamily.INET6, SocketType.STREAM)
                                deny(NetworkFamily.UNIX, SocketType.DGRAM)
                            }
                            
                            dbus {
                                rule(DbusType.SESSION, "org.freedesktop.Notifications", "Notify")
                                rule(DbusType.SYSTEM, "org.freedesktop.NetworkManager", "*")
                            }
                        }
                        
                        profile("/usr/bin/myapp") {
                            mode = AppArmorMode.COMPLAIN
                            include("/etc/apparmor.d/abstractions/base")
                            include("/etc/apparmor.d/abstractions/nameservice")
                        }
                    }
                    
                    tunables {
                        set("@{HOME}", "/home/*")
                        set("@{PROC}", "/proc/*")
                    }
                    
                    logging {
                        enabled = true
                        auditMode = AppArmorAuditMode.ALL
                        rate = 10
                    }
                }
            }
        }
        
        val apparmor = config.security!!.apparmor
        apparmor.enabled.shouldBeTrue()
        
        val profiles = apparmor.profiles
        profiles shouldHaveSize 2
        
        val firefoxProfile = profiles[0]
        firefoxProfile.executable shouldBe "/usr/bin/firefox"
        firefoxProfile.mode shouldBe AppArmorMode.ENFORCE
        
        firefoxProfile.capabilities.allow shouldContain "net_bind_service"
        firefoxProfile.capabilities.deny shouldContain "sys_admin"
        
        firefoxProfile.files.rules shouldHaveSize 4
        firefoxProfile.files.deny shouldHaveSize 2
        firefoxProfile.files.rules[0].path shouldBe "/home/**/Downloads/**"
        firefoxProfile.files.rules[0].permissions shouldBe "rw"
        firefoxProfile.files.deny shouldContain "/etc/shadow"
        
        firefoxProfile.network.allow shouldHaveSize 2
        firefoxProfile.network.deny shouldHaveSize 1
        
        firefoxProfile.dbus.rules shouldHaveSize 2
        firefoxProfile.dbus.rules[0].busType shouldBe DbusType.SESSION
        firefoxProfile.dbus.rules[0].name shouldBe "org.freedesktop.Notifications"
        
        val myappProfile = profiles[1]
        myappProfile.executable shouldBe "/usr/bin/myapp"
        myappProfile.mode shouldBe AppArmorMode.COMPLAIN
        myappProfile.include shouldContain "/etc/apparmor.d/abstractions/base"
        
        apparmor.tunables["@{HOME}"] shouldBe "/home/*"
        apparmor.tunables["@{PROC}"] shouldBe "/proc/*"
        
        apparmor.logging.enabled.shouldBeTrue()
        apparmor.logging.auditMode shouldBe AppArmorAuditMode.ALL
        apparmor.logging.rate shouldBe 10
    }
    
    "should configure firewall rules" {
        val config = horizonOS {
            hostname = "firewall-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                firewall {
                    enabled = true
                    backend = FirewallBackend.FIREWALLD
                    defaultZone = "public"
                    
                    zones {
                        zone("public") {
                            target = ZoneTarget.DEFAULT
                            description = "Public zone for untrusted networks"
                            
                            services("ssh", "http", "https")
                            ports("8080/tcp", "9090/udp")
                            
                            interfaces("eth0", "wlan0")
                            sources("192.168.1.0/24")
                            
                            rich {
                                rule {
                                    source("192.168.1.100")
                                    service("ssh")
                                    action = RichRuleAction.ACCEPT
                                }
                                
                                rule {
                                    source("10.0.0.0/8")
                                    port("22", "tcp")
                                    action = RichRuleAction.DROP
                                    log {
                                        prefix = "SSH-DROP"
                                        level = LogLevel.INFO
                                    }
                                }
                            }
                            
                            masquerade = false
                            icmpBlocks("echo-request", "timestamp-request")
                        }
                        
                        zone("dmz") {
                            target = ZoneTarget.DEFAULT
                            description = "DMZ zone for servers"
                            
                            services("http", "https")
                            ports("8443/tcp")
                            
                            forwardPorts {
                                forward(80, "tcp", 8080, "192.168.1.100")
                                forward(443, "tcp", 8443, "192.168.1.100")
                            }
                        }
                    }
                    
                    ipsets {
                        ipset("blacklist") {
                            type = IpsetType.HASH_IP
                            family = AddressFamily.INET
                            entries("192.168.0.1", "10.0.0.1")
                        }
                        
                        ipset("whitelist") {
                            type = IpsetType.HASH_NET
                            family = AddressFamily.INET
                            entries("192.168.1.0/24", "10.0.0.0/8")
                        }
                    }
                    
                    iptables {
                        enabled = true
                        policy = IptablesPolicy.DROP
                        
                        rules {
                            rule {
                                chain = Chain.INPUT
                                protocol = "tcp"
                                destinationPort = "22"
                                source = "192.168.1.0/24"
                                target = Target.ACCEPT
                            }
                            
                            rule {
                                chain = Chain.OUTPUT
                                protocol = "udp"
                                destinationPort = "53"
                                target = Target.ACCEPT
                            }
                            
                            rule {
                                chain = Chain.FORWARD
                                inputInterface = "eth0"
                                outputInterface = "eth1"
                                target = Target.ACCEPT
                            }
                        }
                    }
                    
                    portKnocking {
                        enabled = true
                        
                        sequence("ssh") {
                            ports(1234, 5678, 9012)
                            protocol = "tcp"
                            timeout = 30.seconds
                            openPort = 22
                            openDuration = 10.minutes
                        }
                    }
                    
                    ddosProtection {
                        enabled = true
                        connectionLimit = 100
                        rateLimit = "10/second"
                        burstLimit = 20
                        
                        synFlood {
                            enabled = true
                            rate = "1/second"
                            burst = 3
                        }
                    }
                }
            }
        }
        
        val firewall = config.security!!.firewall
        firewall.enabled.shouldBeTrue()
        firewall.backend shouldBe FirewallBackend.FIREWALLD
        firewall.defaultZone shouldBe "public"
        
        val zones = firewall.zones
        zones shouldHaveSize 2
        
        val publicZone = zones[0]
        publicZone.name shouldBe "public"
        publicZone.target shouldBe ZoneTarget.DEFAULT
        publicZone.services shouldContain "ssh"
        publicZone.services shouldContain "http"
        publicZone.ports shouldContain "8080/tcp"
        publicZone.interfaces shouldContain "eth0"
        publicZone.sources shouldContain "192.168.1.0/24"
        publicZone.masquerade.shouldBeFalse()
        publicZone.icmpBlocks shouldContain "echo-request"
        
        publicZone.richRules shouldHaveSize 2
        val rule1 = publicZone.richRules[0]
        rule1.source shouldBe "192.168.1.100"
        rule1.service shouldBe "ssh"
        rule1.action shouldBe RichRuleAction.ACCEPT
        
        val rule2 = publicZone.richRules[1]
        rule2.source shouldBe "10.0.0.0/8"
        rule2.port shouldBe "22"
        rule2.protocol shouldBe "tcp"
        rule2.action shouldBe RichRuleAction.DROP
        rule2.log!!.prefix shouldBe "SSH-DROP"
        rule2.log!!.level shouldBe LogLevel.INFO
        
        val dmzZone = zones[1]
        dmzZone.name shouldBe "dmz"
        dmzZone.forwardPorts shouldHaveSize 2
        dmzZone.forwardPorts[0].port shouldBe 80
        dmzZone.forwardPorts[0].toAddr shouldBe "192.168.1.100"
        
        val ipsets = firewall.ipsets
        ipsets shouldHaveSize 2
        ipsets[0].name shouldBe "blacklist"
        ipsets[0].type shouldBe IpsetType.HASH_IP
        ipsets[0].entries shouldContain "192.168.0.1"
        
        val iptables = firewall.iptables
        iptables.enabled.shouldBeTrue()
        iptables.policy shouldBe IptablesPolicy.DROP
        iptables.rules shouldHaveSize 3
        
        val inputRule = iptables.rules[0]
        inputRule.chain shouldBe Chain.INPUT
        inputRule.protocol shouldBe "tcp"
        inputRule.destinationPort shouldBe "22"
        inputRule.source shouldBe "192.168.1.0/24"
        inputRule.target shouldBe Target.ACCEPT
        
        val portKnocking = firewall.portKnocking
        portKnocking.enabled.shouldBeTrue()
        portKnocking.sequences shouldHaveSize 1
        
        val sshSequence = portKnocking.sequences[0]
        sshSequence.name shouldBe "ssh"
        sshSequence.ports shouldContain 1234
        sshSequence.protocol shouldBe "tcp"
        sshSequence.timeout shouldBe 30.seconds
        sshSequence.openPort shouldBe 22
        sshSequence.openDuration shouldBe 10.minutes
        
        val ddos = firewall.ddosProtection
        ddos.enabled.shouldBeTrue()
        ddos.connectionLimit shouldBe 100
        ddos.rateLimit shouldBe "10/second"
        ddos.burstLimit shouldBe 20
        
        ddos.synFlood.enabled.shouldBeTrue()
        ddos.synFlood.rate shouldBe "1/second"
        ddos.synFlood.burst shouldBe 3
    }
    
    "should configure TPM security" {
        val config = horizonOS {
            hostname = "tpm-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                tpm {
                    enabled = true
                    version = TPMVersion.TPM2
                    
                    hierarchy {
                        owner = TPMHierarchy.ENABLED
                        endorsement = TPMHierarchy.ENABLED
                        platform = TPMHierarchy.ENABLED
                        lockout = TPMHierarchy.DISABLED
                    }
                    
                    pcrs {
                        policy {
                            useFor = listOf(PCRUseCase.DISK_ENCRYPTION, PCRUseCase.SECURE_BOOT)
                            pcrBanks = listOf(0, 1, 2, 3, 4, 5, 6, 7, 8, 9)
                            hashAlgorithm = TPMHashAlgorithm.SHA256
                        }
                        
                        measurements {
                            bootMeasurements = true
                            kernelMeasurements = true
                            initrdMeasurements = true
                            applicationMeasurements = false
                        }
                        
                        attestation {
                            enabled = true
                            remoteVerification = false
                            quoteInterval = 1.hours
                            nonceSize = 32
                        }
                    }
                    
                    keys {
                        storageRootKey {
                            algorithm = TPMKeyAlgorithm.RSA2048
                            persistent = true
                            handle = 0x81000001
                        }
                        
                        endorsementKey {
                            algorithm = TPMKeyAlgorithm.RSA2048
                            persistent = true
                            certificate = true
                        }
                        
                        attestationKey {
                            algorithm = TPMKeyAlgorithm.ECC_NIST_P256
                            restricted = true
                            signOnly = true
                        }
                    }
                    
                    ima {
                        enabled = true
                        template = IMATemplate.IMA_NG
                        hashAlgorithm = TPMHashAlgorithm.SHA256
                        
                        policy {
                            measureBootAggregate = true
                            measureExecutables = true
                            measureLibraries = true
                            measureModules = true
                            measureFirmware = false
                            
                            rules = listOf(
                                IMARule(IMAAction.MEASURE, "/usr/bin/**"),
                                IMARule(IMAAction.MEASURE, "/usr/lib/**"),
                                IMARule(IMAAction.APPRAISE, "/etc/**")
                            )
                        }
                        
                        keyring {
                            name = ".ima"
                            certificates = listOf("/etc/keys/ima-cert.pem")
                        }
                    }
                    
                    clevis {
                        enabled = true
                        
                        tpm2 {
                            enabled = true
                            keyFile = "/etc/clevis/tpm2.key"
                            pcrBank = TPMHashAlgorithm.SHA256
                            pcrIds = listOf(0, 1, 2, 3, 4, 5, 6, 7)
                        }
                        
                        tang {
                            enabled = false
                            servers = listOf("tang.example.com:7500")
                            threshold = 1
                        }
                    }
                }
            }
        }
        
        val tpm = config.security!!.tpm
        tpm.enabled.shouldBeTrue()
        tpm.version shouldBe TPMVersion.TPM2
        
        val hierarchy = tpm.hierarchy
        hierarchy.owner shouldBe TPMHierarchy.ENABLED
        hierarchy.endorsement shouldBe TPMHierarchy.ENABLED
        hierarchy.platform shouldBe TPMHierarchy.ENABLED
        hierarchy.lockout shouldBe TPMHierarchy.DISABLED
        
        val pcrs = tpm.pcrs
        pcrs.policy.useFor shouldContain PCRUseCase.DISK_ENCRYPTION
        pcrs.policy.pcrBanks shouldContain 0
        pcrs.policy.hashAlgorithm shouldBe TPMHashAlgorithm.SHA256
        
        pcrs.measurements.bootMeasurements.shouldBeTrue()
        pcrs.measurements.kernelMeasurements.shouldBeTrue()
        pcrs.measurements.initrdMeasurements.shouldBeTrue()
        pcrs.measurements.applicationMeasurements.shouldBeFalse()
        
        pcrs.attestation.enabled.shouldBeTrue()
        pcrs.attestation.remoteVerification.shouldBeFalse()
        pcrs.attestation.quoteInterval shouldBe 1.hours
        
        val keys = tpm.keys
        keys.storageRootKey.algorithm shouldBe TPMKeyAlgorithm.RSA2048
        keys.storageRootKey.persistent.shouldBeTrue()
        keys.storageRootKey.handle shouldBe 0x81000001
        
        keys.endorsementKey.algorithm shouldBe TPMKeyAlgorithm.RSA2048
        keys.endorsementKey.certificate.shouldBeTrue()
        
        keys.attestationKey.algorithm shouldBe TPMKeyAlgorithm.ECC_NIST_P256
        keys.attestationKey.restricted.shouldBeTrue()
        keys.attestationKey.signOnly.shouldBeTrue()
        
        val ima = tpm.ima
        ima.enabled.shouldBeTrue()
        ima.template shouldBe IMATemplate.IMA_NG
        ima.hashAlgorithm shouldBe TPMHashAlgorithm.SHA256
        
        ima.policy.measureBootAggregate.shouldBeTrue()
        ima.policy.measureExecutables.shouldBeTrue()
        ima.policy.rules shouldHaveSize 3
        ima.policy.rules[0].action shouldBe IMAAction.MEASURE
        ima.policy.rules[0].path shouldBe "/usr/bin/**"
        
        ima.keyring.name shouldBe ".ima"
        ima.keyring.certificates shouldContain "/etc/keys/ima-cert.pem"
        
        val clevis = tpm.clevis
        clevis.enabled.shouldBeTrue()
        
        clevis.tpm2.enabled.shouldBeTrue()
        clevis.tpm2.keyFile shouldBe "/etc/clevis/tpm2.key"
        clevis.tpm2.pcrBank shouldBe TPMHashAlgorithm.SHA256
        clevis.tpm2.pcrIds shouldContain 0
        
        clevis.tang.enabled.shouldBeFalse()
        clevis.tang.servers shouldContain "tang.example.com:7500"
        clevis.tang.threshold shouldBe 1
    }
    
    "should configure GPG key management" {
        val config = horizonOS {
            hostname = "gpg-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                gpg {
                    enabled = true
                    
                    keys {
                        generate {
                            keyType = GPGKeyType.RSA
                            keyLength = 4096
                            subkeys = true
                            subkeyType = GPGKeyType.RSA
                            subkeyLength = 4096
                            expiry = 2.years
                            
                            identity {
                                name = "HorizonOS Administrator"
                                email = "admin@horizonos.local"
                                comment = "System Administrator Key"
                            }
                            
                            preferences {
                                cipherAlgorithms = listOf("AES256", "AES192", "AES")
                                hashAlgorithms = listOf("SHA512", "SHA256", "SHA224")
                                compressionAlgorithms = listOf("ZLIB", "BZIP2", "ZIP")
                                features = listOf("MDC")
                            }
                        }
                        
                        import {
                            keyFiles = listOf("/etc/gpg/admin-key.asc", "/etc/gpg/backup-key.asc")
                            keyservers = listOf("hkps://keys.openpgp.org", "hkps://keyserver.ubuntu.com")
                            autoImport = true
                            verifySignatures = true
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
                            }
                        }
                    }
                    
                    agent {
                        enabled = true
                        defaultCacheTtl = 1.hours
                        maxCacheTtl = 8.hours
                        pinentryProgram = "/usr/bin/pinentry-qt"
                        
                        caching {
                            enableSshSupport = true
                            grabKeyboard = false
                            grabPointer = false
                            allowMarkTrusted = true
                            disableScdaemon = false
                        }
                        
                        security {
                            allowPresetPassphrase = false
                            allowLoopbackPinentry = false
                            enforcePassphraseConstraints = true
                            minPassphraseLen = 8
                            minPassphraseNonalpha = 1
                        }
                    }
                    
                    signing {
                        autoSign = false
                        defaultKey = "admin@horizonos.local"
                        
                        policies {
                            requireSignature = listOf("/etc/**", "/usr/bin/**")
                            trustedSigners = listOf("admin@horizonos.local", "security@horizonos.local")
                            verifySignatures = true
                            rejectUnsigned = false
                        }
                        
                        email {
                            enabled = true
                            autoEncrypt = true
                            autoSign = true
                            preferredKeyserver = "hkps://keys.openpgp.org"
                        }
                    }
                    
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
                        }
                        
                        hkps {
                            enabled = true
                            caFile = "/etc/ssl/certs/ca-certificates.crt"
                            verifyPeer = true
                            port = 443
                        }
                    }
                }
            }
        }
        
        val gpg = config.security!!.gpg
        gpg.enabled.shouldBeTrue()
        
        val keys = gpg.keys
        val generate = keys.generate
        generate.keyType shouldBe GPGKeyType.RSA
        generate.keyLength shouldBe 4096
        generate.subkeys.shouldBeTrue()
        generate.expiry shouldBe 2.years
        
        generate.identity.name shouldBe "HorizonOS Administrator"
        generate.identity.email shouldBe "admin@horizonos.local"
        generate.identity.comment shouldBe "System Administrator Key"
        
        generate.preferences.cipherAlgorithms shouldContain "AES256"
        generate.preferences.hashAlgorithms shouldContain "SHA512"
        generate.preferences.compressionAlgorithms shouldContain "ZLIB"
        
        val import = keys.import
        import.keyFiles shouldContain "/etc/gpg/admin-key.asc"
        import.keyservers shouldContain "hkps://keys.openpgp.org"
        import.autoImport.shouldBeTrue()
        import.verifySignatures.shouldBeTrue()
        
        val management = keys.management
        management.autoExpire.shouldBeTrue()
        management.renewalWarning shouldBe 30.days
        management.backupLocation shouldBe "/etc/gpg/backups"
        management.encryptBackups.shouldBeTrue()
        
        management.rotation.enabled.shouldBeTrue()
        management.rotation.interval shouldBe 1.years
        management.rotation.keepOldKeys shouldBe 3
        
        val agent = gpg.agent
        agent.enabled.shouldBeTrue()
        agent.defaultCacheTtl shouldBe 1.hours
        agent.maxCacheTtl shouldBe 8.hours
        agent.pinentryProgram shouldBe "/usr/bin/pinentry-qt"
        
        agent.caching.enableSshSupport.shouldBeTrue()
        agent.caching.grabKeyboard.shouldBeFalse()
        agent.caching.allowMarkTrusted.shouldBeTrue()
        
        agent.security.allowPresetPassphrase.shouldBeFalse()
        agent.security.enforcePassphraseConstraints.shouldBeTrue()
        agent.security.minPassphraseLen shouldBe 8
        
        val signing = gpg.signing
        signing.autoSign.shouldBeFalse()
        signing.defaultKey shouldBe "admin@horizonos.local"
        
        signing.policies.requireSignature shouldContain "/etc/**"
        signing.policies.trustedSigners shouldContain "admin@horizonos.local"
        signing.policies.verifySignatures.shouldBeTrue()
        
        signing.email.enabled.shouldBeTrue()
        signing.email.autoEncrypt.shouldBeTrue()
        signing.email.autoSign.shouldBeTrue()
        
        val keyserver = gpg.keyserver
        keyserver.enabled.shouldBeTrue()
        keyserver.defaultKeyserver shouldBe "hkps://keys.openpgp.org"
        keyserver.autoRetrieve.shouldBeTrue()
        keyserver.autoKeyImport.shouldBeFalse()
        
        keyserver.hkp.enabled.shouldBeTrue()
        keyserver.hkp.port shouldBe 11371
        keyserver.hkp.timeout shouldBe 30.seconds
        
        keyserver.hkps.enabled.shouldBeTrue()
        keyserver.hkps.verifyPeer.shouldBeTrue()
        keyserver.hkps.port shouldBe 443
    }
    
    "should configure certificates and PKI" {
        val config = horizonOS {
            hostname = "cert-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                certificates {
                    enabled = true
                    
                    ca {
                        certificates = listOf(
                            "/etc/ssl/certs/ca-cert.pem",
                            "/etc/ssl/certs/internal-ca.pem"
                        )
                        trustStore = "/etc/ssl/certs/ca-certificates.crt"
                        updateCommand = "update-ca-certificates"
                        
                        validation {
                            enabled = true
                            checkRevocation = true
                            ocspStapling = true
                            crlCheck = true
                            allowSelfSigned = false
                            maxChainLength = 10
                        }
                    }
                    
                    client {
                        certificates = listOf(
                            CertificateInfo(
                                name = "client-auth",
                                certFile = "/etc/ssl/certs/client.pem",
                                keyFile = "/etc/ssl/private/client-key.pem",
                                purpose = CertificatePurpose.CLIENT_AUTH
                            ),
                            CertificateInfo(
                                name = "web-server",
                                certFile = "/etc/ssl/certs/server.pem",
                                keyFile = "/etc/ssl/private/server-key.pem",
                                purpose = CertificatePurpose.SERVER_AUTH
                            )
                        )
                        
                        autoRenewal {
                            enabled = true
                            checkInterval = 1.days
                            renewalThreshold = 30.days
                            notifyBeforeExpiry = 7.days
                            
                            acme {
                                enabled = true
                                server = "https://acme-v02.api.letsencrypt.org/directory"
                                email = "admin@horizonos.local"
                                keyType = ACMEKeyType.RSA2048
                                
                                challenges {
                                    http01 {
                                        enabled = true
                                        webroot = "/var/www/html"
                                        port = 80
                                    }
                                    
                                    dns01 {
                                        enabled = false
                                        provider = "cloudflare"
                                        credentials = "/etc/acme/cloudflare.conf"
                                    }
                                    
                                    tls01 {
                                        enabled = false
                                        port = 443
                                    }
                                }
                            }
                        }
                    }
                    
                    management {
                        autoGenerate = true
                        keyStrength = 4096
                        hashAlgorithm = CertHashAlgorithm.SHA256
                        
                        monitoring {
                            enabled = true
                            checkInterval = 6.hours
                            warnDays = 30
                            criticalDays = 7
                            
                            notifications {
                                email = true
                                syslog = true
                                webhook = ""
                            }
                        }
                        
                        backup {
                            enabled = true
                            location = "/etc/ssl/backups"
                            encryption = true
                            retention = 90.days
                            schedule = "0 2 * * *"
                        }
                    }
                }
            }
        }
        
        val certificates = config.security!!.certificates
        certificates.enabled.shouldBeTrue()
        
        val ca = certificates.ca
        ca.certificates shouldContain "/etc/ssl/certs/ca-cert.pem"
        ca.trustStore shouldBe "/etc/ssl/certs/ca-certificates.crt"
        ca.updateCommand shouldBe "update-ca-certificates"
        
        ca.validation.enabled.shouldBeTrue()
        ca.validation.checkRevocation.shouldBeTrue()
        ca.validation.ocspStapling.shouldBeTrue()
        ca.validation.allowSelfSigned.shouldBeFalse()
        ca.validation.maxChainLength shouldBe 10
        
        val client = certificates.client
        client.certificates shouldHaveSize 2
        
        val clientCert = client.certificates[0]
        clientCert.name shouldBe "client-auth"
        clientCert.certFile shouldBe "/etc/ssl/certs/client.pem"
        clientCert.keyFile shouldBe "/etc/ssl/private/client-key.pem"
        clientCert.purpose shouldBe CertificatePurpose.CLIENT_AUTH
        
        val serverCert = client.certificates[1]
        serverCert.name shouldBe "web-server"
        serverCert.purpose shouldBe CertificatePurpose.SERVER_AUTH
        
        val autoRenewal = client.autoRenewal
        autoRenewal.enabled.shouldBeTrue()
        autoRenewal.checkInterval shouldBe 1.days
        autoRenewal.renewalThreshold shouldBe 30.days
        autoRenewal.notifyBeforeExpiry shouldBe 7.days
        
        val acme = autoRenewal.acme
        acme.enabled.shouldBeTrue()
        acme.server shouldBe "https://acme-v02.api.letsencrypt.org/directory"
        acme.email shouldBe "admin@horizonos.local"
        acme.keyType shouldBe ACMEKeyType.RSA2048
        
        val challenges = acme.challenges
        challenges.http01.enabled.shouldBeTrue()
        challenges.http01.webroot shouldBe "/var/www/html"
        challenges.http01.port shouldBe 80
        
        challenges.dns01.enabled.shouldBeFalse()
        challenges.dns01.provider shouldBe "cloudflare"
        
        challenges.tls01.enabled.shouldBeFalse()
        challenges.tls01.port shouldBe 443
        
        val management = certificates.management
        management.autoGenerate.shouldBeTrue()
        management.keyStrength shouldBe 4096
        management.hashAlgorithm shouldBe CertHashAlgorithm.SHA256
        
        val monitoring = management.monitoring
        monitoring.enabled.shouldBeTrue()
        monitoring.checkInterval shouldBe 6.hours
        monitoring.warnDays shouldBe 30
        monitoring.criticalDays shouldBe 7
        
        monitoring.notifications.email.shouldBeTrue()
        monitoring.notifications.syslog.shouldBeTrue()
        
        val backup = management.backup
        backup.enabled.shouldBeTrue()
        backup.location shouldBe "/etc/ssl/backups"
        backup.encryption.shouldBeTrue()
        backup.retention shouldBe 90.days
        backup.schedule shouldBe "0 2 * * *"
    }
    
    "should validate security configuration properly" {
        val config = horizonOS {
            hostname = "validation-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            security {
                enabled = true
                
                pam {
                    enabled = true
                    failDelay = 5.seconds
                    maxTries = 3
                    lockoutTime = 15.minutes
                }
                
                ssh {
                    enabled = true
                    port = 2222
                    permitRootLogin = PermitRootLogin.NO
                    passwordAuthentication = false
                }
                
                firewall {
                    enabled = true
                    backend = FirewallBackend.IPTABLES
                }
            }
        }
        
        // Configuration should be valid
        config.security shouldNotBe null
        config.security!!.enabled.shouldBeTrue()
        
        // All components should be properly configured
        val pam = config.security!!.pam
        pam.enabled.shouldBeTrue()
        
        val ssh = config.security!!.ssh
        ssh.enabled.shouldBeTrue()
        ssh.port shouldBe 2222
        
        val firewall = config.security!!.firewall
        firewall.enabled.shouldBeTrue()
        firewall.backend shouldBe FirewallBackend.IPTABLES
    }
})