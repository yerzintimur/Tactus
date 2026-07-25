# Tactus — English UI / speech strings (Fluent).
# Identifiers use '-'; dotted profile i18n keys (e.g. "param.tempo") are
# normalised to '-' ("param-tempo") before lookup.

kit-label = Kit { $number }: { $name }

param-tempo = { $value } BPM
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Kit { $number }
param-tempo-switch = Tempo switch: { $value }

# Parameter labels (control / accessibility labels — never carry the value).
param-tempo-label = Tempo
param-kit-name-label = Kit name
param-kit-sub-name-label = Sub-name
param-kit-num-label = Kit
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
