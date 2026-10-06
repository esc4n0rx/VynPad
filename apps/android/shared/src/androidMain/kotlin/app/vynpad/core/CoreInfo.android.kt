package app.vynpad.core

import uniffi.vynpad_client_core.coreVersion
import uniffi.vynpad_client_core.protocolVersion

actual fun loadCoreInfo(): CoreInfo? = try {
    CoreInfo(version = coreVersion(), protocolVersion = protocolVersion().toInt())
} catch (_: UnsatisfiedLinkError) {
    null
}
