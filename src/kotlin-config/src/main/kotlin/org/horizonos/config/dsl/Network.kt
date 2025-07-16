package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.security.FirewallConfig
import org.horizonos.config.dsl.security.FirewallBackend
import org.horizonos.config.dsl.security.FirewallRule
import org.horizonos.config.dsl.security.FirewallZone
import org.horizonos.config.dsl.security.FirewallPolicy
import org.horizonos.config.dsl.security.FirewallAction
import org.horizonos.config.dsl.security.ConnectionState
import org.horizonos.config.dsl.security.NetworkProtocol
import org.horizonos.config.dsl.network.*
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Network Configuration DSL for HorizonOS
 * 
 * Provides comprehensive network configuration and management for HorizonOS systems.
 * This module handles network interfaces, wireless connectivity, VPN connections,
 * firewall rules, DNS configuration, and advanced networking features with a focus
 * on security, performance, and reliability.
 * 
 * ## Network Components:
 * - **Interfaces**: Ethernet, WiFi, bridge, bond, and virtual interfaces
 * - **Wireless**: WiFi network configuration, enterprise authentication
 * - **VPN**: WireGuard, OpenVPN, IPSec tunnel configuration
 * - **Firewall**: iptables, nftables, firewalld integration
 * - **DNS**: DNS resolution, caching, DNS-over-HTTPS/TLS
 * - **Routing**: Static routes, policy routing, traffic shaping
 * - **Monitoring**: Network traffic monitoring and alerting
 * - **Security**: Network security policies and intrusion detection
 * 
 * ## Key Features:
 * - **Auto-configuration**: Automatic network detection and configuration
 * - **Failover**: Automatic network failover and redundancy
 * - **Security**: Built-in firewall and network security features
 * - **Performance**: Network optimization and quality of service
 * - **Monitoring**: Real-time network monitoring and diagnostics
 * - **IPv6 Support**: Full IPv6 support with dual-stack configuration
 * 
 * ## Basic Usage:
 * ```kotlin
 * network {
 *     interface("eth0") {
 *         type = InterfaceType.ETHERNET
 *         dhcp = true
 *         
 *         // Static IP configuration
 *         staticIP {
 *             address = "192.168.1.100"
 *             netmask = "255.255.255.0"
 *             gateway = "192.168.1.1"
 *         }
 *     }
 *     
 *     wifi {
 *         network("home-network") {
 *             ssid = "MyHomeWiFi"
 *             security = WiFiSecurity.WPA2_PSK
 *             password = "secure_password"
 *             autoConnect = true
 *         }
 *         
 *         network("work-network") {
 *             ssid = "CorporateWiFi"
 *             security = WiFiSecurity.WPA2_ENTERPRISE
 *             eap {
 *                 method = EAPMethod.PEAP
 *                 identity = "username"
 *                 password = "password"
 *             }
 *         }
 *     }
 *     
 *     vpn {
 *         wireguard("office-vpn") {
 *             privateKey = "client_private_key"
 *             publicKey = "server_public_key"
 *             endpoint = "vpn.company.com:51820"
 *             allowedIPs = listOf("10.0.0.0/8")
 *         }
 *     }
 *     
 *     firewall {
 *         enabled = true
 *         defaultPolicy = FirewallPolicy.DROP
 *         
 *         rule {
 *             port = 22
 *             protocol = NetworkProtocol.TCP
 *             action = FirewallAction.ACCEPT
 *             comment = "SSH access"
 *         }
 *         
 *         rule {
 *             port = 80
 *             protocol = NetworkProtocol.TCP
 *             action = FirewallAction.ACCEPT
 *             comment = "HTTP access"
 *         }
 *     }
 *     
 *     dns {
 *         servers = listOf("1.1.1.1", "8.8.8.8")
 *         dnssec = true
 *         dnsOverHttps = true
 *         
 *         cache {
 *             enabled = true
 *             size = "100M"
 *             ttl = 3600
 *         }
 *     }
 * }
 * ```
 * 
 * ## Network Profiles:
 * HorizonOS supports different network profiles for various environments:
 * - **Home**: Optimized for home networks with basic security
 * - **Office**: Enterprise-grade security with VPN and authentication
 * - **Public**: High-security profile for public WiFi networks
 * - **Server**: Server-optimized with advanced firewall and monitoring
 * - **Mobile**: Optimized for mobile/cellular connections
 * 
 * ## Security Features:
 * - **Firewall**: Stateful firewall with intrusion detection
 * - **VPN**: Secure VPN connections with modern protocols
 * - **Encryption**: WiFi WPA3 and enterprise authentication
 * - **DNS Security**: DNS filtering and DNS-over-HTTPS
 * - **Network Isolation**: VLAN and network segmentation support
 * 
 * @since 1.0
 * @see [NetworkInterface] for network interface configuration
 * @see [WiFiNetwork] for wireless network configuration
 * @see [VPNConnection] for VPN connection configuration
 * @see [FirewallConfig] for firewall configuration
 * @see [Security] for security policies and configurations
 * @see [SecurityConfig] for security configuration
 * @see [Services] for network services and daemon management
 * @see [Hardware] for network hardware configuration
 * @see [Storage] for network storage configuration
 * @see [horizonOS] for main system configuration entry point
 */

// ===== Network Configuration =====

@Serializable
data class NetworkConfig(
    val interfaces: List<NetworkInterface> = emptyList(),
    val wifiNetworks: List<WiFiNetwork> = emptyList(),
    val vpnConnections: List<VPNConnection> = emptyList(),
    val firewall: FirewallConfig = FirewallConfig(),
    val dns: DNSConfig = DNSConfig(),
    val bridges: List<NetworkBridge> = emptyList(),
    val vlans: List<VLANConfig> = emptyList(),
    val networkManager: NetworkManagerType = NetworkManagerType.NETWORKMANAGER,
    val hostname: String = "",
    val domainName: String = "",
    val proxy: ProxyConfig? = null
)

// Network interface, IPv4/IPv6, WiFi, VPN, DNS, and proxy data classes are now imported from network package

// Firewall classes are now imported from org.horizonos.config.dsl.security

// Bridge, VLAN, and proxy data classes are now imported from network package

// ===== Enums =====

// Enums are now imported from network package

@Serializable
enum class FirewallDirection {
    IN,
    OUT,
    FORWARD
}

@Serializable
enum class FirewallLogLevel {
    NONE,
    ERROR,
    WARN,
    INFO,
    DEBUG
}

// NetworkProtocol is also imported from security package

// ===== DSL Builders =====

@HorizonOSDsl
class NetworkContext {
    private val interfaces = mutableListOf<NetworkInterface>()
    private val wifiNetworks = mutableListOf<WiFiNetwork>()
    private val vpnConnections = mutableListOf<VPNConnection>()
    private var firewall = FirewallConfig()
    private var dns = DNSConfig()
    private val bridges = mutableListOf<NetworkBridge>()
    private val vlans = mutableListOf<VLANConfig>()
    private var networkManager = NetworkManagerType.NETWORKMANAGER
    private var hostname = ""
    private var domainName = ""
    private var proxy: ProxyConfig? = null
    
    fun networkInterface(name: String, block: NetworkInterfaceContext.() -> Unit) {
        val context = NetworkInterfaceContext().apply {
            this.name = name
            block()
        }
        interfaces.add(context.toInterface())
    }
    
    fun wifi(block: WiFiContext.() -> Unit) {
        val context = WiFiContext().apply(block)
        wifiNetworks.addAll(context.networks)
    }
    
    fun vpn(name: String, block: VPNContext.() -> Unit) {
        val context = VPNContext().apply {
            this.name = name
            block()
        }
        vpnConnections.add(context.toVPN())
    }
    
    fun firewall(block: FirewallContext.() -> Unit) {
        firewall = FirewallContext().apply(block).toFirewall()
    }
    
    fun dns(block: DNSContext.() -> Unit) {
        dns = DNSContext().apply(block).toDNS()
    }
    
    fun bridge(name: String, block: BridgeContext.() -> Unit) {
        val context = BridgeContext().apply {
            this.name = name
            block()
        }
        bridges.add(context.toBridge())
    }
    
    fun vlan(name: String, parentInterface: String, vlanId: Int, block: VLANContext.() -> Unit = {}) {
        val context = VLANContext().apply {
            this.name = name
            this.parentInterface = parentInterface
            this.vlanId = vlanId
            block()
        }
        vlans.add(context.toVLAN())
    }
    
    fun hostname(name: String) {
        hostname = name
    }
    
    fun domainName(name: String) {
        domainName = name
    }
    
    fun networkManager(type: NetworkManagerType) {
        networkManager = type
    }
    
    fun proxy(block: ProxyContext.() -> Unit) {
        proxy = ProxyContext().apply(block).toProxy()
    }
    
    fun toConfig() = NetworkConfig(
        interfaces = interfaces,
        wifiNetworks = wifiNetworks,
        vpnConnections = vpnConnections,
        firewall = firewall,
        dns = dns,
        bridges = bridges,
        vlans = vlans,
        networkManager = networkManager,
        hostname = hostname,
        domainName = domainName,
        proxy = proxy
    )
}

// Network interface, IPv4/IPv6, WiFi, and VPN contexts are now imported from network package

@HorizonOSDsl
class FirewallContext {
    var enabled: Boolean = true
    var defaultPolicy: FirewallPolicy = FirewallPolicy.REJECT
    private val rules = mutableListOf<FirewallRule>()
    private val zones = mutableListOf<FirewallZone>()
    var backend: FirewallBackend = FirewallBackend.NFTABLES
    var logLevel: FirewallLogLevel = FirewallLogLevel.WARN
    var logDropped: Boolean = false
    
    fun allow(block: FirewallRuleBuilder.() -> Unit) {
        val builder = FirewallRuleBuilder(FirewallAction.ACCEPT)
        builder.block()
        rules.addAll(builder.rules)
    }
    
    fun deny(block: FirewallRuleBuilder.() -> Unit) {
        val builder = FirewallRuleBuilder(FirewallAction.DROP)
        builder.block()
        rules.addAll(builder.rules)
    }
    
    fun rule(name: String, block: FirewallRuleContext.() -> Unit) {
        val context = FirewallRuleContext().apply {
            this.name = name
            block()
        }
        rules.add(context.toRule())
    }
    
    fun zone(name: String, block: FirewallZoneContext.() -> Unit) {
        val context = FirewallZoneContext().apply {
            this.name = name
            block()
        }
        zones.add(context.toZone())
    }
    
    fun toFirewall() = FirewallConfig(
        enabled = enabled,
        backend = backend,
        defaultPolicy = mapOf("INPUT" to defaultPolicy, "FORWARD" to FirewallPolicy.DROP, "OUTPUT" to FirewallPolicy.ACCEPT),
        rules = rules,
        zones = zones
    )
}

@HorizonOSDsl
class FirewallRuleBuilder(private val action: FirewallAction) {
    internal val rules = mutableListOf<FirewallRule>()
    
    fun ssh(from: String = "any", name: String = "allow-ssh") {
        rules.add(FirewallRule(
            name = name,
            chain = "INPUT",
            action = action,
            protocol = "tcp",
            dport = "22",
            source = if (from == "any") null else from
        ))
    }
    
    fun http(from: String = "any", name: String = "allow-http") {
        rules.add(FirewallRule(
            name = name,
            chain = "INPUT",
            action = action,
            protocol = "tcp",
            dport = "80",
            source = if (from == "any") null else from
        ))
    }
    
    fun https(from: String = "any", name: String = "allow-https") {
        rules.add(FirewallRule(
            name = name,
            chain = "INPUT",
            action = action,
            protocol = "tcp",
            dport = "443",
            source = if (from == "any") null else from
        ))
    }
    
    fun port(port: Int, protocol: NetworkProtocol = NetworkProtocol.TCP, from: String = "any", name: String? = null) {
        rules.add(FirewallRule(
            name = name ?: "${action.name.lowercase()}-port-$port",
            chain = "INPUT",
            action = action,
            protocol = protocol.name.lowercase(),
            dport = port.toString(),
            source = if (from == "any") null else from
        ))
    }
    
    fun service(service: String, from: String = "any", name: String? = null) {
        val servicePort = when(service) {
            "ssh" -> "22"
            "http" -> "80"
            "https" -> "443"
            "smtp" -> "25"
            "ftp" -> "21"
            "dns" -> "53"
            else -> service // assume it's a port number
        }
        rules.add(FirewallRule(
            name = name ?: "${action.name.lowercase()}-$service",
            chain = "INPUT",
            action = action,
            protocol = "tcp",
            dport = servicePort,
            source = if (from == "any") null else from
        ))
    }
}

@HorizonOSDsl
class FirewallRuleContext {
    var name: String = ""
    var action: FirewallAction = FirewallAction.ACCEPT
    var direction: FirewallDirection = FirewallDirection.IN
    var protocol: NetworkProtocol? = null
    var port: String? = null
    var sourceAddress: String? = null
    var destinationAddress: String? = null
    var interfaceName: String? = null
    var service: String? = null
    var priority: Int = 50
    var enabled: Boolean = true
    var log: Boolean = false
    
    fun toRule() = FirewallRule(
        name = name,
        chain = when(direction) {
            FirewallDirection.IN -> "INPUT"
            FirewallDirection.OUT -> "OUTPUT"
            FirewallDirection.FORWARD -> "FORWARD"
        },
        protocol = protocol?.name?.lowercase(),
        source = sourceAddress,
        destination = destinationAddress,
        dport = port,
        action = action,
        comment = if (log) "LOG: $name" else null,
        priority = priority,
        enabled = enabled
    )
}

@HorizonOSDsl
class FirewallZoneContext {
    var name: String = ""
    private val interfaces = mutableListOf<String>()
    private val sources = mutableListOf<String>()
    var defaultPolicy: FirewallPolicy = FirewallPolicy.REJECT
    private val services = mutableListOf<String>()
    private val ports = mutableListOf<String>()
    var masquerade: Boolean = false
    
    fun interfaces(vararg interfaceList: String) {
        interfaces.addAll(interfaceList)
    }
    
    fun sources(vararg sourceList: String) {
        sources.addAll(sourceList)
    }
    
    fun services(vararg serviceList: String) {
        services.addAll(serviceList)
    }
    
    fun ports(vararg portList: String) {
        ports.addAll(portList)
    }
    
    fun toZone() = FirewallZone(
        name = name,
        interfaces = interfaces,
        sources = sources,
        target = defaultPolicy,
        services = services,
        ports = ports,
        masquerade = masquerade
    )
}

// DNS, bridge, VLAN, and proxy contexts are now imported from network package

// ===== Extension Functions =====

fun CompiledConfig.hasNetworking(): Boolean = network != null

fun CompiledConfig.getInterface(name: String): NetworkInterface? = 
    network?.interfaces?.find { it.name == name }

fun CompiledConfig.getWiFiNetwork(ssid: String): WiFiNetwork? = 
    network?.wifiNetworks?.find { it.ssid == ssid }

fun CompiledConfig.getVPN(name: String): VPNConnection? = 
    network?.vpnConnections?.find { it.name == name }