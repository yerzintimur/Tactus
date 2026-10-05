# Tactus — English UI / speech strings (Fluent).
# Identifiers use '-'; dotted profile i18n keys (e.g. "param.tempo") are
# normalised to '-' ("param-tempo") before lookup.

kit-label = Kit { $number }: { $name }
# The answer to "next/previous kit" at the end of the module's kit list.
kit-at-first = First kit.
kit-at-last = Last kit.
setlist-step-kit = Step { $step }, Kit { $number }: { $name }
setlist-at-first = First step of the set list.
setlist-at-last = Last step of the set list.
setlist-empty = The set list is empty.

param-tempo = { $value } BPM
param-kit-name = { $name }
param-kit-sub-name = { $name }
param-kit-num = Kit { $value }
param-tempo-switch = Tempo switch: { $value }
param-setlist-name = { $name }
param-setlist-step = Kit { $value }
# Raw values with a meaning of their own (see the profile's `sentinel`).
value-setlist-end = End of the set list
# A level at its floor: the module shows -INF, and nothing is heard.
value-level-silent = Silent

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
# A bank we have no catalog for — an expansion pack not yet catalogued.
instrument-unknown-bank = Instrument #{ $number } in bank { $bank } (unknown)

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
# Module setup: the switches only the drummer can flip on the module itself,
# listed at the foot of the main screen until each is seen flipped. Transmit
# Edit Data is not in the address map; the value is the module's own menu
# path, as the profile gives it.
ui-section-setup = Module setup
ui-hint-transmit-edit-data = Turn on Transmit Edit Data on the module ({ $value }) to hear what you change on its panel.

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

ui-section-setlist = Set lists
ui-value-setlist-number = Set list { $value }
ui-label-setlist-name = Set list name
ui-value-setlist-step = Step { $value }
ui-value-setlist-empty = No kits in this set list yet
ui-hint-setlist = Read and rearrange the kits of a set list
ui-button-add-current-kit = Add the current kit
ui-button-move-step-up = Move up
ui-button-move-step-down = Move down
ui-button-remove-step = Remove
ui-button-rename-setlist = Rename set list…
ui-title-rename-setlist = Rename set list
ui-button-previous-step = Previous step
ui-button-next-step = Next step
ui-value-setlist-current-step = { $value }, current step

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

# Names of the positions a parameter repeats over (the profile's `dimensions`).
# Spoken before the parameter: "Snare rim, Layer A, Layer volume: 0.5 dB".
# Keys follow Roland's Data List, which calls every pad's zones HEAD/RIM (EDGE
# and BELL on the ride); the spoken names are what drummers call them, and what
# Roland's own prose calls them ("hi-hat bow", "ride edge").
pad-kick = Kick
pad-snare = Snare
pad-snare-head = Snare head
pad-snare-rim = Snare rim
pad-tom1 = Tom 1
pad-tom1-head = Tom 1 head
pad-tom1-rim = Tom 1 rim
pad-tom2 = Tom 2
pad-tom2-head = Tom 2 head
pad-tom2-rim = Tom 2 rim
pad-tom3 = Tom 3
pad-tom3-head = Tom 3 head
pad-tom3-rim = Tom 3 rim
pad-tom4 = Tom 4
pad-tom4-head = Tom 4 head
pad-tom4-rim = Tom 4 rim
pad-hihat = Hi-hat
pad-hihat-head = Hi-hat bow
pad-hihat-rim = Hi-hat edge
pad-crash1 = Crash 1
pad-crash1-head = Crash 1 bow
pad-crash1-rim = Crash 1 edge
pad-crash2 = Crash 2
pad-crash2-head = Crash 2 bow
pad-crash2-rim = Crash 2 edge
pad-ride = Ride
pad-ride-head = Ride bow
pad-ride-edge = Ride edge
pad-ride-bell = Ride bell
# The module labels its first aux input AUX, the others AUX2–AUX4.
pad-aux = Aux
pad-aux-head = Aux head
pad-aux-rim = Aux rim
pad-aux2 = Aux 2
pad-aux2-head = Aux 2 head
pad-aux2-rim = Aux 2 rim
pad-aux3 = Aux 3
pad-aux3-head = Aux 3 head
pad-aux3-rim = Aux 3 rim
pad-aux4 = Aux 4
pad-aux4-head = Aux 4 head
pad-aux4-rim = Aux 4 rim

layer-a = Layer A
layer-b = Layer B
layer-c = Layer C

# Kit FX slots: two per bus, four buses (BUS-A FX1 … BUS-D FX2).
fx-bus-a-1 = Bus A effect 1
fx-bus-a-2 = Bus A effect 2
fx-bus-b-1 = Bus B effect 1
fx-bus-b-2 = Bus B effect 2
fx-bus-c-1 = Bus C effect 1
fx-bus-c-2 = Bus C effect 2
fx-bus-d-1 = Bus D effect 1
fx-bus-d-2 = Bus D effect 2

# A position that has only a number (1-based, as on the module's screen).
dim-step = Step { $number }
