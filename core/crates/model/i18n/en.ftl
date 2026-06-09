# Tactus — English UI / speech strings (Fluent).
# Identifiers use '-'; dotted profile i18n keys (e.g. "param.tempo") are
# normalised to '-' ("param-tempo") before lookup.

kit-label = Kit { $number }: { $name }

param-tempo = { $value } BPM
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Kit { $number }
param-tempo-switch = Tempo switch: { $value }

instrument-name = { $name }
instrument-unknown = Instrument #{ $number } (unknown)

device-connected = Connected to { $device }, firmware { $firmware }.
device-firmware-untested = This firmware isn't in Tactus's tested list — it should work; please report any problems.
device-unrecognized = Connected to an unrecognised module. Some features may be unavailable.
