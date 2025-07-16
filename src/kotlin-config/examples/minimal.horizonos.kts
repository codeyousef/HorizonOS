#!/usr/bin/env kotlin

/**
 * Minimal HorizonOS Configuration
 * 
 * This is the absolute minimum configuration required for a HorizonOS system.
 * It only sets the required system parameters without any additional packages,
 * services, or users.
 */

horizonOS {
    // Required system configuration
    hostname = "horizonos-minimal"
    timezone = "UTC"
    locale = "en_US.UTF-8"
}