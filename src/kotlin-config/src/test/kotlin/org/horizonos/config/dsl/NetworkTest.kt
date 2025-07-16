package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse
import kotlin.time.Duration.Companion.seconds

class NetworkTest : StringSpec({
    
    "should create basic network configuration" {
        val config = horizonOS {
            hostname = "network-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                hostname("test-machine")
                domainName("example.com")
                networkManager(NetworkManagerType.NETWORKMANAGER)
                
                networkInterface("eth0") {
                    type = InterfaceType.ETHERNET
                    enabled = true
                    ipv4 {
                        method = IPv4Method.DHCP
                        dns("1.1.1.1", "8.8.8.8")
                    }
                }
            }
        }
        
        config.network shouldNotBe null
        config.hasNetworking().shouldBeTrue()
        config.network!!.hostname shouldBe "test-machine"
        config.network!!.domainName shouldBe "example.com"
        config.network!!.networkManager shouldBe NetworkManagerType.NETWORKMANAGER
        config.network!!.interfaces shouldHaveSize 1
        
        val eth0 = config.getInterface("eth0")
        eth0 shouldNotBe null
        eth0!!.type shouldBe InterfaceType.ETHERNET
        eth0.enabled.shouldBeTrue()
        eth0.ipv4!!.method shouldBe IPv4Method.DHCP
        eth0.ipv4!!.dns shouldContain "1.1.1.1"
        eth0.ipv4!!.dns shouldContain "8.8.8.8"
    }
    
    "should configure static IP interface" {
        val config = horizonOS {
            hostname = "static-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                networkInterface("eth0") {
                    type = InterfaceType.ETHERNET
                    ipv4 {
                        method = IPv4Method.STATIC
                        address = "192.168.1.100"
                        netmask = "255.255.255.0"
                        gateway = "192.168.1.1"
                        dns("192.168.1.1", "8.8.8.8")
                        
                        route("10.0.0.0/8", "192.168.1.1", 100)
                    }
                    
                    ipv6 {
                        method = IPv6Method.STATIC
                        address = "2001:db8::100"
                        prefixLength = 64
                        gateway = "2001:db8::1"
                        privacy = IPv6Privacy.PREFER_PUBLIC
                    }
                    
                    mtu = 1500
                    metrics = 100
                }
            }
        }
        
        val eth0 = config.getInterface("eth0")
        eth0 shouldNotBe null
        
        val ipv4 = eth0!!.ipv4!!
        ipv4.method shouldBe IPv4Method.STATIC
        ipv4.address shouldBe "192.168.1.100"
        ipv4.netmask shouldBe "255.255.255.0"
        ipv4.gateway shouldBe "192.168.1.1"
        ipv4.routes shouldHaveSize 1
        ipv4.routes[0].destination shouldBe "10.0.0.0/8"
        ipv4.routes[0].gateway shouldBe "192.168.1.1"
        ipv4.routes[0].metric shouldBe 100
        
        val ipv6 = eth0.ipv6!!
        ipv6.method shouldBe IPv6Method.STATIC
        ipv6.address shouldBe "2001:db8::100"
        ipv6.prefixLength shouldBe 64
        ipv6.gateway shouldBe "2001:db8::1"
        ipv6.privacy shouldBe IPv6Privacy.PREFER_PUBLIC
        
        eth0.mtu shouldBe 1500
        eth0.metrics shouldBe 100
    }
    
    "should configure WiFi networks" {
        val config = horizonOS {
            hostname = "wifi-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                wifi {
                    network("HomeWiFi") {
                        password = "supersecret"
                        security = WiFiSecurity.WPA3_PSK
                        autoConnect = true
                        priority = 100
                        band = WiFiBand.ANY
                    }
                    
                    network("WorkWiFi") {
                        security = WiFiSecurity.WPA2_EAP
                        priority = 50
                        enterprise {
                            eap = EAPMethod.PEAP
                            identity = "user@company.com"
                            password = "workpass"
                            phase2 = EAPPhase2.MSCHAPv2
                        }
                    }
                    
                    network("GuestWiFi") {
                        security = WiFiSecurity.NONE
                        autoConnect = false
                        priority = 1
                    }
                }
            }
        }
        
        config.network!!.wifiNetworks shouldHaveSize 3
        
        val homeWifi = config.getWiFiNetwork("HomeWiFi")
        homeWifi shouldNotBe null
        homeWifi!!.password shouldBe "supersecret"
        homeWifi.security shouldBe WiFiSecurity.WPA3_PSK
        homeWifi.autoConnect.shouldBeTrue()
        homeWifi.priority shouldBe 100
        homeWifi.band shouldBe WiFiBand.ANY
        
        val workWifi = config.getWiFiNetwork("WorkWiFi")
        workWifi shouldNotBe null
        workWifi!!.security shouldBe WiFiSecurity.WPA2_EAP
        workWifi.enterprise shouldNotBe null
        workWifi.enterprise!!.eap shouldBe EAPMethod.PEAP
        workWifi.enterprise!!.identity shouldBe "user@company.com"
        workWifi.enterprise!!.phase2 shouldBe EAPPhase2.MSCHAPv2
        
        val guestWifi = config.getWiFiNetwork("GuestWiFi")
        guestWifi shouldNotBe null
        guestWifi!!.security shouldBe WiFiSecurity.NONE
        guestWifi.autoConnect.shouldBeFalse()
        guestWifi.priority shouldBe 1
    }
    
    "should configure VPN connections" {
        val config = horizonOS {
            hostname = "vpn-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                vpn("work-vpn") {
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
                    }
                    
                    routes("10.0.0.0/8", "172.16.0.0/12")
                    dns("10.0.0.1", "10.0.0.2")
                    
                    autoStart {
                        onSSID = "HomeWiFi"
                    }
                }
                
                vpn("wireguard-home") {
                    type = VPNType.WIREGUARD
                    server = "wg.home.com"
                    port = 51820
                    configFile = "/etc/wireguard/home.conf"
                    autoConnect = true
                }
            }
        }
        
        config.network!!.vpnConnections shouldHaveSize 2
        
        val workVpn = config.getVPN("work-vpn")
        workVpn shouldNotBe null
        workVpn!!.type shouldBe VPNType.OPENVPN
        workVpn.server shouldBe "vpn.company.com"
        workVpn.port shouldBe 1194
        workVpn.username shouldBe "john.doe"
        workVpn.configFile shouldBe "/etc/openvpn/work.conf"
        workVpn.killSwitch.shouldBeTrue()
        workVpn.certificates shouldNotBe null
        workVpn.routes shouldHaveSize 2
        workVpn.dnsServers shouldHaveSize 2
        workVpn.autoStart shouldNotBe null
        workVpn.autoStart!!.onSSID shouldBe "HomeWiFi"
        
        val wgVpn = config.getVPN("wireguard-home")
        wgVpn shouldNotBe null
        wgVpn!!.type shouldBe VPNType.WIREGUARD
        wgVpn.autoConnect.shouldBeTrue()
    }
    
    "should configure firewall with semantic rules" {
        val config = horizonOS {
            hostname = "firewall-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                firewall {
                    enabled = true
                    defaultPolicy = FirewallPolicy.REJECT
                    backend = FirewallBackend.NFTABLES
                    logLevel = FirewallLogLevel.WARN
                    
                    allow {
                        ssh(from = "192.168.1.0/24")
                        http()
                        https()
                        port(8080, NetworkProtocol.TCP, from = "localhost")
                        service("nginx")
                    }
                    
                    deny {
                        port(22, from = "0.0.0.0/0", name = "deny-ssh-internet")
                    }
                    
                    rule("custom-rule") {
                        action = FirewallAction.ALLOW
                        direction = FirewallDirection.OUT
                        protocol = NetworkProtocol.UDP
                        port = "53"
                        destinationAddress = "8.8.8.8"
                        log = true
                    }
                    
                    zone("dmz") {
                        interfaces("eth1")
                        sources("10.0.1.0/24")
                        defaultPolicy = FirewallPolicy.ACCEPT
                        services("http", "https")
                        masquerade = true
                    }
                }
            }
        }
        
        val firewall = config.network!!.firewall
        firewall.enabled.shouldBeTrue()
        firewall.defaultPolicy shouldBe FirewallPolicy.REJECT
        firewall.backend shouldBe FirewallBackend.NFTABLES
        firewall.logLevel shouldBe FirewallLogLevel.WARN
        
        firewall.rules shouldHaveSize 7 // 5 allow + 1 deny + 1 custom
        firewall.zones shouldHaveSize 1
        
        val sshRule = firewall.rules.find { it.service == "ssh" }
        sshRule shouldNotBe null
        sshRule!!.action shouldBe FirewallAction.ALLOW
        sshRule.sourceAddress shouldBe "192.168.1.0/24"
        
        val customRule = firewall.rules.find { it.name == "custom-rule" }
        customRule shouldNotBe null
        customRule!!.direction shouldBe FirewallDirection.OUT
        customRule.protocol shouldBe NetworkProtocol.UDP
        customRule.destinationAddress shouldBe "8.8.8.8"
        customRule.log.shouldBeTrue()
        
        val dmzZone = firewall.zones[0]
        dmzZone.name shouldBe "dmz"
        dmzZone.interfaces shouldContain "eth1"
        dmzZone.sources shouldContain "10.0.1.0/24"
        dmzZone.masquerade.shouldBeTrue()
    }
    
    "should configure DNS with different providers" {
        val config = horizonOS {
            hostname = "dns-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                dns {
                    cloudflare()
                    fallbackServers("8.8.8.8", "8.8.4.4")
                    domains("example.com", "test.local")
                    searchDomains("local", "lan")
                    dnsOverTls = true
                    dnsOverHttps = true
                    dnssec = true
                    cache = true
                    resolver = DNSResolver.SYSTEMD_RESOLVED
                    
                    hosts("router", "192.168.1.1")
                    hosts("nas", "192.168.1.10")
                }
            }
        }
        
        val dns = config.network!!.dns
        dns.servers shouldContain "1.1.1.1"
        dns.servers shouldContain "1.0.0.1"
        dns.fallbackServers shouldContain "8.8.8.8"
        dns.domains shouldContain "example.com"
        dns.searchDomains shouldContain "local"
        dns.dnsOverTls.shouldBeTrue()
        dns.dnsOverHttps.shouldBeTrue()
        dns.dnssec.shouldBeTrue()
        dns.cache.shouldBeTrue()
        dns.resolver shouldBe DNSResolver.SYSTEMD_RESOLVED
        dns.hostsFile["router"] shouldBe "192.168.1.1"
        dns.hostsFile["nas"] shouldBe "192.168.1.10"
    }
    
    "should configure network bridges" {
        val config = horizonOS {
            hostname = "bridge-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                bridge("br0") {
                    interfaces("eth0", "eth1")
                    stp = true
                    stpPriority = 32768
                    helloTime = 2.seconds
                    
                    ipv4 {
                        method = IPv4Method.STATIC
                        address = "192.168.100.1"
                        netmask = "255.255.255.0"
                    }
                }
            }
        }
        
        val bridges = config.network!!.bridges
        bridges shouldHaveSize 1
        
        val br0 = bridges[0]
        br0.name shouldBe "br0"
        br0.interfaces shouldContain "eth0"
        br0.interfaces shouldContain "eth1"
        br0.stp.shouldBeTrue()
        br0.stpPriority shouldBe 32768
        br0.helloTime shouldBe 2.seconds
        br0.ipv4!!.method shouldBe IPv4Method.STATIC
        br0.ipv4!!.address shouldBe "192.168.100.1"
    }
    
    "should configure VLANs" {
        val config = horizonOS {
            hostname = "vlan-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                vlan("vlan100", "eth0", 100) {
                    enabled = true
                    ipv4 {
                        method = IPv4Method.DHCP
                    }
                }
                
                vlan("vlan200", "eth0", 200) {
                    ipv4 {
                        method = IPv4Method.STATIC
                        address = "192.168.200.1"
                        netmask = "255.255.255.0"
                    }
                }
            }
        }
        
        val vlans = config.network!!.vlans
        vlans shouldHaveSize 2
        
        val vlan100 = vlans.find { it.vlanId == 100 }
        vlan100 shouldNotBe null
        vlan100!!.name shouldBe "vlan100"
        vlan100.parentInterface shouldBe "eth0"
        vlan100.enabled.shouldBeTrue()
        vlan100.ipv4!!.method shouldBe IPv4Method.DHCP
        
        val vlan200 = vlans.find { it.vlanId == 200 }
        vlan200 shouldNotBe null
        vlan200!!.ipv4!!.method shouldBe IPv4Method.STATIC
        vlan200.ipv4!!.address shouldBe "192.168.200.1"
    }
    
    "should configure proxy settings" {
        val config = horizonOS {
            hostname = "proxy-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                proxy {
                    httpProxy = "http://proxy.company.com:8080"
                    httpsProxy = "https://proxy.company.com:8443"
                    socksProxy = "socks5://proxy.company.com:1080"
                    noProxy("localhost", "127.0.0.1", "*.local")
                    autoConfigUrl = "http://proxy.company.com/proxy.pac"
                }
            }
        }
        
        val proxy = config.network!!.proxy
        proxy shouldNotBe null
        proxy!!.httpProxy shouldBe "http://proxy.company.com:8080"
        proxy.httpsProxy shouldBe "https://proxy.company.com:8443"
        proxy.socksProxy shouldBe "socks5://proxy.company.com:1080"
        proxy.noProxy shouldContain "localhost"
        proxy.noProxy shouldContain "*.local"
        proxy.autoConfigUrl shouldBe "http://proxy.company.com/proxy.pac"
    }
    
    "should handle bonding configuration" {
        val config = horizonOS {
            hostname = "bonding-test"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                networkInterface("bond0") {
                    type = InterfaceType.BOND
                    bonding("eth0", "eth1", "eth2")
                    ipv4 {
                        method = IPv4Method.STATIC
                        address = "192.168.1.100"
                        netmask = "255.255.255.0"
                        gateway = "192.168.1.1"
                    }
                }
                
                networkInterface("eth0") {
                    bondingMaster = "bond0"
                }
                
                networkInterface("eth1") {
                    bondingMaster = "bond0"
                }
            }
        }
        
        val bond0 = config.getInterface("bond0")
        bond0 shouldNotBe null
        bond0!!.type shouldBe InterfaceType.BOND
        bond0.bondingSlaves shouldHaveSize 3
        bond0.bondingSlaves shouldContain "eth0"
        bond0.bondingSlaves shouldContain "eth1"
        bond0.bondingSlaves shouldContain "eth2"
        
        val eth0 = config.getInterface("eth0")
        eth0!!.bondingMaster shouldBe "bond0"
    }
})