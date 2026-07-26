# Tactus — English UI / speech strings (Fluent).
# Identifiers use '-'; dotted profile i18n keys (e.g. "param.tempo") are
# normalised to '-' ("param-tempo") before lookup.

kit-label = Kit { $number }: { $name }
# The answer to "next/previous kit" at the end of the module's kit list.
kit-at-first = First kit.
kit-at-last = Last kit.

param-tempo = { $value } BPM
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Kit { $value }
param-tempo-switch = Tempo switch: { $value }
param-setlist-name = { $name }
param-setlist-step = Kit { $value }
# A raw value with a meaning of its own (see the profile's `sentinel`).
value-setlist-end = End of the set list

# Parameter labels (control / accessibility labels — never carry the value).
param-tempo-label = Tempo
param-kit-name-label = Kit name
param-kit-sub-name-label = Sub-name
param-kit-num-label = Kit
param-setlist-name-label = Set list name
param-setlist-step-label = Step
param-tempo-switch-label = Tempo switch

instrument-name = { $name }
instrument-unknown = Instrument #{ $number } (unknown)

edit-mismatch = Couldn't change it — it's still { $value }.
edit-timeout = No response — the value is unknown. Check the connection.
edit-out-of-range = That value is out of range.
edit-not-ready = Not connected to a device.

device-connected = Connected to { $device }, firmware { $firmware }.
device-firmware-untested = This firmware isn't in Tactus's tested list — it should work; please report any problems.
device-unrecognized = Connected to an unrecognised module. Some features may be unavailable.

# ── The app's own interface (ADR-0008: one source of phrasing per platform) ──
ui-section-connection = Connection
ui-label-status = Status
ui-label-device = Device
ui-label-firmware = Firmware
ui-status-disconnected = Disconnected
ui-status-identifying = Identifying…
ui-status-ready = Ready
ui-connect-prompt = Connect your drum module with a USB cable.
ui-firmware-newer = This firmware is newer than we've tested. Everything should still work.
ui-firmware-older = This firmware is older than we've tested. Everything should still work.
ui-firmware-unknown = This firmware hasn't been tested. Everything should still work.

ui-section-kit = Kit
ui-label-current-kit = Current kit
ui-value-current-kit = Current kit: { $value }
ui-button-previous-kit = Previous kit
ui-button-next-kit = Next kit
ui-button-rename-kit = Rename kit…
ui-hint-rename-kit = Edit the name of the current kit
ui-title-rename-kit = Rename kit
ui-label-kit-name = Kit name
ui-button-save = Save
ui-button-cancel = Cancel

ui-section-tempo = Tempo
ui-label-tempo = Tempo
ui-value-updating = Updating…
ui-hint-tempo-adjust = Swipe up or down to adjust the tempo
ui-value-unknown = —

ui-section-language = Language
ui-language-system = System

# ── Kit parameters ──
# An enum value is the module's own word (OFF, WARM HALL, SRV-2000): spoken
# verbatim and tagged as English, matching what the module and Roland's manual
# say. Only the *labels* below are ours to translate.
param-enum-value = { $value }

param-kit-volume = { $value } dB
param-kit-volume-label = Kit volume

param-unit-volume = { $value } dB
param-unit-volume-label = Pad volume
param-unit-overhead-send = { $value } dB
param-unit-overhead-send-label = Overhead send
param-unit-room-send = { $value } dB
param-unit-room-send-label = Room send
param-unit-reverb-send = { $value } dB
param-unit-reverb-send-label = Reverb send

param-layer-switch-label = Layer
param-layer-instrument = { $value }
param-layer-instrument-label = Instrument
param-layer-inst-bank = { $value }
param-layer-inst-bank-label = Instrument bank
param-layer-volume = { $value } dB
param-layer-volume-label = Layer volume
param-layer-pitch = { $value } cents
param-layer-pitch-label = Pitch
param-layer-decay = { $value }
param-layer-decay-label = Decay

param-pad-pan = { $value }
param-pad-pan-label = Pan

param-fx-type = { $value }
param-fx-type-label = Effect type
param-fx-switch-label = Effect

param-overhead-switch-label = Overhead mics
param-overhead-mic-type-label = Overhead mic type
param-overhead-level = { $value } dB
param-overhead-level-label = Overhead level

param-room-switch-label = Room
param-room-type-label = Room type
param-room-level = { $value } dB
param-room-level-label = Room level

param-reverb-switch-label = Reverb
param-reverb-type-label = Reverb type
param-reverb-level = { $value } dB
param-reverb-level-label = Reverb level
