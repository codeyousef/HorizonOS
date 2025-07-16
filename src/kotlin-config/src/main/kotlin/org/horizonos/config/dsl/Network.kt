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
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

/**
 * Network Configuration DSL for HorizonOS
 * 
 * Provides type-safe configuration for all networking aspects including
 * interfaces, WiFi, VPN, firewall, DNS, and advanced networking features.
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

@Serializable
data class NetworkInterface(
    val name: String,
    val type: InterfaceType = InterfaceType.ETHERNET,
    val enabled: Boolean = true,
    val ipv4: IPv4Config? = null,
    val ipv6: IPv6Config? = null,
    val mtu: Int? = null,
    val mac: String? = null,
    val bondingMaster: String? = null,
    val bondingSlaves: List<String> = emptyList(),
    val metrics: Int? = null
)

@Serializable
data class IPv4Config(
    val method: IPv4Method = IPv4Method.DHCP,
    val address: String? = null,
    val netmask: String? = null,
    val gateway: String? = null,
    val dns: List<String> = emptyList(),
    val routes: List<StaticRoute> = emptyList(),
    val dhcpOptions: Map<String, String> = emptyMap()
)

@Serializable
data class IPv6Config(
    val method: IPv6Method = IPv6Method.AUTO,
    val address: String? = null,
    val prefixLength: Int? = null,
    val gateway: String? = null,
    val dns: List<String> = emptyList(),
    val routes: List<StaticRoute> = emptyList(),
    val privacy: IPv6Privacy = IPv6Privacy.PREFER_TEMPORARY
)

@Serializable
data class StaticRoute(
    val destination: String,
    val gateway: String,
    val metric: Int? = null,
    val interfaceName: String? = null
)

@Serializable
data class WiFiNetwork(
    val ssid: String,
    val password: String? = null,
    val security: WiFiSecurity = WiFiSecurity.WPA3_PSK,
    val hidden: Boolean = false,
    val priority: Int = 50,
    val autoConnect: Boolean = true,
    val bssid: String? = null,
    val band: WiFiBand? = null,
    val powerSave: Boolean = true,
    val enterprise: WiFiEnterprise? = null
)

@Serializable
data class WiFiEnterprise(
    val eap: EAPMethod,
    val identity: String,
    val password: String? = null,
    val certificate: String? = null,
    val privateKey: String? = null,
    val caCertificate: String? = null,
    val phase2: EAPPhase2? = null
)

@Serializable
data class VPNConnection(
    val name: String,
    val type: VPNType,
    val enabled: Boolean = true,
    val autoConnect: Boolean = false,
    val server: String,
    val port: Int? = null,
    val username: String? = null,
    val password: String? = null,
    val configFile: String? = null,
    val certificates: VPNCertificates? = null,
    val routes: List<String> = emptyList(),
    val dnsServers: List<String> = emptyList(),
    val killSwitch: Boolean = false,
    val autoStart: VPNAutoStart? = null
)

@Serializable
data class VPNCertificates(
    val caCertificate: String? = null,
    val clientCertificate: String? = null,
    val privateKey: String? = null,
    val tlsAuthKey: String? = null
)

@Serializable
data class VPNAutoStart(
    val onNetwork: String? = null,
    val onSSID: String? = null,
    val onInterface: String? = null
)

// Firewall classes are now imported from org.horizonos.config.dsl.security

@Serializable
data class DNSConfig(
    val servers: List<String> = listOf("1.1.1.1", "8.8.8.8"),
    val fallbackServers: List<String> = emptyList(),
    val domains: List<String> = emptyList(),
    val searchDomains: List<String> = emptyList(),
    val dnsOverTls: Boolean = false,
    val dnsOverHttps: Boolean = false,
    val dnssec: Boolean = false,
    val cache: Boolean = true,
    val resolver: DNSResolver = DNSResolver.SYSTEMD_RESOLVED,
    val hostsFile: Map<String, String> = emptyMap()
)

@Serializable
data class NetworkBridge(
    val name: String,
    val interfaces: List<String> = emptyList(),
    val stp: Boolean = true,
    val stpPriority: Int = 32768,
    val helloTime: Duration = 2.seconds,
    val forwardDelay: Duration = 15.seconds,
    val maxAge: Duration = 20.seconds,
    val ipv4: IPv4Config? = null,
    val ipv6: IPv6Config? = null
)

@Serializable
data class VLANConfig(
    val name: String,
    val parentInterface: String,
    val vlanId: Int,
    val ipv4: IPv4Config? = null,
    val ipv6: IPv6Config? = null,
    val enabled: Boolean = true
)

@Serializable
data class ProxyConfig(
    val httpProxy: String? = null,
    val httpsProxy: String? = null,
    val ftpProxy: String? = null,
    val socksProxy: String? = null,
    val noProxy: List<String> = emptyList(),
    val autoConfigUrl: String? = null
)

// ===== Enums =====

@Serializable
enum class InterfaceType {
    ETHERNET,
    WIRELESS,
    BRIDGE,
    BOND,
    VLAN,
    TUNNEL,
    LOOPBACK
}

@Serializable
enum class IPv4Method {
    DHCP,
    STATIC,
    LINK_LOCAL,
    SHARED,
    DISABLED
}

@Serializable
enum class IPv6Method {
    AUTO,
    DHCP,
    STATIC,
    LINK_LOCAL,
    IGNORE,
    SHARED
}

@Serializable
enum class IPv6Privacy {
    DISABLED,
    PREFER_PUBLIC,
    PREFER_TEMPORARY
}

@Serializable
enum class WiFiSecurity {
    NONE,
    WEP,
    WPA_PSK,
    WPA2_PSK,
    WPA3_PSK,
    WPA_EAP,
    WPA2_EAP,
    WPA3_EAP,
    SAE
}

@Serializable
enum class WiFiBand {
    A,      // 5GHz
    BG,     // 2.4GHz
    ANY
}

@Serializable
enum class EAPMethod {
    TLS,
    TTLS,
    PEAP,
    FAST,
    LEAP
}

@Serializable
enum class EAPPhase2 {
    MSCHAPv2,
    MD5,
    GTC,
    TLS
}

@Serializable
enum class VPNType {
    OPENVPN,
    WIREGUARD,
    IPSEC,
    PPTP,
    L2TP,
    SSTP,
    TAILSCALE
}

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

@Serializable
enum class DNSResolver {
    SYSTEMD_RESOLVED,
    DNSMASQ,
    BIND,
    UNBOUND,
    PIHOLE
}

@Serializable
enum class NetworkManagerType {
    NETWORKMANAGER,
    SYSTEMD_NETWORKD,
    NETCTL,
    WICD
}

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

@HorizonOSDsl
class NetworkInterfaceContext {
    var name: String = ""
    var type: InterfaceType = InterfaceType.ETHERNET
    var enabled: Boolean = true
    private var ipv4: IPv4Config? = null
    private var ipv6: IPv6Config? = null
    var mtu: Int? = null
    var mac: String? = null
    var bondingMaster: String? = null
    private val bondingSlaves = mutableListOf<String>()
    var metrics: Int? = null
    
    fun ipv4(block: IPv4Context.() -> Unit) {
        ipv4 = IPv4Context().apply(block).toIPv4()
    }
    
    fun ipv6(block: IPv6Context.() -> Unit) {
        ipv6 = IPv6Context().apply(block).toIPv6()
    }
    
    fun bonding(vararg slaves: String) {
        bondingSlaves.addAll(slaves)
    }
    
    fun toInterface() = NetworkInterface(
        name = name,
        type = type,
        enabled = enabled,
        ipv4 = ipv4,
        ipv6 = ipv6,
        mtu = mtu,
        mac = mac,
        bondingMaster = bondingMaster,
        bondingSlaves = bondingSlaves,
        metrics = metrics
    )
}

@HorizonOSDsl
class IPv4Context {
    var method: IPv4Method = IPv4Method.DHCP
    var address: String? = null
    var netmask: String? = null
    var gateway: String? = null
    private val dns = mutableListOf<String>()
    private val routes = mutableListOf<StaticRoute>()
    private val dhcpOptions = mutableMapOf<String, String>()
    
    fun dns(vararg servers: String) {
        dns.addAll(servers)
    }
    
    fun route(destination: String, gateway: String, metric: Int? = null, interfaceName: String? = null) {
        routes.add(StaticRoute(destination, gateway, metric, interfaceName))
    }
    
    fun dhcpOption(key: String, value: String) {
        dhcpOptions[key] = value
    }
    
    fun toIPv4() = IPv4Config(
        method = method,
        address = address,
        netmask = netmask,
        gateway = gateway,
        dns = dns,
        routes = routes,
        dhcpOptions = dhcpOptions
    )
}

@HorizonOSDsl
class IPv6Context {
    var method: IPv6Method = IPv6Method.AUTO
    var address: String? = null
    var prefixLength: Int? = null
    var gateway: String? = null
    private val dns = mutableListOf<String>()
    private val routes = mutableListOf<StaticRoute>()
    var privacy: IPv6Privacy = IPv6Privacy.PREFER_TEMPORARY
    
    fun dns(vararg servers: String) {
        dns.addAll(servers)
    }
    
    fun route(destination: String, gateway: String, metric: Int? = null, interfaceName: String? = null) {
        routes.add(StaticRoute(destination, gateway, metric, interfaceName))
    }
    
    fun toIPv6() = IPv6Config(
        method = method,
        address = address,
        prefixLength = prefixLength,
        gateway = gateway,
        dns = dns,
        routes = routes,
        privacy = privacy
    )
}

@HorizonOSDsl
class WiFiContext {
    internal val networks = mutableListOf<WiFiNetwork>()
    
    fun network(ssid: String, block: WiFiNetworkContext.() -> Unit = {}) {
        val context = WiFiNetworkContext().apply {
            this.ssid = ssid
            block()
        }
        networks.add(context.toNetwork())
    }
}

@HorizonOSDsl
class WiFiNetworkContext {
    var ssid: String = ""
    var password: String? = null
    var security: WiFiSecurity = WiFiSecurity.WPA3_PSK
    var hidden: Boolean = false
    var priority: Int = 50
    var autoConnect: Boolean = true
    var bssid: String? = null
    var band: WiFiBand? = null
    var powerSave: Boolean = true
    private var enterprise: WiFiEnterprise? = null
    
    fun enterprise(block: WiFiEnterpriseContext.() -> Unit) {
        enterprise = WiFiEnterpriseContext().apply(block).toEnterprise()
    }
    
    fun toNetwork() = WiFiNetwork(
        ssid = ssid,
        password = password,
        security = security,
        hidden = hidden,
        priority = priority,
        autoConnect = autoConnect,
        bssid = bssid,
        band = band,
        powerSave = powerSave,
        enterprise = enterprise
    )
}

@HorizonOSDsl
class WiFiEnterpriseContext {
    var eap: EAPMethod = EAPMethod.PEAP
    var identity: String = ""
    var password: String? = null
    var certificate: String? = null
    var privateKey: String? = null
    var caCertificate: String? = null
    var phase2: EAPPhase2? = null
    
    fun toEnterprise() = WiFiEnterprise(
        eap = eap,
        identity = identity,
        password = password,
        certificate = certificate,
        privateKey = privateKey,
        caCertificate = caCertificate,
        phase2 = phase2
    )
}

@HorizonOSDsl
class VPNContext {
    var name: String = ""
    var type: VPNType = VPNType.OPENVPN
    var enabled: Boolean = true
    var autoConnect: Boolean = false
    var server: String = ""
    var port: Int? = null
    var username: String? = null
    var password: String? = null
    var configFile: String? = null
    private var certificates: VPNCertificates? = null
    private val routes = mutableListOf<String>()
    private val dnsServers = mutableListOf<String>()
    var killSwitch: Boolean = false
    private var autoStart: VPNAutoStart? = null
    
    fun certificates(block: VPNCertificatesContext.() -> Unit) {
        certificates = VPNCertificatesContext().apply(block).toCertificates()
    }
    
    fun routes(vararg routeList: String) {
        routes.addAll(routeList)
    }
    
    fun dns(vararg servers: String) {
        dnsServers.addAll(servers)
    }
    
    fun autoStart(block: VPNAutoStartContext.() -> Unit) {
        autoStart = VPNAutoStartContext().apply(block).toAutoStart()
    }
    
    fun toVPN() = VPNConnection(
        name = name,
        type = type,
        enabled = enabled,
        autoConnect = autoConnect,
        server = server,
        port = port,
        username = username,
        password = password,
        configFile = configFile,
        certificates = certificates,
        routes = routes,
        dnsServers = dnsServers,
        killSwitch = killSwitch,
        autoStart = autoStart
    )
}

@HorizonOSDsl
class VPNCertificatesContext {
    var caCertificate: String? = null
    var clientCertificate: String? = null
    var privateKey: String? = null
    var tlsAuthKey: String? = null
    
    fun toCertificates() = VPNCertificates(
        caCertificate = caCertificate,
        clientCertificate = clientCertificate,
        privateKey = privateKey,
        tlsAuthKey = tlsAuthKey
    )
}

@HorizonOSDsl
class VPNAutoStartContext {
    var onNetwork: String? = null
    var onSSID: String? = null
    var onInterface: String? = null
    
    fun toAutoStart() = VPNAutoStart(
        onNetwork = onNetwork,
        onSSID = onSSID,
        onInterface = onInterface
    )
}

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

@HorizonOSDsl
class DNSContext {
    private val servers = mutableListOf<String>()
    private val fallbackServers = mutableListOf<String>()
    private val domains = mutableListOf<String>()
    private val searchDomains = mutableListOf<String>()
    var dnsOverTls: Boolean = false
    var dnsOverHttps: Boolean = false
    var dnssec: Boolean = false
    var cache: Boolean = true
    var resolver: DNSResolver = DNSResolver.SYSTEMD_RESOLVED
    private val hostsFile = mutableMapOf<String, String>()
    
    fun servers(vararg serverList: String) {
        servers.clear()
        servers.addAll(serverList)
    }
    
    fun fallbackServers(vararg serverList: String) {
        fallbackServers.addAll(serverList)
    }
    
    fun domains(vararg domainList: String) {
        domains.addAll(domainList)
    }
    
    fun searchDomains(vararg domainList: String) {
        searchDomains.addAll(domainList)
    }
    
    fun hosts(hostname: String, ip: String) {
        hostsFile[hostname] = ip
    }
    
    fun cloudflare() {
        servers("1.1.1.1", "1.0.0.1")
    }
    
    fun google() {
        servers("8.8.8.8", "8.8.4.4")
    }
    
    fun quad9() {
        servers("9.9.9.9", "149.112.112.112")
    }
    
    fun toDNS() = DNSConfig(
        servers = if (servers.isEmpty()) listOf("1.1.1.1", "8.8.8.8") else servers,
        fallbackServers = fallbackServers,
        domains = domains,
        searchDomains = searchDomains,
        dnsOverTls = dnsOverTls,
        dnsOverHttps = dnsOverHttps,
        dnssec = dnssec,
        cache = cache,
        resolver = resolver,
        hostsFile = hostsFile
    )
}

@HorizonOSDsl
class BridgeContext {
    var name: String = ""
    private val interfaces = mutableListOf<String>()
    var stp: Boolean = true
    var stpPriority: Int = 32768
    var helloTime: Duration = 2.seconds
    var forwardDelay: Duration = 15.seconds
    var maxAge: Duration = 20.seconds
    private var ipv4: IPv4Config? = null
    private var ipv6: IPv6Config? = null
    
    fun interfaces(vararg interfaceList: String) {
        interfaces.addAll(interfaceList)
    }
    
    fun ipv4(block: IPv4Context.() -> Unit) {
        ipv4 = IPv4Context().apply(block).toIPv4()
    }
    
    fun ipv6(block: IPv6Context.() -> Unit) {
        ipv6 = IPv6Context().apply(block).toIPv6()
    }
    
    fun toBridge() = NetworkBridge(
        name = name,
        interfaces = interfaces,
        stp = stp,
        stpPriority = stpPriority,
        helloTime = helloTime,
        forwardDelay = forwardDelay,
        maxAge = maxAge,
        ipv4 = ipv4,
        ipv6 = ipv6
    )
}

@HorizonOSDsl
class VLANContext {
    var name: String = ""
    var parentInterface: String = ""
    var vlanId: Int = 0
    private var ipv4: IPv4Config? = null
    private var ipv6: IPv6Config? = null
    var enabled: Boolean = true
    
    fun ipv4(block: IPv4Context.() -> Unit) {
        ipv4 = IPv4Context().apply(block).toIPv4()
    }
    
    fun ipv6(block: IPv6Context.() -> Unit) {
        ipv6 = IPv6Context().apply(block).toIPv6()
    }
    
    fun toVLAN() = VLANConfig(
        name = name,
        parentInterface = parentInterface,
        vlanId = vlanId,
        ipv4 = ipv4,
        ipv6 = ipv6,
        enabled = enabled
    )
}

@HorizonOSDsl
class ProxyContext {
    var httpProxy: String? = null
    var httpsProxy: String? = null
    var ftpProxy: String? = null
    var socksProxy: String? = null
    private val noProxy = mutableListOf<String>()
    var autoConfigUrl: String? = null
    
    fun noProxy(vararg hosts: String) {
        noProxy.addAll(hosts)
    }
    
    fun toProxy() = ProxyConfig(
        httpProxy = httpProxy,
        httpsProxy = httpsProxy,
        ftpProxy = ftpProxy,
        socksProxy = socksProxy,
        noProxy = noProxy,
        autoConfigUrl = autoConfigUrl
    )
}

// ===== Extension Functions =====

fun CompiledConfig.hasNetworking(): Boolean = network != null

fun CompiledConfig.getInterface(name: String): NetworkInterface? = 
    network?.interfaces?.find { it.name == name }

fun CompiledConfig.getWiFiNetwork(ssid: String): WiFiNetwork? = 
    network?.wifiNetworks?.find { it.ssid == ssid }

fun CompiledConfig.getVPN(name: String): VPNConnection? = 
    network?.vpnConnections?.find { it.name == name }