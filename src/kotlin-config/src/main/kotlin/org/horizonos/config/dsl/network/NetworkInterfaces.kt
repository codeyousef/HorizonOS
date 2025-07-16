package org.horizonos.config.dsl.network

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

@Serializable
enum class NetworkManagerType {
    NETWORKMANAGER,
    SYSTEMD_NETWORKD,
    NETCTL,
    WICD
}

// ===== Network Interface Configuration =====

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

// ===== Bridge Configuration =====

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

// ===== VLAN Configuration =====

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

// ===== Data Classes =====

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