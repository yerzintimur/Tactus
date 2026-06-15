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
