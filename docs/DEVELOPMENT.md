# Спецификация разработки (детальная)

> **Язык документа:** русский (рабочая спека для мейнтейнера). Временный — перед
> публичным релизом будет переведён на английский или удалён. Технические термины,
> идентификаторы кода, имена файлов и зависимостей — в оригинале (English). См.
> языковую политику в [AGENTS.md](../AGENTS.md).
>
> **Статус:** черновик архитектуры. Это «как строим». Что и зачем —
> [SPEC.md](SPEC.md); доступность — [ACCESSIBILITY.md](ACCESSIBILITY.md); протокол —
> [PROTOCOL.md](PROTOCOL.md).
>
> **Главные принципы, которые пронизывают всё ниже:** Nonvisual-first
> ([North Star](SPEC.md#-north-star--nonvisual-first)) и мульти-девайсность через
> профили устройств. Если что-то зависит от конкретного модуля — это **данные**
> (profile), а не код.

---

## 1. Требования

### 1.1 Функциональные
- Подключение к модулю по **USB-C MIDI** (primary) и **BLE-MIDI** (secondary).
- **Авто-детект модели** по Identity Reply → выбор профиля устройства.
- Озвучка состояния: текущий кит (номер+имя), темп; live при изменении.
- Переключение кита из доступного списка.
- Редактирование: глобальные настройки, триггеры/чувствительность, полный
  редактор кита (инструменты по пэдам/слоям, pitch/decay/transient, volume/pan,
  pad EQ/comp, sends), FX (типы+пресеты), ambience.
- Каждая правка: **write → readback → verify → озвучить фактическое значение**.
- Live-озвучка правок, сделанных на железе (Transmit Edit Data).
- **Мультиязычность** UI и озвучки.
- **Мульти-девайс**: V31 сейчас; V51/V71 и будущие модули — добавлением профиля.

### 1.2 Нефункциональные
- **Доступность — это product, а не фича.** Полная работа «вслепую» (см. §9).
- **Латентность:** анонс смены кита < ~300 мс; цикл правки (write→readback→speak)
  < ~500 мс по USB-C.
- **Надёжность:** нет «тихих» сбоев; любое действие завершается произнесённым
  успехом или произнесённой, действенной ошибкой. Авто-reconnect.
- **Offline-first:** ноль сети для основной работы; on-device TTS; без телеметрии
  без явного opt-in.
- **Расширяемость:** новый модуль не должен требовать изменений в `core`/`apps`.
- **Тестируемость:** вся опасная логика — в чистом, детерминированном core.
- **Размер бинаря** под контролем (release profile, ограничение ABI).

### 1.3 Ограничения
- Один разработчик, новичок в мобильной разработке → ставка на **общий core** и
  **тонкие нативные слои**, плюс максимально стандартные инструменты.
- На руках только модуль **V31** → всё, что нельзя проверить на железе (напр.
  персистентность, точные адреса других моделей), помечается как «verify on HW».
- Документы Roland © Roland → в репозиторий не коммитятся (см.
  [ADR-0004](adr/0004-vendor-docs-not-committed.md)).
- Open-source, Apache-2.0.

---

## 2. Высокоуровневая архитектура

```
┌───────────────────────────────┐     ┌───────────────────────────────┐
│  iOS app (Swift / SwiftUI)    │     │ Android app (Kotlin / Compose)│
│  • Accessible UI (VoiceOver)  │     │ • Accessible UI (TalkBack)    │
│  • CoreMIDI transport         │     │ • android.media.midi transport│
│  • AVSpeechSynthesizer + earc.│     │ • TextToSpeech + earcons      │
│  • Timer/tick, haptics        │     │ • Timer/tick, haptics         │
└───────────────┬───────────────┘     └───────────────┬───────────────┘
   реализует MidiSender / SessionListener / Clock (callback interfaces)
                │   UniFFI (Swift / Kotlin bindings)   │
        ┌───────┴──────────────────────────────────────┴───────┐
        │                  Rust core — SANS-I/O                  │
        │  ┌───────┐ ┌────────┐ ┌────────┐ ┌─────────┐ ┌──────┐ │
        │  │ sysex │ │ device │ │ model  │ │ engine  │ │ ffi  │ │
        │  └───────┘ └────────┘ └────────┘ └─────────┘ └──────┘ │
        │   codec    profiles    domain    session-FSM  uniffi  │
        └───────────────────────┬───────────────────────────────┘
                  embedded data: profiles/*.json + Fluent *.ftl
```

**Ключевая идея потоков:** core ничего не делает «сам». Нативный слой кормит его
событиями (`handle_midi_input(bytes)`, `tick(now)`, интенты пользователя), а core
возвращает/эмитит **действия** (отправь эти MIDI-байты; произнеси этот
локализованный текст с приоритетом; запланируй tick через N мс; обнови
view-model). Это паттерн **sans-I/O** — core детерминирован и полностью
юнит-тестируем без железа, потоков и таймеров.

См. [ADR-0008](adr/0008-sans-io-core-and-i18n.md).

---

## 3. Мульти-девайс: профили устройств

Самая важная архитектурная развилка. **Механика протокола одинакова у всех модулей
Roland; различаются данные.** Поэтому:

- **`sysex` (код)** — инвариант: RQ1/DT1, checksum, 4-байтная 7-битная адресная
  арифметика, кодировки (nibble/signed/ASCII), сборка фрагментированного SysEx.
  Не знает ни про один конкретный модуль.
- **`DeviceProfile` (данные)** — всё, что специфично для модели.

### 3.1 Схема профиля (JSON, версионируемая)

```jsonc
{
  "schema_version": 1,
  "profile_id": "roland-v31",
  "display_name": "Roland V31",
  "family": "roland-vdrums",
  "model_id": [1, 6, 1],              // SysEx Model ID — ключ авто-детекта
  "device_id_default": 16,            // 0x10
  "source": "V31 MIDI Implementation v2.00 (2025-11-11); V31 Data List eng02",
  "firmware": { "min": null, "max": null },

  "capabilities": {
    "kit_count": 200,
    "fx_slots": 4,
    "pads": [ /* машинно-читаемый layout пэдов/зон, индексы как в PROTOCOL */ ],
    "features": ["transmit_edit_data", "ambience", "bus_fx", "set_lists"]
  },

  "areas": {
    "current": { "address": [0,0,0,0] },
    "setup":   { "address": [1,0,0,0] },
    "kit":     { "address": [4,0,0,0], "stride": [0,4,0,0], "count": 200 }
    // ...
  },

  "parameters": [
    {
      "id": "kit.common.name",
      "area": "kit",
      "offset": [0,0],                // внутри области/юнита
      "len": 16,
      "encoding": "ascii",
      "i18n_key": "param.kit_name"
    },
    {
      "id": "kit.common.tempo",
      "area": "kit",
      "offset": [0,111],              // 0x00 0x6F
      "len": 2,
      "encoding": { "nibble": { "div": 10 } },
      "range": { "min": 200, "max": 2600 },
      "unit": "bpm",
      "i18n_key": "param.tempo"
    }
    // ...
  ],

  "catalogs": {
    "instruments": "catalogs/roland-v31/instruments.json",  // No↔name↔group
    "fx_types":    "catalogs/roland-v31/fx.json",
    "ambience":    "catalogs/roland-v31/ambience.json"
  }
}
```

### 3.2 Реестр и авто-детект
- `ProfileRegistry` держит **встроенные** профили (compiled-in) и умеет принимать
  **догружаемые** профили (downloadable «profile packs») для будущих модулей и
  расширений каталога без обновления приложения.
- Поток детекта: connect → **Identity Request** → Identity Reply содержит Model ID
  → `registry.match(model_id, firmware?)` → `DeviceProfile`.
- **Неизвестный модуль** (нет профиля): не падаем. Включаем **generic/degraded
  mode** — то, что работает на инвариантной механике (напр., чтение `Current`,
  если адрес совпадает; явное «модуль не распознан»), и предлагаем
  «contribute a profile». Capabilities = минимальные.
- Версионирование: `schema_version` для миграций; `firmware` диапазон, если
  адресная карта менялась между прошивками.

### 3.3 Принцип
В `core` (кроме данных профиля), `apps/*`, `ffi` **не должно быть** хардкода V31.
Зависит от модели → в профиль. Это и есть «архитектурно верно, а не заточено под
V31».

---

## 4. Rust core: крейты

Воркспейс `core/crates/*`. Все — без I/O.

### 4.1 `sysex` (device-agnostic)
- `build_rq1(dev, model_id, addr[4], size[4]) -> Vec<u8>`,
  `build_dt1(dev, model_id, addr[4], data) -> Vec<u8>`,
  `build_identity_request(dev)`.
- `parse(bytes) -> SysexMessage` (DT1 / IdentityReply / unknown), с проверкой
  checksum.
- `roland_checksum(addr_and_data) -> u8`.
- Адресная арифметика: сложение base+offset с переносом по 7-битным байтам.
- Кодировки `encode/decode`: `plain7`, `nibble{div}`, `signed1`, `signed2`,
  `ascii`.
- Реассемблер фрагментированного SysEx (вход — поток байт, выход — целые
  сообщения).
- **Покрыто golden-векторами** из [PROTOCOL.md](PROTOCOL.md) §3 + property-tests
  (proptest) + fuzz (cargo-fuzz). Никогда не паникует на мусоре.

### 4.2 `device`
- `DeviceProfile`, `Capabilities`, `ParameterDef`, `AreaDef`, `Encoding`.
- `ProfileRegistry`: `embedded()`, `register(profile)`, `match(model_id, fw)`.
- Загрузка встроенных профилей (`include_str!`/`rust-embed` → парс в типы).
- `address_of(profile, param_id, indices) -> [u8;4]` — резолв адреса параметра
  (область + stride кита/пэда/слоя + offset).

### 4.3 `model`
- Типизированные `Parameter`, `ParameterArea`, агрегаты `Kit`, `PadUnit`, `Layer`,
  `Fx`, `Ambience`.
- Интенты (команды): `SelectKit`, `SetParameter{param_id, indices, value}`,
  `RenameKit`, `RefreshAll` и т.п.
- **value ⇄ человекочитаемое**: не возвращает готовую строку на конкретном языке,
  а формирует **локализуемый дескриптор** `Message{ id, args }` (напр.
  `param.tempo` + `{value: 120.0}`). Рендер в строку — через Fluent (§8), с учётом
  текущей локали, множественных чисел и формата чисел. Это даёт мультиязычность и
  единый, тестируемый источник фраз.

### 4.4 `engine` (sans-I/O session FSM)
- **Connection FSM:** `Disconnected → Connecting → Identifying → Ready → Error`,
  авто-reconnect.
- **Детект устройства:** Identity handshake → выбор профиля (§3.2).
- **Polling scheduler:** опрос `Current` 2–4 Гц (Program Change ненадёжен, см.
  PROTOCOL §6); backoff в простое. Реализован через эмит `ScheduleTick`.
- **Edit pipeline:** FSM `write→readback→verify` (SPEC §10), сериализация по
  параметру, debounce быстрых правок.
- **Inbound:** парс pushed DT1 (Transmit Edit Data) → апдейт модели → события.
- **Event bus:** типизированные `CoreEvent` наружу (см. §7).
- Вход: `on_connected`, `on_disconnected`, `handle_midi_input`, `tick(now)`,
  интенты. Выход: `Action`(SendMidi/Speak/Earcon/ScheduleTick/UpdateViewModel) —
  доставляются нативу через listener/sender (§7).

### 4.5 `ffi`
- Публичная поверхность через `#[uniffi::export]` (см. §7).
- `crate-type = ["cdylib", "staticlib", "lib"]` (cdylib → Android `.so`,
  staticlib → iOS XCFramework).

---

## 5. Доменная модель и кодировки (резюме)

Подробности адресов/кодировок — в [PROTOCOL.md](PROTOCOL.md). Ключевое для кода:

- Адрес — 4 байта по 7 бит. Кит k: `kit_base + k*stride` (из профиля).
- Кодировки: `plain7` (aa·128+bb), `nibble` (по 4 бита на байт, для KitNum/tempo),
  `signed1` (−64..+63), `signed2` (−8192..+8191), `ascii`.
- Темп V31: `kit.common.tempo`, offset `00 6F`, len 2, nibble, /10 → 20.0–260.0
  BPM (в стартовом брифе был ошибочно `00 6D`; проверить на HW).
- Имя кита: 16 ASCII-байт, offset 0 в `KitCommon`.
- `Current.KitNum`: nibble-packed, 0–199 (display 1–200) — основной источник
  «какой кит активен», поллим.

---

## 6. FFI-контракт (UniFFI)

UniFFI **0.31.x**, proc-macro в **library mode** (UDL только для пробелов).
`uniffi` и `uniffi-bindgen` держим в **одной версии**. Async поддерживается
(маппится в Swift `async` / Kotlin `suspend`), но базовый дизайн — синхронный
sans-I/O + callbacks (детерминированнее и проще тестировать).

### 6.1 Эскиз API

```rust
// ── Данные (Records / Enums) ──
#[derive(uniffi::Enum)]
pub enum ConnectionState { Disconnected, Connecting, Identifying, Ready, Error { message: LocalizedText } }

#[derive(uniffi::Record)]
pub struct DeviceInfo { pub model_id: Vec<u8>, pub name: String, pub firmware: Option<String>, pub profile_id: String, pub recognized: bool }

#[derive(uniffi::Record)]
pub struct KitRef { pub number: u32, pub name: String }

#[derive(uniffi::Record)]
pub struct LocalizedText { pub key: String, pub text: String } // text уже отрендерен core под текущую локаль

#[derive(uniffi::Enum)]
pub enum SpeechPriority { Low, Default, High } // маппинг на платформенные приоритеты анонсов

#[derive(uniffi::Enum)]
pub enum CoreEvent {
    ConnectionChanged { state: ConnectionState },
    DeviceIdentified { device: DeviceInfo },
    CurrentKitChanged { kit: KitRef },
    ParameterChanged { param_id: String, value: Option<f64>, display: LocalizedText },
    EditConfirmed { param_id: String, display: LocalizedText },
    EditFailed { param_id: String, reason: LocalizedText },
    Speak { text: LocalizedText, priority: SpeechPriority },
    Earcon { id: String },
}

// ── Callback interfaces (реализует натив) ──
#[uniffi::export(callback_interface)]
pub trait MidiSender { fn send(&self, bytes: Vec<u8>); }

#[uniffi::export(callback_interface)]
pub trait SessionListener { fn on_event(&self, event: CoreEvent); }

// ── Объект сессии ──
#[derive(uniffi::Object)]
pub struct Session { /* Mutex<EngineState> */ }

#[uniffi::export]
impl Session {
    #[uniffi::constructor]
    pub fn new(midi: Box<dyn MidiSender>, listener: Box<dyn SessionListener>, locale: String) -> Arc<Session>;

    pub fn set_locale(&self, locale: String);

    // транспортные события от натива
    pub fn on_connected(&self);
    pub fn on_disconnected(&self);
    pub fn handle_midi_input(&self, bytes: Vec<u8>); // батчами, не по байту (важно для Android/JNA)
    pub fn tick(&self, now_millis: u64);             // двигает таймеры/polling

    // интенты пользователя
    pub fn select_kit(&self, number: u32);
    pub fn set_parameter(&self, param_id: String, indices: Vec<u32>, value: f64);
    pub fn rename_kit(&self, number: u32, name: String);
    pub fn request_refresh(&self);

    // запросы view-model / каталогов
    pub fn snapshot(&self) -> ViewModelSnapshot;
    pub fn list_instruments(&self, pad_index: u32, layer: u32) -> Vec<CatalogEntry>;
}
```

### 6.2 Замечания по контракту
- **Потоки:** методы безопасны с main-thread натива; внутри `Mutex`. События
  доставляются на известном потоке (документируем; на Android — не на RT-потоке
  MIDI).
- **Паники:** публичные методы не паникуют; ошибки — через `Result`/throwing
  (UniFFI: паника в non-throwing → fatal). Дизайним API на `Result`.
- **Android/JNA hot-path:** `handle_midi_input` принимает буфер, не байт —
  минимизируем число FFI-вызовов (JNA attach/detach на вызов).
- **Версионирование биндингов:** checksum-контракт UniFFI проверяется на init;
  биндинги регенерируем в CI, не коммитим устаревшие.

---

## 7. Поток данных и состояния

### 7.1 Подключение и детект
```
native: open MIDI port → session.on_connected()
engine: → Connecting → emit SendMidi(IdentityRequest), ScheduleTick(timeout)
native: bytes → session.handle_midi_input(IdentityReply)
engine: parse → match profile → Identifying→Ready
        → emit DeviceIdentified, Speak("Connected to {name}", High), Earcon(connected)
        → start polling Current (ScheduleTick)
```

### 7.2 Правка (write→readback→verify)
```
native: session.set_parameter("kit.common.tempo", [k], 120.0)
engine: encode → emit SendMidi(DT1 write)
        → emit SendMidi(RQ1 read same addr)         // readback
        → state Verifying
native: bytes(DT1 reply) → handle_midi_input
engine: decode actual a
        a == v → emit EditConfirmed + Speak(display(a), Default) + Earcon(ok)
        a != v → emit EditFailed + Speak("still {a}", High)
        timeout(tick) → emit EditFailed + Speak("no response", High)
```

### 7.3 Live-правка на железе
```
native: bytes(pushed DT1) → handle_midi_input  // Transmit Edit Data = ON
engine: parse addr+value → resolve param via profile → update model
        → emit ParameterChanged + Speak(display, Low)  // Low: не перебивать VO
```

---

## 8. i18n / l10n

Цель: мультиязычные UI и **озвучка**, при этом фразы озвучки — единый,
тестируемый источник (SPEC: «все строки речи из core»).

- **Озвучка и значения параметров → Fluent в core.** Библиотека `fluent`
  (`fluent-bundle`) + `.ftl`-каталоги на локаль, встроены в core. `model` отдаёт
  `Message{id,args}`; `engine`/`ffi` рендерит в `LocalizedText.text` под текущую
  локаль (учёт plural rules и формата чисел). Переводчики правят `.ftl` (хороший
  тулинг).
- **Чистые UI-строки чрома** (заголовки экранов, кнопки, не связанные с данными
  модуля) — нативные ресурсы: Android **string resources/plurals**, iOS **String
  Catalogs (.xcstrings)**. Стандартный тулинг перевода на каждой платформе.
- **Выбор языка:** по умолчанию — локаль устройства; override в настройках
  (`session.set_locale`).
- **Доступность:** язык озвучки = язык приложения; голос/скорость TTS — системные
  (уважаем выбор пользователя). RTL (ar/he) учитываем в визуальном слое, хотя он
  вторичен.

Компромисс (Fluent-в-core vs только нативные ресурсы) — см. §15 и
[ADR-0008](adr/0008-sans-io-core-and-i18n.md).

---

## 9. Доступность: маппинг на платформенные API

Принципы — в [ACCESSIBILITY.md](ACCESSIBILITY.md). Здесь — конкретные API
(проверено по докам, mid-2026).

### 9.1 iOS (SwiftUI, VoiceOver)
- Label/Value/Hint: `.accessibilityLabel/Value/Hint`; динамика — в **value**.
- Регулируемые значения (темп/громкость свайпом ↑↓): **`.accessibilityAdjustableAction`**.
- Анонсы динамики: **`AccessibilityNotification.Announcement`** +
  **`.accessibilitySpeechAnnouncementPriority(.high/.default/.low)`** (iOS 17+),
  приоритет через `AttributedString`.
- Детект VoiceOver: `UIAccessibility.isVoiceOverRunning` /
  `@Environment(\.accessibilityVoiceOverEnabled)`.
- Фокус: `@AccessibilityFocusState` + `.accessibilityFocused`.
- Rotor: `.accessibilityRotor` (напр. ротор «пэды», «устройства»).

### 9.2 Android (Compose, TalkBack)
- `Modifier.semantics { contentDescription; stateDescription; role; heading() }`.
- Регулируемые значения: `progressBarRangeInfo = ProgressBarRangeInfo(...)` +
  action `setProgress` (у `Slider` уже зашито).
- Анонсы: `liveRegion = LiveRegionMode.Polite/Assertive` (известный баг — иногда
  нужен `contentDescription` в том же блоке); imperative —
  `View.announceForAccessibility` (для транзиентных событий).
- Кастом-экшены: `customActions = listOf(CustomAccessibilityAction(...))`.
- Порядок обхода: `isTraversalGroup`, `traversalIndex`.
- Детект: `AccessibilityManager.isEnabled` / `isTouchExplorationEnabled`.

### 9.3 Речевой роутинг (критично — без двойной речи)
Единая абстракция **Speech Output** с двумя бэкендами, выбор в рантайме по «включён
ли screen reader»:
- **Screen reader ВКЛ** → маршрутизируем `Speak` через **системный анонс** (iOS
  `AccessibilityNotification.Announcement` с приоритетом; Android liveRegion или
  свой TTS с аккуратным аудиофокусом) — нет наложения на VoiceOver/TalkBack,
  уважается голос/скорость пользователя.
- **Screen reader ВЫКЛ** → собственный TTS (AVSpeechSynthesizer / TextToSpeech)
  с очередью.
- `SpeechPriority` из core (`Low/Default/High`) маппится на приоритеты анонсов:
  High («Connected to V31») перебивает; Low (эхо live-правок) — в очередь.
- **Earcons** (короткие тоны) — всегда, до речи; **haptics** — третий канал.

---

## 10. Платформенные слои

### 10.1 iOS
- **Xcode 26.5 / Swift 6.3**, Swift 6 mode (strict concurrency — аккуратно с
  CoreMIDI read-block на RT-потоке: хоп в actor/`@MainActor`). **min iOS 18**
  (баланс охвата и API; можно поднять до 26 ради новейших API без `#available`).
- **MIDI:** CoreMIDI, современный путь — `MIDIInputPortCreateWithProtocol` +
  **MIDIEventList/UMP**, `MIDISendEventList`; SysEx крупный — фрагментируется,
  реассемблить (это делает `sysex` в core). USB-C class-compliant — без
  entitlement. **BLE:** `CABTMIDICentralViewController` (нет в Simulator!),
  **Info.plist `NSBluetoothAlwaysUsageDescription`** обязателен.
- **TTS:** AVSpeechSynthesizer (очередь FIFO, rate/voice; при VO-ON — предпочесть
  анонсы; AVAudioSession `.duckOthers`, деактивировать в `didFinish`).
- **Тесты:** Swift Testing (юниты) + XCUITest **`performAccessibilityAudit`** на
  каждом экране в CI + ручной VoiceOver.
- **Core:** XCFramework + UniFFI Swift через **SPM `binaryTarget`** + wrapper
  target; имя `module.modulemap` строго; чек-сумма XCFramework автоматизируется.

### 10.2 Android
- **Kotlin 2.4.0, AGP 9.2.0, Gradle 9.5.1, Compose BOM 2026.06.00**,
  compileSdk/targetSdk **36**, **minSdk 26**. (Play: с 31.08.2026 новые апдейты —
  target 36.)
- **MIDI:** `android.media.midi` (`MidiManager.openDevice`/`openBluetoothDevice`,
  `registerDeviceCallback`), USB host + BLE; MIDI 2.0/UMP с API 33. SysEx — сырой
  поток байт через `MidiReceiver.send`, реассемблить. Манифест: `uses-feature
  android.software.midi`, `usb.host`, `bluetooth_le` (required=false). BLE-права:
  `BLUETOOTH_SCAN`(+`neverForLocation`)/`BLUETOOTH_CONNECT`.
- **TTS:** TextToSpeech (`QUEUE_FLUSH` для актуальности, `UtteranceProgressListener`,
  `AudioAttributes USAGE_ASSISTANCE_ACCESSIBILITY`, аудиофокус
  `MAY_DUCK`).
- **Тесты:** Compose semantics-assertions + **ATF в Compose-тестах**
  (`enableAccessibilityChecks()` с Compose 1.8+) + Espresso `AccessibilityChecks`
  + ручной TalkBack + Accessibility Scanner.
- **Core:** UniFFI Kotlin (**JNA direct mapping**, JNA ≥ 5.12.0 `@aar`) + `.so`
  из **cargo-ndk**; **R8 keep-rules** для `com.sun.jna.**` обязательны;
  **16 KB page alignment** (NDK r28+) обязателен (API 35+, с 01.11.2025); AAB с
  ABI splits.

---

## 11. Сборка и интеграция

### 11.1 Toolchain / workspace
- `rust-toolchain.toml`: pin `channel = "1.95.0"` (выровнять CI/локаль/Gradle/Xcode).
- `Cargo.toml` workspace; **release profile**:
  `lto = "fat"`, `panic = "abort"`, `strip = true`, `opt-level = "z"`,
  `codegen-units = 1`.
- Rust targets:
  - Android: `aarch64-linux-android`, `x86_64-linux-android`.
  - iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim` (+ `x86_64-apple-ios` если в
    CI есть Intel-маки).

### 11.2 Команды (justfile)
```
just gen-bindings     # uniffi-bindgen → Swift + Kotlin
just build-android    # cargo-ndk -t arm64-v8a -t x86_64 -o apps/android/app/src/main/jniLibs build --release
just build-ios        # cargo-swift package → XCFramework + Swift Package
just test-core        # cargo test + clippy
```
- **Android Gradle:** задача `Exec`, хукнутая в `preBuild`, зовёт cargo-ndk и
  uniffi-bindgen (официального Gradle-плагина UniFFI нет; community-плагины —
  оценить на актуальность).
- **iOS:** XCFramework через **cargo-swift** (батарейки в комплекте); при росте —
  cargo-xcframework/скрипты.

### 11.3 CI
- Матрица: core (Linux) → clippy/test/fuzz-smoke; Android (Linux/macOS + NDK);
  iOS-leg **только macOS-runner** (XCFramework), доминирует по времени.
- Кэш `~/.cargo`, `target/`; pin Rust через toolchain-файл.
- Гейты: a11y-аудит (iOS `performAccessibilityAudit`, Android ATF) — обязательны.

---

## 12. Технологии и версии (mid-2026)

> Всегда проверять актуальное перед фиксацией (`rustc --version`, crates.io,
> developer.android.com, Apple). **8 июня — WWDC 2026** → беты iOS 27/Xcode 27
> на подходе; до GA (~сентябрь) сидим на stable.

| Область | Выбор | Версия (verify) |
|---|---|---|
| Rust | toolchain, edition 2024 | 1.95.x |
| FFI | UniFFI (proc-macro, library mode) | 0.31.x |
| Android pkg | cargo-ndk + NDK (16 KB) | 4.1.2 / r28+ |
| Android FFI | JNA direct mapping | JNA ≥ 5.12.0 `@aar` |
| Android | Kotlin/AGP/Gradle/Compose BOM | 2.4.0 / 9.2.0 / 9.5.1 / 2026.06.00 |
| Android SDK | compile/target/min | 36 / 36 / 26 |
| Android ABIs | ship | arm64-v8a, x86_64 |
| iOS pkg | cargo-swift → XCFramework | 0.11.x |
| iOS | Xcode/Swift/min iOS | 26.5 / 6.3 / 18 |
| l10n (core) | Fluent | fluent-bundle (последняя) |
| Build | orchestration | just |

---

## 13. Обработка ошибок, надёжность, offline

- **Нет тихих сбоев.** Каждый Action завершается произнесённым успехом/ошибкой;
  ошибки — действенные («не подключено — вставьте USB-кабель»).
- **Reconnect:** транспорт сообщает разрыв → engine `Disconnected` → авто-повтор;
  состояние линка озвучивается (earcon + речь).
- **Толерантность к мусору:** частичный/битый SysEx, неизвестные адреса,
  power-cycle модуля посреди сессии — не валят app.
- **No blind writes:** UI-значение всегда = readback, не оптимистичное.
- **Offline-first:** профили и каталоги встроены; TTS on-device; сети не требуется.
- **«Changed vs Saved»:** до решения вопроса персистентности (SPEC §14, risk #1)
  явно различаем «изменено (live)» и «сохранено в слот».

---

## 14. Тестирование

- **Core (приоритет — nonvisual + корректность):**
  - Unit: golden-векторы (PROTOCOL §3), все кодировки value⇄string (точные
    произносимые строки на каждую локаль — snapshot-тесты Fluent), FSM
    write→readback→verify против симулятора модуля.
  - Property (proptest): адресная арифметика, checksum.
  - Fuzz (cargo-fuzz): парсер SysEx не паникует.
  - Профили: валидация схемы; контрактные тесты «профиль ↔ адресная карта».
- **Bindings:** smoke-тесты Swift/Kotlin (создать Session, прогнать сценарий).
- **iOS:** Swift Testing (юниты слоя), XCUITest `performAccessibilityAudit` на
  каждом экране (CI-гейт), ручной VoiceOver «вслепую».
- **Android:** Compose semantics-assertions (label/value/role/RangeInfo), ATF в
  Compose-тестах (гейт), Espresso a11y, ручной TalkBack, Accessibility Scanner.
- **Cross-platform contract:** общий набор фикстур, против которого тестируется
  core; нативные слои — на правильность «проводки».
- **Hardware-in-the-loop:** харнесс к реальному модулю (round-trip,
  персистентность, live-edit).
- **Тесты следуют Nonvisual-first:** невизуальные проверки — первичны и пишутся
  первыми; визуальные — вторичны (SPEC §16).

---

## 15. Трейд-оффы (явно)

| Решение | Плюсы | Минусы / риски |
|---|---|---|
| **Rust core + UniFFI** vs логика на каждой платформе | одна тестируемая реализация опасного кода; общий | сложность сборки; FFI-оверхед; UniFFI pre-1.0 (churn) |
| **Sans-I/O core** vs async-core с потоками | детерминизм, простые тесты | больше «проводки» в нативе (tick/таймеры) |
| **Профили (данные)** vs код под модель | расширяемость, будущие модули без кода | надо спроектировать схему; парсинг Data List |
| **i18n в core (Fluent)** vs только нативные ресурсы | единые тестируемые фразы озвучки | переводчикам нужен Fluent; чуть менее «стандартно» |
| **Нативный UI** vs кросс-платформенный | лучшая поддержка VoiceOver/TalkBack | два UI-кодбейза (см. [ADR-0005](adr/0005-native-ui-per-platform.md)) |
| **JNA** (UniFFI default) vs JNI | работает из коробки | не класть per-event вызовы в hot-path; R8 keep-rules |
| **USB-C primary**, BLE secondary | надёжность/латентность/права | BLE-нестабильность вторична |
| **min iOS 18 / minSdk 26** | современные API без gating | отсечение старых устройств (для нишевой аудитории ок) |

---

## 16. Масштаб и устойчивость (для клиентского приложения)

«Масштаб» здесь — не серверная нагрузка, а:
- **Число поддерживаемых устройств/параметров** → реестр профилей; ленивая
  загрузка крупных каталогов (инструменты); профили как данные.
- **Рост каталога** (Roland Cloud expansions) → версионируемые, догружаемые
  profile packs.
- **200 китов × десятки параметров** → инкрементальная подгрузка, кэш модели.
- **Надёжность линка** → robust reconnect, обработка частичных дампов,
  no-blind-writes.

---

## 17. Порядок реализации (увязан с [ROADMAP.md](../ROADMAP.md))

1. **PoC (Web MIDI, TS):** round-trip RQ1/DT1, чтение Current/имя/темп,
   **вопрос персистентности**. Снять risk #1.
2. **`sysex` + `device`:** codec + кодировки + адресная арифметика + схема
   профиля + golden/property/fuzz-тесты. Профиль V31 как данные.
3. **`tools`:** парсер Data List → каталоги/параметры (JSON) для профиля V31.
4. **`model` + `engine`:** доменная модель, Fluent-локализация, session-FSM,
   write→readback→verify (против симулятора).
5. **`ffi` + биндинги:** UniFFI API, smoke на Swift/Kotlin.
6. **Android end-to-end (MVP):** транспорт + TTS + доступный UI вокруг core;
   полный TalkBack-проход «вслепую».
7. **iOS end-to-end:** тот же core; полный VoiceOver-проход.
8. **V1-редакторы** + Performance mode + (опц.) голосовые команды.
9. **Второй профиль** (V51/V71, когда будет доступ к HW/докам) — проверка, что
   расширение идёт через данные, не код.

---

## 18. Открытые вопросы / verify

- **Персистентность** (SPEC §14, risk #1) — на HW.
- **Точная stable-версия Rust** и патч **UniFFI 0.31.x** (+ JNI-статус Android).
- **Адреса/кодировки V51/V71** — нужны доки/железо; пока гипотеза «механика та же,
  отличаются данные».
- **cargo-ndk Gradle-интеграция:** community-плагин vs голый `Exec`.
- **Темп V31 offset `00 6F`** (не `00 6D`) — подтвердить на HW.
- **WWDC 2026** — после GA iOS 27/Xcode 27 пересмотреть min-таргеты и API.
- **Имя продукта** — device-agnostic, до публикации.
