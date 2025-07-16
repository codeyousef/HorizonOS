package org.horizonos.config.dsl.network

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== WiFi Network Configuration =====

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

// ===== VPN Configuration =====

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

// ===== DNS Configuration =====

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

// ===== Proxy Configuration =====

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

// ===== Enums =====

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
enum class DNSResolver {
    SYSTEMD_RESOLVED,
    DNSMASQ,
    BIND,
    UNBOUND,
    PIHOLE
}


// ===== Data Classes =====

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
data class ProxyConfig(
    val httpProxy: String? = null,
    val httpsProxy: String? = null,
    val ftpProxy: String? = null,
    val socksProxy: String? = null,
    val noProxy: List<String> = emptyList(),
    val autoConfigUrl: String? = null
)