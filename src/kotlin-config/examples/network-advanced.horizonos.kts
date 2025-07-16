import org.horizonos.config.dsl.*

horizonOS {
    // System configuration
    hostname = "network-showcase"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"

    // Comprehensive network configuration
    network {
        hostname("secure-workstation")
        domainName("company.local")
        networkManager(NetworkManagerType.NETWORKMANAGER)
        
        // Ethernet interface with static IP
        networkInterface("eth0") {
            type = InterfaceType.ETHERNET
            enabled = true
            ipv4 {
                method = IPv4Method.STATIC
                address = "192.168.1.100"
                netmask = "255.255.255.0"
                gateway = "192.168.1.1"
                dns("192.168.1.1", "1.1.1.1")
                
                // Custom routes
                route("10.0.0.0/8", "192.168.1.1", 100)
                route("172.16.0.0/12", "192.168.1.1", 200)
            }
            
            ipv6 {
                method = IPv6Method.AUTO
                privacy = IPv6Privacy.PREFER_TEMPORARY
            }
            
            mtu = 1500
            metrics = 100
        }
        
        // WiFi networks
        wifi {
            // Home WiFi with WPA3
            network("HomeWiFi") {
                password = "supersecret123"
                security = WiFiSecurity.WPA3_PSK
                autoConnect = true
                priority = 100
                band = WiFiBand.ANY
                powerSave = false
            }
            
            // Enterprise WiFi
            network("CorpWiFi") {
                security = WiFiSecurity.WPA2_EAP
                priority = 80
                autoConnect = false
                enterprise {
                    eap = EAPMethod.PEAP
                    identity = "john.doe@company.com"
                    password = "corporatepass"
                    phase2 = EAPPhase2.MSCHAPv2
                    caCertificate = "/etc/ssl/certs/corp-ca.pem"
                }
            }
            
            // Guest network
            network("GuestWiFi") {
                security = WiFiSecurity.NONE
                autoConnect = false
                priority = 10
            }
        }
        
        // VPN connections
        vpn("work-openvpn") {
            type = VPNType.OPENVPN
            server = "vpn.company.com"
            port = 1194
            username = "john.doe"
            configFile = "/etc/openvpn/work.conf"
            killSwitch = true
            
            certificates {
                caCertificate = "/etc/openvpn/ca.crt"
                clientCertificate = "/etc/openvpn/client.crt"
                privateKey = "/etc/openvpn/client.key"
                tlsAuthKey = "/etc/openvpn/tls-auth.key"
            }
            
            routes("10.0.0.0/8", "172.16.0.0/12")
            dns("10.0.0.1", "10.0.0.2")
            
            autoStart {
                onSSID = "HomeWiFi"
            }
        }
        
        vpn("home-wireguard") {
            type = VPNType.WIREGUARD
            server = "wg.home.example.com"
            port = 51820
            configFile = "/etc/wireguard/home.conf"
            autoConnect = false
            killSwitch = true
        }
        
        // Comprehensive firewall configuration
        firewall {
            enabled = true
            defaultPolicy = FirewallPolicy.REJECT
            backend = FirewallBackend.NFTABLES
            logLevel = FirewallLogLevel.WARN
            logDropped = true
            
            // Allow common services with semantic rules
            allow {
                ssh(from = "192.168.1.0/24")
                http()
                https()
                
                // Custom application ports
                port(8080, NetworkProtocol.TCP, from = "localhost", name = "dev-server")
                port(3000, NetworkProtocol.TCP, from = "192.168.1.0/24", name = "react-dev")
                port(5432, NetworkProtocol.TCP, from = "10.0.0.0/8", name = "postgres")
                
                service("nginx")
                service("docker")
            }
            
            // Deny specific traffic
            deny {
                port(22, from = "0.0.0.0/0", name = "deny-ssh-internet")
                service("telnet")
            }
            
            // Custom rules
            rule("allow-dns-out") {
                action = FirewallAction.ALLOW
                direction = FirewallDirection.OUT
                protocol = NetworkProtocol.UDP
                port = "53"
                destinationAddress = "1.1.1.1"
                log = false
            }
            
            rule("block-torrent") {
                action = FirewallAction.DENY
                direction = FirewallDirection.OUT
                port = "6881-6889"
                log = true
            }
            
            // Network zones
            zone("dmz") {
                interfaces("eth1")
                sources("10.0.1.0/24")
                defaultPolicy = FirewallPolicy.ACCEPT
                services("http", "https", "ssh")
                ports("80", "443", "22")
                masquerade = true
            }
            
            zone("internal") {
                interfaces("eth0")
                sources("192.168.1.0/24")
                defaultPolicy = FirewallPolicy.ACCEPT
                services("ssh", "nfs", "samba")
                masquerade = false
            }
        }
        
        // DNS configuration
        dns {
            // Use Cloudflare DNS
            cloudflare()
            fallbackServers("8.8.8.8", "8.8.4.4")
            
            domains("company.local", "internal.lan")
            searchDomains("local", "lan", "company.local")
            
            // Privacy and security features
            dnsOverTls = true
            dnsOverHttps = true
            dnssec = true
            cache = true
            resolver = DNSResolver.SYSTEMD_RESOLVED
            
            // Custom host entries
            hosts("router", "192.168.1.1")
            hosts("nas", "192.168.1.10")
            hosts("printer", "192.168.1.20")
            hosts("homeassistant", "192.168.1.30")
        }
        
        // Network bridge for virtualization
        bridge("br0") {
            interfaces("eth2", "eth3")
            stp = true
            stpPriority = 32768
            helloTime = 2.seconds
            forwardDelay = 15.seconds
            maxAge = 20.seconds
            
            ipv4 {
                method = IPv4Method.STATIC
                address = "192.168.100.1"
                netmask = "255.255.255.0"
            }
        }
        
        // VLANs for network segmentation
        vlan("vlan-servers", "eth0", 100) {
            enabled = true
            ipv4 {
                method = IPv4Method.STATIC
                address = "192.168.100.1"
                netmask = "255.255.255.0"
            }
        }
        
        vlan("vlan-iot", "eth0", 200) {
            enabled = true
            ipv4 {
                method = IPv4Method.DHCP
            }
        }
        
        vlan("vlan-guest", "eth0", 300) {
            enabled = true
            ipv4 {
                method = IPv4Method.STATIC
                address = "192.168.300.1"
                netmask = "255.255.255.0"
            }
        }
        
        // Corporate proxy configuration
        proxy {
            httpProxy = "http://proxy.company.com:8080"
            httpsProxy = "https://proxy.company.com:8443"
            socksProxy = "socks5://proxy.company.com:1080"
            noProxy("localhost", "127.0.0.1", "*.local", "*.company.com")
            autoConfigUrl = "http://proxy.company.com/proxy.pac"
        }
    }

    // Basic packages for networking
    packages {
        group("network") {
            install(
                "networkmanager", "networkmanager-openvpn", "networkmanager-vpnc",
                "wireguard-tools", "openvpn", "nftables", "iptables-nft",
                "dnsmasq", "systemd-resolved", "bridge-utils", "vlan"
            )
        }
    }

    // Network services
    services {
        enable("NetworkManager") {
            autoRestart = true
            restartOnFailure = true
        }
        enable("systemd-resolved")
        enable("nftables")
        disable("iptables") // Using nftables instead
    }

    // Network admin user
    users {
        user("netadmin") {
            uid = 1001
            shell = "/usr/bin/bash"
            groups("wheel", "networkmanager", "docker")
        }
    }
}