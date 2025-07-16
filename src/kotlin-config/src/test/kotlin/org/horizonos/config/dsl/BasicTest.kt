package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.booleans.shouldBeTrue
import org.horizonos.config.dsl.network.InterfaceType

class BasicTest : StringSpec({
    
    "should create basic configuration with new DSL structure" {
        val config = horizonOS {
            hostname = "test-host"
            timezone = "UTC"
            locale = "en_US.UTF-8"
            
            network {
                hostname("test-network")
                networkInterface("eth0") {
                    type = InterfaceType.ETHERNET
                }
            }
            
            security {
                ssh {
                    port = 22
                }
            }
            
            hardware {
                gpu {
                }
            }
        }
        
        config.system.hostname shouldBe "test-host"
        config.system.timezone shouldBe "UTC"
        config.system.locale shouldBe "en_US.UTF-8"
        
        config.network shouldNotBe null
        config.network!!.hostname shouldBe "test-network"
        config.network!!.interfaces.size shouldBe 1
        
        config.security shouldNotBe null
        config.hardware shouldNotBe null
    }
})