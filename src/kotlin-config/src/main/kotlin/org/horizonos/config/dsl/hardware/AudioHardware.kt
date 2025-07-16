package org.horizonos.config.dsl.hardware

import kotlinx.serialization.Serializable

// ===== Audio Configuration =====

@Serializable
data class AudioConfig(
    val server: AudioServer = AudioServer.PIPEWIRE,
    val devices: List<AudioDevice> = emptyList(),
    val profiles: List<AudioProfile> = emptyList(),
    val pulseaudio: PulseAudioConfig = PulseAudioConfig(),
    val alsa: ALSAConfig = ALSAConfig(),
    val jack: JACKConfig = JACKConfig(),
    val bluetooth: BluetoothAudioConfig = BluetoothAudioConfig()
)

@Serializable
data class AudioDevice(
    val name: String,
    val description: String? = null,
    val enabled: Boolean = true,
    val volume: Double = 1.0,
    val muted: Boolean = false,
    val channels: Int = 2,
    val sampleRate: Int = 44100,
    val bitDepth: Int = 16
)

@Serializable
data class AudioProfile(
    val name: String,
    val description: String? = null,
    val devices: List<String> = emptyList(),
    val active: Boolean = false
)

@Serializable
data class PulseAudioConfig(
    val enabled: Boolean = true,
    val sampleRate: Int = 44100,
    val sampleFormat: String = "s16le",
    val channels: Int = 2,
    val fragments: Int = 4,
    val fragmentSizeMs: Int = 25,
    val enableLFE: Boolean = false,
    val enableSurround: Boolean = true,
    val modules: List<String> = emptyList()
)

@Serializable
data class ALSAConfig(
    val enabled: Boolean = true,
    val cardOrder: List<String> = emptyList(),
    val mixerControls: Map<String, String> = emptyMap(),
    val powerSave: Boolean = true,
    val powerSaveController: Boolean = true
)

@Serializable
data class JACKConfig(
    val enabled: Boolean = false,
    val sampleRate: Int = 48000,
    val bufferSize: Int = 1024,
    val periods: Int = 2,
    val priority: Int = 70,
    val realtimeScheduling: Boolean = true
)

@Serializable
data class BluetoothAudioConfig(
    val enabled: Boolean = true,
    val codecs: List<BluetoothCodec> = listOf(BluetoothCodec.SBC, BluetoothCodec.AAC),
    val quality: BluetoothQuality = BluetoothQuality.HIGH,
    val latency: BluetoothLatency = BluetoothLatency.NORMAL
)

// ===== Enums =====

@Serializable
enum class AudioServer {
    PULSEAUDIO,    // PulseAudio server
    PIPEWIRE,      // PipeWire server
    JACK,          // JACK Audio Connection Kit
    ALSA           // Advanced Linux Sound Architecture
}

@Serializable
enum class BluetoothCodec {
    SBC,           // Sub-band Coding
    AAC,           // Advanced Audio Coding
    APTX,          // aptX codec
    APTX_HD,       // aptX HD codec
    LDAC,          // LDAC codec
    LC3            // Low Complexity Communication Codec
}

@Serializable
enum class BluetoothQuality {
    LOW,           // Low quality/high compression
    NORMAL,        // Normal quality
    HIGH,          // High quality/low compression
    LOSSLESS       // Lossless quality
}

@Serializable
enum class BluetoothLatency {
    LOW,           // Low latency mode
    NORMAL,        // Normal latency
    HIGH           // High latency mode
}