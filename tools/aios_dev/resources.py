"""Conservative host reserves and fresh-VM defaults from measured resources."""
import shutil
from .errors import DevctlError, ExitCode

GIB = 1024**3
HOST_BUILD_RESERVE_BYTES = 8 * GIB


def require_build_headroom(config):
    """Refuse new guest work before publication; sparse disks still use host space.

    This is a measured preflight floor, not a quota or a concurrency reservation.
    It never cleans files and does not block status/cancellation/evidence reads.
    Guest-side build reserves remain independently required.
    """
    try:
        free = shutil.disk_usage(config.root).free
    except OSError as error:
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "HOST_STORAGE_DISCOVERY_FAILED",
                          "Cannot measure host storage for the guest workspace") from error
    if type(free) is not int or free < HOST_BUILD_RESERVE_BYTES:
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "HOST_RECOVERY_RESERVE_UNAVAILABLE",
                          "New guest work requires at least 8 GiB free on its host storage filesystem",
                          details={"workspace": str(config.root), "free_bytes": free,
                                   "required_free_bytes": HOST_BUILD_RESERVE_BYTES})
    return {"free_bytes": free, "required_free_bytes": HOST_BUILD_RESERVE_BYTES}


def recommend(report):
    cpu = report.get("logical_cpus")
    available = report.get("memory", {}).get("available_bytes")
    free = report.get("disk", {}).get("free_bytes")
    if any(type(value) is not int or value <= 0 for value in (cpu, available, free)):
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "RESOURCE_DISCOVERY_REQUIRED", "Fresh VM sizing requires measured CPUs, available RAM and free storage")
    memory_reserve = max(2 * GIB, available // 4)
    memory_gib = min(16, (available - memory_reserve) // GIB)
    disk_gib = min(96, (free - 8 * GIB) // GIB)
    if memory_gib < 4 or disk_gib < 48:
        raise DevctlError(ExitCode.UNMET_PREREQUISITE, "INSUFFICIENT_DEFAULT_VM_RESOURCES", "Fresh development defaults require at least 4 GiB guest RAM and 48 GiB disk after host reserves")
    return {"vcpus": min(8, max(1, cpu // 2)), "memory_mib": memory_gib * 1024,
            "disk_gib": disk_gib, "host_memory_reserve_bytes": memory_reserve,
            "host_disk_reserve_bytes": 8 * GIB}
